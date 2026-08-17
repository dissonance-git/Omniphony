#include <windows.h>
#include <unknwn.h>
#include <audioenginebaseapo.h>
#include <audioengineextensionapo.h>
#include <BaseAudioProcessingObject.h>

#include <cstring>
#include <new>
#include <string>

namespace {

constexpr GUID kOmniphonyApoClsid = {
    0xa9333bfe, 0x39c1, 0x40fd, {0xb4, 0xb0, 0xec, 0xc5, 0x91, 0x41, 0x0b, 0x47}};

HINSTANCE g_module = nullptr;
volatile LONG g_factoryLocks = 0;

class INonDelegatingUnknown {
public:
    virtual HRESULT STDMETHODCALLTYPE NonDelegatingQueryInterface(REFIID riid, void** object) = 0;
    virtual ULONG STDMETHODCALLTYPE NonDelegatingAddRef() = 0;
    virtual ULONG STDMETHODCALLTYPE NonDelegatingRelease() = 0;
};

class OmniphonyAPO final : public CBaseAudioProcessingObject,
                           public IAudioSystemEffects,
                           public INonDelegatingUnknown {
public:
    static volatile LONG instanceCount;
    static const CRegAPOProperties<1> registration;

    explicit OmniphonyAPO(IUnknown* outer)
        : CBaseAudioProcessingObject(registration),
          outer_(outer ? outer : reinterpret_cast<IUnknown*>(static_cast<INonDelegatingUnknown*>(this))) {
        InterlockedIncrement(&instanceCount);
    }

    ~OmniphonyAPO() override {
        InterlockedDecrement(&instanceCount);
    }

    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID riid, void** object) override {
        return outer_->QueryInterface(riid, object);
    }

    ULONG STDMETHODCALLTYPE AddRef() override {
        return outer_->AddRef();
    }

    ULONG STDMETHODCALLTYPE Release() override {
        return outer_->Release();
    }

    HRESULT STDMETHODCALLTYPE NonDelegatingQueryInterface(REFIID riid, void** object) override {
        if (!object) {
            return E_POINTER;
        }
        *object = nullptr;

        if (IsEqualIID(riid, IID_IUnknown)) {
            *object = static_cast<INonDelegatingUnknown*>(this);
        } else if (IsEqualIID(riid, __uuidof(IAudioProcessingObject))) {
            *object = static_cast<IAudioProcessingObject*>(this);
        } else if (IsEqualIID(riid, __uuidof(IAudioProcessingObjectRT))) {
            *object = static_cast<IAudioProcessingObjectRT*>(this);
        } else if (IsEqualIID(riid, __uuidof(IAudioProcessingObjectConfiguration))) {
            *object = static_cast<IAudioProcessingObjectConfiguration*>(this);
        } else if (IsEqualIID(riid, __uuidof(IAudioSystemEffects))) {
            *object = static_cast<IAudioSystemEffects*>(this);
        } else {
            return E_NOINTERFACE;
        }

        reinterpret_cast<IUnknown*>(*object)->AddRef();
        return S_OK;
    }

    ULONG STDMETHODCALLTYPE NonDelegatingAddRef() override {
        return static_cast<ULONG>(InterlockedIncrement(&references_));
    }

    ULONG STDMETHODCALLTYPE NonDelegatingRelease() override {
        const LONG value = InterlockedDecrement(&references_);
        if (value == 0) {
            delete this;
            return 0;
        }
        return static_cast<ULONG>(value);
    }

