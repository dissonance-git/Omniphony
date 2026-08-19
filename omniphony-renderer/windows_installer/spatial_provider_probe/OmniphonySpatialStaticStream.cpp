#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <mmreg.h>
#include <spatialaudioclient.h>

#include <algorithm>
#include <atomic>
#include <cmath>
#include <cstdint>
#include <cstring>
#include <memory>
#include <mutex>
#include <new>
#include <vector>

#include "OmniphonySpatialStaticStream.h"

namespace {

constexpr std::uint32_t Bits(AudioObjectType type) noexcept {
    return static_cast<std::uint32_t>(type);
}

constexpr AudioObjectType kSupportedStaticMask = static_cast<AudioObjectType>(
    Bits(AudioObjectType_FrontLeft) |
    Bits(AudioObjectType_FrontRight) |
    Bits(AudioObjectType_FrontCenter) |
    Bits(AudioObjectType_LowFrequency) |
    Bits(AudioObjectType_SideLeft) |
    Bits(AudioObjectType_SideRight) |
    Bits(AudioObjectType_BackLeft) |
    Bits(AudioObjectType_BackRight) |
    Bits(AudioObjectType_BackCenter) |
    Bits(AudioObjectType_TopFrontLeft) |
    Bits(AudioObjectType_TopFrontRight) |
    Bits(AudioObjectType_TopBackLeft) |
    Bits(AudioObjectType_TopBackRight) |
    Bits(AudioObjectType_BottomFrontLeft) |
    Bits(AudioObjectType_BottomFrontRight) |
    Bits(AudioObjectType_BottomBackLeft) |
    Bits(AudioObjectType_BottomBackRight));

bool IsSingleObjectType(AudioObjectType type) noexcept {
    const auto bits = Bits(type);
    return bits != 0 && (bits & (bits - 1)) == 0;
}

struct StaticStreamState;
class StaticProbeObject;

struct StaticStreamState {
    std::mutex mutex;
    bool destroyed = false;
    bool running = false;
    bool inUpdate = false;
    std::uint64_t generation = 0;
    AudioObjectType staticMask = AudioObjectType_None;
    UINT32 frameCount = 0;
    std::vector<StaticProbeObject*> objects;
};

class StaticProbeObject final : public ISpatialAudioObject {
public:
    StaticProbeObject(
        std::shared_ptr<StaticStreamState> state,
        AudioObjectType type,
        UINT32 frameCount)
        : state_(std::move(state)),
          type_(type),
          staging_(frameCount, 0.0f) {
        std::lock_guard<std::mutex> lock(state_->mutex);
        state_->objects.push_back(this);
    }

