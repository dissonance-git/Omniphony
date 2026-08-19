#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <audioclient.h>
#include <mmreg.h>
#include <spatialaudioclient.h>
#include <unknwn.h>

#include <cstdint>
#include <cstring>
#include <new>

#include "OmniphonySpatialRoles.h"

namespace {

constexpr GUID kProbeClsid = {
    0xf3cdf827, 0x20c4, 0x405e, {0xa4, 0x30, 0x8f, 0x73, 0x93, 0x43, 0xfc, 0x89}};

constexpr UINT32 kObjectSampleRate = 48'000;
constexpr UINT32 kObjectFramesPerBuffer = 480;

volatile LONG g_liveReferences = 0;

bool IsProbeObjectFormat(const WAVEFORMATEX* format) noexcept {
    return format != nullptr &&
           format->wFormatTag == WAVE_FORMAT_IEEE_FLOAT &&
           format->nChannels == 1 &&
           format->nSamplesPerSec == kObjectSampleRate &&
           format->wBitsPerSample == 32 &&
           format->nBlockAlign == sizeof(float) &&
           format->nAvgBytesPerSec == kObjectSampleRate * sizeof(float);
}

void FillProbeObjectFormat(WAVEFORMATEX& format) noexcept {
    std::memset(&format, 0, sizeof(format));
    format.wFormatTag = WAVE_FORMAT_IEEE_FLOAT;
    format.nChannels = 1;
    format.nSamplesPerSec = kObjectSampleRate;
    format.wBitsPerSample = 32;
    format.nBlockAlign = sizeof(float);
    format.nAvgBytesPerSec = kObjectSampleRate * sizeof(float);
    format.cbSize = 0;
}

HRESULT StaticPosition(AudioObjectType type, float* x, float* y, float* z) noexcept {
    if (!x || !y || !z) {
        return E_POINTER;
    }
    const auto* role = FindOmniphonySpatialStaticRole(type);
    if (!role) {
        return E_INVALIDARG;
    }
    *x = role->x_right_m;
    *y = role->y_up_m;
    *z = role->z_back_m;
    return S_OK;
}

class ProbeFormatEnumerator final : public IAudioFormatEnumerator {
public:
    ProbeFormatEnumerator() {
        InterlockedIncrement(&g_liveReferences);
    }

    ~ProbeFormatEnumerator() {
        InterlockedDecrement(&g_liveReferences);
    }

    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID riid, void** object) override {
        if (!object) {
            return E_POINTER;
        }
        *object = nullptr;
        if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, __uuidof(IAudioFormatEnumerator))) {
            *object = static_cast<IAudioFormatEnumerator*>(this);
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

    HRESULT STDMETHODCALLTYPE GetCount(UINT32* count) override {
        if (!count) {
            return E_POINTER;
        }
        *count = 1;
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE GetFormat(UINT32 index, WAVEFORMATEX** format) override {
        if (!format) {
            return E_POINTER;
        }
        *format = nullptr;
        if (index != 0) {
            return E_INVALIDARG;
        }

        auto* allocated = static_cast<WAVEFORMATEX*>(CoTaskMemAlloc(sizeof(WAVEFORMATEX)));
        if (!allocated) {
            return E_OUTOFMEMORY;
        }
        FillProbeObjectFormat(*allocated);
        *format = allocated;
        return S_OK;
    }

private:
    volatile LONG references_ = 1;
};

class ProbeObject final : public ISpatialAudioClient {
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
        if (IsEqualIID(riid, IID_IUnknown) || IsEqualIID(riid, __uuidof(ISpatialAudioClient))) {
            *object = static_cast<ISpatialAudioClient*>(this);
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

    HRESULT STDMETHODCALLTYPE GetStaticObjectPosition(
        AudioObjectType type,
        float* x,
        float* y,
        float* z) override {
        return StaticPosition(type, x, y, z);
    }

    HRESULT STDMETHODCALLTYPE GetNativeStaticObjectTypeMask(AudioObjectType* mask) override {
        if (!mask) {
            return E_POINTER;
        }
        *mask = OmniphonyCanonicalStaticMask();
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE GetMaxDynamicObjectCount(UINT32* value) override {
        if (!value) {
            return E_POINTER;
        }
        // Static transport is being built first. Dynamic XYZ capacity remains
        // zero until identity, position updates, lifetime, and realtime transport
        // exist as a separate truthful contract.
        *value = 0;
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE GetSupportedAudioObjectFormatEnumerator(
        IAudioFormatEnumerator** enumerator) override {
        if (!enumerator) {
            return E_POINTER;
        }
        *enumerator = new (std::nothrow) ProbeFormatEnumerator();
        return *enumerator ? S_OK : E_OUTOFMEMORY;
    }

    HRESULT STDMETHODCALLTYPE GetMaxFrameCount(
        const WAVEFORMATEX* objectFormat,
        UINT32* frameCountPerBuffer) override {
        if (!objectFormat || !frameCountPerBuffer) {
            return E_POINTER;
        }
        if (!IsProbeObjectFormat(objectFormat)) {
            return AUDCLNT_E_UNSUPPORTED_FORMAT;
        }
        *frameCountPerBuffer = kObjectFramesPerBuffer;
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE IsAudioObjectFormatSupported(
        const WAVEFORMATEX* objectFormat) override {
        if (!objectFormat) {
            return E_POINTER;
        }
        return IsProbeObjectFormat(objectFormat) ? S_OK : AUDCLNT_E_UNSUPPORTED_FORMAT;
    }

    HRESULT STDMETHODCALLTYPE IsSpatialAudioStreamAvailable(
        REFIID,
        const PROPVARIANT*) override {
        // Internal COM -> Current transport now exists in source, but the final
        // Windows output/cadence boundary is not yet proven. Keep the provider
        // closed so an application can never submit audio to a silent sink.
        return SPTLAUDCLNT_E_STREAM_IS_NOT_AVAILABLE;
    }

    HRESULT STDMETHODCALLTYPE ActivateSpatialAudioStream(
        const PROPVARIANT*,
        REFIID,
        void** stream) override {
        if (!stream) {
            return E_POINTER;
        }
        *stream = nullptr;
        return SPTLAUDCLNT_E_STREAM_IS_NOT_AVAILABLE;
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

STDAPI DllGetClassObject(REFCLSID clsid, REFIID riid, LPVOID* object) {
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

STDAPI DllCanUnloadNow() {
    return g_liveReferences == 0 ? S_OK : S_FALSE;
}

BOOL WINAPI DllMain(HINSTANCE, DWORD, LPVOID) {
    return TRUE;
}
