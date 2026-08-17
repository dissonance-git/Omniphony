#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <unknwn.h>

#include <new>

namespace {

constexpr GUID kProbeClsid = {
    0xf3cdf827, 0x20c4, 0x405e, {0xa4, 0x30, 0x8f, 0x73, 0x93, 0x43, 0xfc, 0x89}};

volatile LONG g_liveReferences = 0;

class ProbeObject final : public IUnknown {
public:
    ProbeObject() {
        InterlockedIncrement(&g_liveReferences);
    }

    ~ProbeObject() {
        InterlockedDecrement(&g_liveReferences);
    }

    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID riid, void** object) override {
        if (!object) {
            return E_POINTER;
        }
        *object = nullptr;
        if (IsEqualIID(riid, IID_IUnknown)) {
            *object = static_cast<IUnknown*>(this);
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

private:
    volatile LONG references_ = 1;
};

class ProbeClassFactory final : public IClassFactory {
public:
    ProbeClassFactory() {
        InterlockedIncrement(&g_liveReferences);
    }

    ~ProbeClassFactory() {
        InterlockedDecrement(&g_liveReferences);
    }

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
        if (outer) {
            return CLASS_E_NOAGGREGATION;
        }

        auto* probe = new (std::nothrow) ProbeObject();
        if (!probe) {
            return E_OUTOFMEMORY;
        }
        const HRESULT result = probe->QueryInterface(riid, object);
        probe->Release();
        return result;
    }

    HRESULT STDMETHODCALLTYPE LockServer(BOOL lock) override {
        if (lock) {
            InterlockedIncrement(&g_liveReferences);
        } else {
            InterlockedDecrement(&g_liveReferences);
        }
        return S_OK;
    }

private:
    volatile LONG references_ = 1;
};

} // namespace

extern "C" __declspec(dllexport) HRESULT __stdcall DllGetClassObject(
    REFCLSID clsid, REFIID riid, void** object) {
    if (!object) {
        return E_POINTER;
    }
    *object = nullptr;
    if (!IsEqualCLSID(clsid, kProbeClsid)) {
        return CLASS_E_CLASSNOTAVAILABLE;
    }

    auto* factory = new (std::nothrow) ProbeClassFactory();
    if (!factory) {
        return E_OUTOFMEMORY;
    }
    const HRESULT result = factory->QueryInterface(riid, object);
    factory->Release();
    return result;
}

extern "C" __declspec(dllexport) HRESULT __stdcall DllCanUnloadNow() {
    return g_liveReferences == 0 ? S_OK : S_FALSE;
}

BOOL WINAPI DllMain(HINSTANCE, DWORD, LPVOID) {
    return TRUE;
}
