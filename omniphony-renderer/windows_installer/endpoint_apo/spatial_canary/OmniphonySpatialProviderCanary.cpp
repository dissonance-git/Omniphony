#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#include <windows.h>
#include <objbase.h>

#include <atomic>
#include <cwchar>
#include <new>

namespace {

// Experimental provider CLSID. This is intentionally distinct from every
// production Omniphony APO identity and from the experimental spatial format
// GUID used by the registration helper.
constexpr GUID kProviderClsid = {
    0x7aee0f13, 0x1f6b, 0x4d83,
    {0x9f, 0x6d, 0x6c, 0x9c, 0x0e, 0x33, 0xa1, 0x51}
};

constexpr wchar_t kPipeName[] = L"\\\\.\\pipe\\OmniphonySpatialProviderCanaryV1";

std::atomic<LONG> g_factoryCount{0};
std::atomic<LONG> g_serverLocks{0};

void EmitRequestedInterface(REFIID riid) {
    wchar_t iidText[64] = {};
    StringFromGUID2(riid, iidText, static_cast<int>(sizeof(iidText) / sizeof(iidText[0])));

    wchar_t processPath[MAX_PATH] = {};
    const DWORD processChars = GetModuleFileNameW(
        nullptr,
        processPath,
        static_cast<DWORD>(sizeof(processPath) / sizeof(processPath[0])));
    if (processChars == 0) {
        wcscpy_s(processPath, L"<unknown>");
    }

    SYSTEMTIME now = {};
    GetSystemTime(&now);

    wchar_t message[1024] = {};
    swprintf_s(
        message,
        L"omniphony_spatial_provider_canary version=1 utc=%04u-%02u-%02uT%02u:%02u:%02u.%03uZ pid=%lu process=\"%ls\" requested_iid=%ls\r\n",
        now.wYear,
        now.wMonth,
        now.wDay,
        now.wHour,
        now.wMinute,
        now.wSecond,
        now.wMilliseconds,
        static_cast<unsigned long>(GetCurrentProcessId()),
        processPath,
        iidText);

    // Debug output is a fallback witness if the one-shot listener is not
    // running. The product renderer must never depend on this diagnostic path.
    OutputDebugStringW(message);

    // The controller creates this pipe only during an explicit Phase 1 test.
    // Failure to connect is deliberately non-fatal because Windows may probe
    // the class outside the observation window.
    HANDLE pipe = CreateFileW(
        kPipeName,
        GENERIC_WRITE,
        0,
        nullptr,
        OPEN_EXISTING,
        FILE_ATTRIBUTE_NORMAL,
        nullptr);
    if (pipe == INVALID_HANDLE_VALUE) {
        return;
    }

    const DWORD bytes = static_cast<DWORD>(wcslen(message) * sizeof(wchar_t));
    DWORD written = 0;
    WriteFile(pipe, message, bytes, &written, nullptr);
    CloseHandle(pipe);
}

class CanaryClassFactory final : public IClassFactory {
public:
    CanaryClassFactory() {
        g_factoryCount.fetch_add(1, std::memory_order_relaxed);
    }

    STDMETHODIMP QueryInterface(REFIID riid, void** object) override {
        if (object == nullptr) {
            return E_POINTER;
        }
        *object = nullptr;
        if (riid == IID_IUnknown || riid == IID_IClassFactory) {
            *object = static_cast<IClassFactory*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }

    STDMETHODIMP_(ULONG) AddRef() override {
        return refCount_.fetch_add(1, std::memory_order_relaxed) + 1;
    }

    STDMETHODIMP_(ULONG) Release() override {
        const ULONG value = refCount_.fetch_sub(1, std::memory_order_acq_rel) - 1;
        if (value == 0) {
            delete this;
        }
        return value;
    }

    STDMETHODIMP CreateInstance(IUnknown* outer, REFIID riid, void** object) override {
        if (object == nullptr) {
            return E_POINTER;
        }
        *object = nullptr;

        // This is the entire point of Phase 1: capture the interface Windows
        // actually asks a selected spatial provider to instantiate. Do not
        // guess ISpatialAudioClient or any undocumented encoder interface here.
        EmitRequestedInterface(riid);

        if (outer != nullptr) {
            return CLASS_E_NOAGGREGATION;
        }
        return E_NOINTERFACE;
    }

    STDMETHODIMP LockServer(BOOL lock) override {
        if (lock != FALSE) {
            g_serverLocks.fetch_add(1, std::memory_order_relaxed);
        } else {
            g_serverLocks.fetch_sub(1, std::memory_order_relaxed);
        }
        return S_OK;
    }

private:
    ~CanaryClassFactory() override {
        g_factoryCount.fetch_sub(1, std::memory_order_relaxed);
    }

    std::atomic<ULONG> refCount_{1};
};

} // namespace

extern "C" HRESULT __stdcall DllGetClassObject(REFCLSID rclsid, REFIID riid, void** object) {
    if (object == nullptr) {
        return E_POINTER;
    }
    *object = nullptr;
    if (!IsEqualGUID(rclsid, kProviderClsid)) {
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    auto* factory = new (std::nothrow) CanaryClassFactory();
    if (factory == nullptr) {
        return E_OUTOFMEMORY;
    }
    const HRESULT result = factory->QueryInterface(riid, object);
    factory->Release();
    return result;
}

extern "C" HRESULT __stdcall DllCanUnloadNow() {
    return (g_factoryCount.load(std::memory_order_relaxed) == 0 &&
            g_serverLocks.load(std::memory_order_relaxed) == 0)
        ? S_OK
        : S_FALSE;
}