    ~StaticProbeObject() {
        std::lock_guard<std::mutex> lock(state_->mutex);
        auto& objects = state_->objects;
        objects.erase(std::remove(objects.begin(), objects.end(), this), objects.end());
    }

    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID riid, void** object) override {
        if (!object) {
            return E_POINTER;
        }
        *object = nullptr;
        if (IsEqualIID(riid, IID_IUnknown) ||
            IsEqualIID(riid, __uuidof(ISpatialAudioObjectBase)) ||
            IsEqualIID(riid, __uuidof(ISpatialAudioObject))) {
            *object = static_cast<ISpatialAudioObject*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }

    ULONG STDMETHODCALLTYPE AddRef() override {
        return references_.fetch_add(1, std::memory_order_relaxed) + 1;
    }

    ULONG STDMETHODCALLTYPE Release() override {
        const ULONG value = references_.fetch_sub(1, std::memory_order_acq_rel) - 1;
        if (value == 0) {
            delete this;
        }
        return value;
    }

    HRESULT STDMETHODCALLTYPE GetBuffer(BYTE** buffer, UINT32* bufferLength) override {
        if (!buffer || !bufferLength) {
            return E_POINTER;
        }
        *buffer = nullptr;
        *bufferLength = 0;

        std::lock_guard<std::mutex> stateLock(state_->mutex);
        if (state_->destroyed) {
            return SPTLAUDCLNT_E_DESTROYED;
        }
        if (!active_) {
            return SPTLAUDCLNT_E_RESOURCES_INVALIDATED;
        }
        if (!state_->inUpdate) {
            return SPTLAUDCLNT_E_OUT_OF_ORDER;
        }

        std::lock_guard<std::mutex> objectLock(mutex_);
        std::fill(staging_.begin(), staging_.end(), 0.0f);
        lastBufferGeneration_ = state_->generation;
        *buffer = reinterpret_cast<BYTE*>(staging_.data());
        *bufferLength = static_cast<UINT32>(staging_.size() * sizeof(float));
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE SetEndOfStream(UINT32 frameCount) override {
        std::lock_guard<std::mutex> stateLock(state_->mutex);
        if (state_->destroyed) {
            return SPTLAUDCLNT_E_DESTROYED;
        }
        if (!active_) {
            return SPTLAUDCLNT_E_RESOURCES_INVALIDATED;
        }
        if (!state_->inUpdate) {
            return SPTLAUDCLNT_E_OUT_OF_ORDER;
        }
        if (frameCount > state_->frameCount) {
            return E_INVALIDARG;
        }

        std::lock_guard<std::mutex> objectLock(mutex_);
        endOfStreamPending_ = true;
        endOfStreamFrameCount_ = frameCount;
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE IsActive(BOOL* isActive) override {
        if (!isActive) {
            return E_POINTER;
        }
        std::lock_guard<std::mutex> stateLock(state_->mutex);
        if (state_->destroyed) {
            *isActive = FALSE;
            return SPTLAUDCLNT_E_DESTROYED;
        }
        std::lock_guard<std::mutex> objectLock(mutex_);
        *isActive = active_ ? TRUE : FALSE;
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE GetAudioObjectType(AudioObjectType* type) override {
        if (!type) {
            return E_POINTER;
        }
        *type = type_;
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE SetPosition(float, float, float) override {
        std::lock_guard<std::mutex> stateLock(state_->mutex);
        if (state_->destroyed) {
            return SPTLAUDCLNT_E_DESTROYED;
        }
        {
            std::lock_guard<std::mutex> objectLock(mutex_);
            if (!active_) {
                return SPTLAUDCLNT_E_RESOURCES_INVALIDATED;
            }
        }
        if (!state_->inUpdate) {
            return SPTLAUDCLNT_E_OUT_OF_ORDER;
        }
        // This P3 stream activates static roles only. Their positions are fixed
        // by AudioObjectType / ISpatialAudioClient::GetStaticObjectPosition.
        return SPTLAUDCLNT_E_PROPERTY_NOT_SUPPORTED;
    }

    HRESULT STDMETHODCALLTYPE SetVolume(float volume) override {
        if (!std::isfinite(volume) || volume < 0.0f || volume > 1.0f) {
            return E_INVALIDARG;
        }
        std::lock_guard<std::mutex> stateLock(state_->mutex);
        if (state_->destroyed) {
            return SPTLAUDCLNT_E_DESTROYED;
        }
        if (!state_->inUpdate) {
            return SPTLAUDCLNT_E_OUT_OF_ORDER;
        }
        std::lock_guard<std::mutex> objectLock(mutex_);
        if (!active_) {
            return SPTLAUDCLNT_E_RESOURCES_INVALIDATED;
        }
        volume_ = volume;
        return S_OK;
    }

    void CommitPass(std::uint64_t generation) noexcept {
        std::lock_guard<std::mutex> lock(mutex_);
        if (!active_) {
            return;
        }
        // Windows implicitly ends an object if GetBuffer is skipped for a pass.
        if (lastBufferGeneration_ != generation || endOfStreamPending_) {
            active_ = false;
        }
    }

    void Revoke() noexcept {
        std::lock_guard<std::mutex> lock(mutex_);
        active_ = false;
    }

    bool IsActiveInternal() const noexcept {
        std::lock_guard<std::mutex> lock(mutex_);
        return active_;
    }

    AudioObjectType Type() const noexcept { return type_; }

private:
    std::atomic<ULONG> references_{1};
    std::shared_ptr<StaticStreamState> state_;
    AudioObjectType type_ = AudioObjectType_None;
    mutable std::mutex mutex_;
    std::vector<float> staging_;
    bool active_ = true;
    std::uint64_t lastBufferGeneration_ = 0;
    bool endOfStreamPending_ = false;
    UINT32 endOfStreamFrameCount_ = 0;
    float volume_ = 1.0f;
};

class StaticProbeStream final : public ISpatialAudioObjectRenderStream {
public:
    StaticProbeStream(AudioObjectType staticMask, UINT32 frameCount)
        : state_(std::make_shared<StaticStreamState>()) {
        state_->staticMask = staticMask;
        state_->frameCount = frameCount;
    }

    ~StaticProbeStream() {
        std::lock_guard<std::mutex> lock(state_->mutex);
        state_->destroyed = true;
        state_->running = false;
        state_->inUpdate = false;
        for (auto* object : state_->objects) {
            if (object) {
                object->Revoke();
            }
        }
    }

    HRESULT STDMETHODCALLTYPE QueryInterface(REFIID riid, void** object) override {
        if (!object) {
            return E_POINTER;
        }
        *object = nullptr;
        if (IsEqualIID(riid, IID_IUnknown) ||
            IsEqualIID(riid, __uuidof(ISpatialAudioObjectRenderStreamBase)) ||
            IsEqualIID(riid, __uuidof(ISpatialAudioObjectRenderStream))) {
            *object = static_cast<ISpatialAudioObjectRenderStream*>(this);
            AddRef();
            return S_OK;
        }
        return E_NOINTERFACE;
    }

    ULONG STDMETHODCALLTYPE AddRef() override {
        return references_.fetch_add(1, std::memory_order_relaxed) + 1;
    }

    ULONG STDMETHODCALLTYPE Release() override {
        const ULONG value = references_.fetch_sub(1, std::memory_order_acq_rel) - 1;
        if (value == 0) {
            delete this;
        }
        return value;
    }

    HRESULT STDMETHODCALLTYPE GetAvailableDynamicObjectCount(UINT32* count) override {
        if (!count) {
            return E_POINTER;
        }
        std::lock_guard<std::mutex> lock(state_->mutex);
        if (state_->destroyed) {
            return SPTLAUDCLNT_E_DESTROYED;
        }
        *count = 0;
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE GetService(REFIID, void** service) override {
        if (!service) {
            return E_POINTER;
        }
        *service = nullptr;
        return E_NOINTERFACE;
    }

    HRESULT STDMETHODCALLTYPE Start() override {
        std::lock_guard<std::mutex> lock(state_->mutex);
        if (state_->destroyed) {
            return SPTLAUDCLNT_E_DESTROYED;
        }
        if (state_->running) {
            return SPTLAUDCLNT_E_STREAM_NOT_STOPPED;
        }
        state_->running = true;
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE Stop() override {
        std::lock_guard<std::mutex> lock(state_->mutex);
        if (state_->destroyed) {
            return SPTLAUDCLNT_E_DESTROYED;
        }
        state_->running = false;
        state_->inUpdate = false;
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE Reset() override {
        std::lock_guard<std::mutex> lock(state_->mutex);
        if (state_->destroyed) {
            return SPTLAUDCLNT_E_DESTROYED;
        }
        if (state_->running) {
            return SPTLAUDCLNT_E_STREAM_NOT_STOPPED;
        }
        state_->generation = 0;
        state_->inUpdate = false;
        for (auto* object : state_->objects) {
            if (object) {
                object->Revoke();
            }
        }
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE BeginUpdatingAudioObjects(
        UINT32* availableDynamicObjectCount,
        UINT32* frameCountPerBuffer) override {
        if (!availableDynamicObjectCount || !frameCountPerBuffer) {
            return E_POINTER;
        }
        std::lock_guard<std::mutex> lock(state_->mutex);
        if (state_->destroyed) {
            return SPTLAUDCLNT_E_DESTROYED;
        }
        if (!state_->running) {
            return AUDCLNT_E_SERVICE_NOT_RUNNING;
        }
        if (state_->inUpdate) {
            return SPTLAUDCLNT_E_OUT_OF_ORDER;
        }
        state_->inUpdate = true;
        ++state_->generation;
        *availableDynamicObjectCount = 0;
        *frameCountPerBuffer = state_->frameCount;
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE EndUpdatingAudioObjects() override {
        std::lock_guard<std::mutex> lock(state_->mutex);
        if (state_->destroyed) {
            return SPTLAUDCLNT_E_DESTROYED;
        }
        if (!state_->inUpdate) {
            return SPTLAUDCLNT_E_OUT_OF_ORDER;
        }
        const auto generation = state_->generation;
        for (auto* object : state_->objects) {
            if (object) {
                object->CommitPass(generation);
            }
        }
        state_->inUpdate = false;
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE ActivateSpatialAudioObject(
        AudioObjectType type,
        ISpatialAudioObject** object) override {
        if (!object) {
            return E_POINTER;
        }
        *object = nullptr;

        std::unique_lock<std::mutex> lock(state_->mutex);
        if (state_->destroyed) {
            return SPTLAUDCLNT_E_DESTROYED;
        }
        if (type == AudioObjectType_Dynamic) {
            return SPTLAUDCLNT_E_NO_MORE_OBJECTS;
        }
        if (!IsSingleObjectType(type)) {
            return E_INVALIDARG;
        }
        if ((Bits(state_->staticMask) & Bits(type)) != Bits(type)) {
            return SPTLAUDCLNT_E_STATIC_OBJECT_NOT_AVAILABLE;
        }
        for (auto* existing : state_->objects) {
            if (existing && existing->Type() == type && existing->IsActiveInternal()) {
                return SPTLAUDCLNT_E_OBJECT_ALREADY_ACTIVE;
            }
        }

        // Constructor registration also takes the state mutex. The topology and
        // frame count are immutable, so capture what is needed then unlock.
        const UINT32 frameCount = state_->frameCount;
        lock.unlock();
        auto* created = new (std::nothrow) StaticProbeObject(state_, type, frameCount);
        if (!created) {
            return E_OUTOFMEMORY;
        }
        *object = static_cast<ISpatialAudioObject*>(created);
        return S_OK;
    }

private:
    std::atomic<ULONG> references_{1};
    std::shared_ptr<StaticStreamState> state_;
};

bool ValidActivationParams(const SpatialAudioObjectRenderStreamActivationParams& params) noexcept {
    const auto* format = params.ObjectFormat;
    if (!format ||
        format->wFormatTag != WAVE_FORMAT_IEEE_FLOAT ||
        format->nChannels != 1 ||
        format->nSamplesPerSec != 48'000 ||
        format->wBitsPerSample != 32 ||
        format->nBlockAlign != sizeof(float) ||
        format->nAvgBytesPerSec != 48'000 * sizeof(float)) {
        return false;
    }
    if (params.MinDynamicObjectCount != 0 || params.MaxDynamicObjectCount != 0) {
        return false;
    }
    const auto requestedMask = Bits(params.StaticObjectTypeMask);
    return requestedMask != 0 &&
           (requestedMask & ~Bits(kSupportedStaticMask)) == 0;
}

} // namespace

HRESULT CreateOmniphonyStaticProbeStream(
    const SpatialAudioObjectRenderStreamActivationParams& params,
    ISpatialAudioObjectRenderStream** stream) {
    if (!stream) {
        return E_POINTER;
    }
    *stream = nullptr;
    if (!ValidActivationParams(params)) {
        return AUDCLNT_E_UNSUPPORTED_FORMAT;
    }

    // The public capability layer currently advertises a 480-frame quantum.
    // The real provider will replace this fixed cadence with the RAW endpoint
    // transport's event-driven cadence before the stream is wired live.
    auto* created = new (std::nothrow) StaticProbeStream(
        params.StaticObjectTypeMask,
        480);
    if (!created) {
        return E_OUTOFMEMORY;
    }
    *stream = static_cast<ISpatialAudioObjectRenderStream*>(created);
    return S_OK;
}

HRESULT CreateOmniphonyStaticProbeStreamFromActivation(
    const PROPVARIANT* activationParams,
    REFIID riid,
    void** stream) {
    if (!stream) {
        return E_POINTER;
    }
    *stream = nullptr;

    if (!IsEqualIID(riid, __uuidof(ISpatialAudioObjectRenderStream))) {
        return E_NOINTERFACE;
    }
    if (!activationParams) {
        return E_POINTER;
    }
    if (activationParams->vt != VT_BLOB ||
        activationParams->blob.cbSize != sizeof(SpatialAudioObjectRenderStreamActivationParams) ||
        !activationParams->blob.pBlobData) {
        return E_INVALIDARG;
    }

    SpatialAudioObjectRenderStreamActivationParams params{};
    std::memcpy(
        &params,
        activationParams->blob.pBlobData,
        sizeof(params));

    ISpatialAudioObjectRenderStream* typedStream = nullptr;
    const HRESULT hr = CreateOmniphonyStaticProbeStream(params, &typedStream);
    if (FAILED(hr)) {
        return hr;
    }
    *stream = static_cast<void*>(typedStream);
    return S_OK;
}