    HRESULT STDMETHODCALLTYPE Initialize(UINT32 dataSize, BYTE* data) override {
        if ((data == nullptr) != (dataSize == 0)) {
            return E_INVALIDARG;
        }
        if (dataSize != sizeof(APOInitSystemEffects) && dataSize != sizeof(APOInitSystemEffects2)) {
            return E_INVALIDARG;
        }
        if (m_bIsInitialized) {
            return HRESULT_FROM_WIN32(ERROR_ALREADY_EXISTS);
        }
        m_bIsInitialized = true;
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE GetLatency(HNSTIME* latency) override {
        if (!latency) {
            return E_POINTER;
        }
        *latency = 0;
        return S_OK;
    }

    void STDMETHODCALLTYPE APOProcess(
        UINT32 inputCount,
        APO_CONNECTION_PROPERTY** inputs,
        UINT32 outputCount,
        APO_CONNECTION_PROPERTY** outputs) override {
        if (inputCount == 0 || outputCount == 0 || !inputs || !outputs || !inputs[0] || !outputs[0]) {
            return;
        }

        auto* input = inputs[0];
        auto* output = outputs[0];
        const UINT32 frames = input->u32ValidFrameCount;
        const size_t samples = static_cast<size_t>(frames) * GetSamplesPerFrame();
        const size_t bytes = samples * sizeof(float);
        auto* inputBuffer = reinterpret_cast<const void*>(input->pBuffer);
        auto* outputBuffer = reinterpret_cast<void*>(output->pBuffer);

        switch (input->u32BufferFlags) {
        case BUFFER_VALID:
            if (output->pBuffer != input->pBuffer && bytes != 0) {
                std::memmove(outputBuffer, inputBuffer, bytes);
            }
            output->u32BufferFlags = BUFFER_VALID;
            output->u32ValidFrameCount = frames;
            break;
        case BUFFER_SILENT:
            if (output->pBuffer && bytes != 0) {
                std::memset(outputBuffer, 0, bytes);
            }
            output->u32BufferFlags = BUFFER_SILENT;
            output->u32ValidFrameCount = frames;
            break;
        default:
            output->u32BufferFlags = BUFFER_INVALID;
            output->u32ValidFrameCount = 0;
            break;
        }
    }

    STDMETHODIMP GetEffectsList(LPGUID* effects, UINT* effectCount, HANDLE eventHandle) {
        UNREFERENCED_PARAMETER(eventHandle);
        if (!effects || !effectCount) {
            return E_POINTER;
        }
        *effects = nullptr;
        *effectCount = 0;
        return S_OK;
    }

private:
    volatile LONG references_ = 1;
    IUnknown* outer_ = nullptr;
};

volatile LONG OmniphonyAPO::instanceCount = 0;
#pragma warning(disable : 4815)
const CRegAPOProperties<1> OmniphonyAPO::registration(
    kOmniphonyApoClsid,
    L"Omniphony Endpoint APO",
    L"Omniphony downstream fork",
    1,
    0,
    __uuidof(IAudioProcessingObject),
    static_cast<APO_FLAG>(APO_FLAG_FRAMESPERSECOND_MUST_MATCH |
                          APO_FLAG_BITSPERSAMPLE_MUST_MATCH |
                          APO_FLAG_INPLACE));

class ApoClassFactory final : public IClassFactory {
public:
    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID riid, void** object) override {
        if (!object) {
            return E_POINTER;
        }
        *object = nullptr;
        if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, IID_IClassFactory)) {
            *object = static_cast<IClassFactory*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }

    ULONG STDMETHODCALLTYPE AddRef() override {
        return static_cast<ULONG>(InterlockedIncrement(&references_));
    }

    ULONG STDMETHODCALLTYPE Release() override {
        const LONG value = InterlockedDecrement(&references_);
        if (value == 0) {
            delete this;
            return 0;
        }
        return static_cast<ULONG>(value);
    }

    HRESULT STDMETHODCALLTYPE CreateInstance(IUnknown* outer, REFIID riid, void** object) override {
        if (!object) {
            return E_POINTER;
        }
        *object = nullptr;
        if (outer && !IsEqualIID(riid, IID_IUnknown)) {
            return CLASS_E_NOAGGREGATION;
        }
        auto* apo = new (std::nothrow) OmniphonyAPO(outer);
        if (!apo) {
            return E_OUTOFMEMORY;
        }
        const HRESULT hr = apo->NonDelegatingQueryInterface(riid, object);
        apo->NonDelegatingRelease();
        return hr;
    }

    HRESULT STDMETHODCALLTYPE LockServer(BOOL lock) override {
        if (lock) {
            InterlockedIncrement(&g_factoryLocks);
        } else {
            InterlockedDecrement(&g_factoryLocks);
        }
        return S_OK;
    }

private:
    volatile LONG references_ = 1;
};

std::wstring GuidText(REFGUID guid) {
    wchar_t text[64] = {};
    StringFromGUID2(guid, text, 64);
    return text;
}

HRESULT WriteString(HKEY key, const wchar_t* name, const std::wstring& value) {
    const LSTATUS status = RegSetValueExW(
        key, name, 0, REG_SZ, reinterpret_cast<const BYTE*>(value.c_str()),
        static_cast<DWORD>((value.size() + 1) * sizeof(wchar_t)));
    return HRESULT_FROM_WIN32(status);
}

HRESULT RegisterComClass() {
    wchar_t modulePath[MAX_PATH] = {};
    if (!GetModuleFileNameW(g_module, modulePath, MAX_PATH)) {
        return HRESULT_FROM_WIN32(GetLastError());
    }

    const std::wstring clsid = GuidText(kOmniphonyApoClsid);
    const std::wstring path = L"SOFTWARE\\Classes\\CLSID\\" + clsid;
    HKEY classKey = nullptr;
    LSTATUS status = RegCreateKeyExW(HKEY_LOCAL_MACHINE, path.c_str(), 0, nullptr, 0, KEY_WRITE, nullptr, &classKey, nullptr);
    if (status != ERROR_SUCCESS) {
        return HRESULT_FROM_WIN32(status);
    }
    HRESULT hr = WriteString(classKey, nullptr, L"Omniphony Endpoint APO");
    RegCloseKey(classKey);
    if (FAILED(hr)) {
        return hr;
    }

    HKEY serverKey = nullptr;
    status = RegCreateKeyExW(HKEY_LOCAL_MACHINE, (path + L"\\InprocServer32").c_str(), 0, nullptr, 0, KEY_WRITE, nullptr, &serverKey, nullptr);
    if (status != ERROR_SUCCESS) {
        return HRESULT_FROM_WIN32(status);
    }
    hr = WriteString(serverKey, nullptr, modulePath);
    if (SUCCEEDED(hr)) {
        hr = WriteString(serverKey, L"ThreadingModel", L"Both");
    }
    RegCloseKey(serverKey);
    return hr;
}

void UnregisterComClass() {
    const std::wstring path = L"SOFTWARE\\Classes\\CLSID\\" + GuidText(kOmniphonyApoClsid);
    RegDeleteTreeW(HKEY_LOCAL_MACHINE, path.c_str());
}

} // namespace

STDAPI DllGetClassObject(REFCLSID clsid, REFIID riid, LPVOID* object) {
    if (!object) {
        return E_POINTER;
    }
    *object = nullptr;
    if (!IsEqualCLSID(clsid, kOmniphonyApoClsid)) {
        return CLASS_E_CLASSNOTAVAILABLE;
    }
    auto* factory = new (std::nothrow) ApoClassFactory();
    if (!factory) {
        return E_OUTOFMEMORY;
    }
    const HRESULT hr = factory->QueryInterface(riid, object);
    factory->Release();
    return hr;
}

STDAPI DllCanUnloadNow() {
    return OmniphonyAPO::instanceCount == 0 && g_factoryLocks == 0 ? S_OK : S_FALSE;
}

STDAPI DllRegisterServer() {
    HRESULT hr = RegisterAPO(OmniphonyAPO::registration);
    if (FAILED(hr)) {
        return hr;
    }
    hr = RegisterComClass();
    if (FAILED(hr)) {
        UnregisterAPO(kOmniphonyApoClsid);
    }
    return hr;
}

STDAPI DllUnregisterServer() {
    UnregisterComClass();
    return UnregisterAPO(kOmniphonyApoClsid);
}

BOOL WINAPI DllMain(HINSTANCE module, DWORD reason, LPVOID) {
    if (reason == DLL_PROCESS_ATTACH) {
        g_module = module;
        DisableThreadLibraryCalls(module);
    }
    return TRUE;
}
