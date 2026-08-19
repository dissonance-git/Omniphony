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

#include "OmniphonySpatialRoles.h"
#include "OmniphonySpatialStaticStream.h"

namespace {

struct StaticStreamState;
class StaticProbeObject;

struct StaticStreamState {
    std::mutex mutex;
    bool destroyed = false;
    bool running = false;
    bool inUpdate = false;
    bool transportInFlight = false;
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
        if (!state_->inUpdate || state_->transportInFlight) {
            return SPTLAUDCLNT_E_OUT_OF_ORDER;
        }

        std::lock_guard<std::mutex> objectLock(mutex_);
        if (!active_) {
            return SPTLAUDCLNT_E_RESOURCES_INVALIDATED;
        }
        std::fill(staging_.begin(), staging_.end(), 0.0f);
        lastBufferGeneration_ = state_->generation;
        endOfStreamPending_ = false;
        endOfStreamFrameCount_ = 0;
        *buffer = reinterpret_cast<BYTE*>(staging_.data());
        *bufferLength = static_cast<UINT32>(staging_.size() * sizeof(float));
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE SetEndOfStream(UINT32 frameCount) override {
        std::lock_guard<std::mutex> stateLock(state_->mutex);
        if (state_->destroyed) {
            return SPTLAUDCLNT_E_DESTROYED;
        }
        if (!state_->inUpdate || state_->transportInFlight) {
            return SPTLAUDCLNT_E_OUT_OF_ORDER;
        }
        if (frameCount > state_->frameCount) {
            return E_INVALIDARG;
        }

        std::lock_guard<std::mutex> objectLock(mutex_);
        if (!active_) {
            return SPTLAUDCLNT_E_RESOURCES_INVALIDATED;
        }
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
        if (!state_->inUpdate || state_->transportInFlight) {
            return SPTLAUDCLNT_E_OUT_OF_ORDER;
        }
        std::lock_guard<std::mutex> objectLock(mutex_);
        if (!active_) {
            return SPTLAUDCLNT_E_RESOURCES_INVALIDATED;
        }
        // Static object positions are authoritative AudioObjectType geometry.
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
        if (!state_->inUpdate || state_->transportInFlight) {
            return SPTLAUDCLNT_E_OUT_OF_ORDER;
        }
        std::lock_guard<std::mutex> objectLock(mutex_);
        if (!active_) {
            return SPTLAUDCLNT_E_RESOURCES_INVALIDATED;
        }
        volume_ = volume;
        return S_OK;
    }

    // Copy exactly one completed Windows update pass into the immutable planar
    // stream slot. Destination is already zeroed by the stream.
    //
    // Skipping GetBuffer implicitly ends the object. SetEndOfStream(N) keeps only
    // the first N frames from the final buffer and zeroes the remainder because
    // the fixed realtime topology always submits a complete quantum.
    void SnapshotPass(
        std::uint64_t generation,
        float* destination,
        UINT32 frameCount) noexcept {
        std::lock_guard<std::mutex> lock(mutex_);
        if (!active_) {
            return;
        }
        if (lastBufferGeneration_ != generation) {
            active_ = false;
            return;
        }

        UINT32 validFrames = frameCount;
        if (endOfStreamPending_) {
            validFrames = std::min(frameCount, endOfStreamFrameCount_);
        }
        const UINT32 available = static_cast<UINT32>(staging_.size());
        validFrames = std::min(validFrames, available);
        for (UINT32 frame = 0; frame < validFrames; ++frame) {
            destination[frame] = staging_[frame] * volume_;
        }

        if (endOfStreamPending_) {
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
    StaticProbeStream(
        AudioObjectType staticMask,
        UINT32 frameCount,
        std::shared_ptr<OmniphonySpatialStaticQuantumTransport> transport)
        : state_(std::make_shared<StaticStreamState>()),
          transport_(std::move(transport)) {
        state_->staticMask = staticMask;
        state_->frameCount = frameCount;

        const auto roleCount = OmniphonyStaticRoleCount(staticMask);
        roleOrder_.reserve(roleCount);
        for (const auto& role : kOmniphonySpatialStaticRoles) {
            if ((OmniphonySpatialObjectBits(staticMask) &
                 OmniphonySpatialObjectBits(role.audio_object_type)) != 0) {
                roleOrder_.push_back(role.audio_object_type);
            }
        }
        planar_.assign(roleCount * static_cast<std::size_t>(frameCount), 0.0f);
        stereo_.assign(static_cast<std::size_t>(frameCount) * 2, 0.0f);
    }

    ~StaticProbeStream() {
        std::lock_guard<std::mutex> lock(state_->mutex);
        state_->destroyed = true;
        state_->running = false;
        state_->inUpdate = false;
        state_->transportInFlight = false;
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
        if (state_->inUpdate || state_->transportInFlight) {
            return SPTLAUDCLNT_E_OUT_OF_ORDER;
        }
        state_->running = true;
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE Stop() override {
        std::lock_guard<std::mutex> lock(state_->mutex);
        if (state_->destroyed) {
            return SPTLAUDCLNT_E_DESTROYED;
        }
        if (state_->inUpdate || state_->transportInFlight) {
            return SPTLAUDCLNT_E_OUT_OF_ORDER;
        }
        state_->running = false;
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
        if (state_->inUpdate || state_->transportInFlight) {
            return SPTLAUDCLNT_E_OUT_OF_ORDER;
        }
        state_->generation = 0;
        std::fill(planar_.begin(), planar_.end(), 0.0f);
        std::fill(stereo_.begin(), stereo_.end(), 0.0f);
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
        if (state_->inUpdate || state_->transportInFlight) {
            return SPTLAUDCLNT_E_OUT_OF_ORDER;
        }
        state_->inUpdate = true;
        ++state_->generation;
        *availableDynamicObjectCount = 0;
        *frameCountPerBuffer = state_->frameCount;
        return S_OK;
    }

    HRESULT STDMETHODCALLTYPE EndUpdatingAudioObjects() override {
        std::shared_ptr<OmniphonySpatialStaticQuantumTransport> transport;
        UINT32 frameCount = 0;

        {
            std::lock_guard<std::mutex> lock(state_->mutex);
            if (state_->destroyed) {
                return SPTLAUDCLNT_E_DESTROYED;
            }
            if (!state_->inUpdate || state_->transportInFlight) {
                return SPTLAUDCLNT_E_OUT_OF_ORDER;
            }

            const auto generation = state_->generation;
            frameCount = state_->frameCount;
            std::fill(planar_.begin(), planar_.end(), 0.0f);

            for (auto* object : state_->objects) {
                if (!object) {
                    continue;
                }
                const auto slot = OmniphonyStaticRoleSlot(
                    state_->staticMask,
                    object->Type());
                if (slot == static_cast<std::size_t>(-1) || slot >= roleOrder_.size()) {
                    continue;
                }
                object->SnapshotPass(
                    generation,
                    planar_.data() + slot * frameCount,
                    frameCount);
            }

            state_->inUpdate = false;
            transport = transport_;
            if (transport) {
                state_->transportInFlight = true;
            }
        }

        if (!transport) {
            return S_OK;
        }

        std::fill(stereo_.begin(), stereo_.end(), 0.0f);
        const HRESULT transportResult = transport->Process(
            planar_.data(),
            stereo_.data(),
            frameCount);

        {
            std::lock_guard<std::mutex> lock(state_->mutex);
            state_->transportInFlight = false;
        }
        return transportResult;
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
        if (!OmniphonyIsSingleStaticObjectType(type) ||
            !FindOmniphonySpatialStaticRole(type)) {
            return E_INVALIDARG;
        }
        if ((OmniphonySpatialObjectBits(state_->staticMask) &
             OmniphonySpatialObjectBits(type)) != OmniphonySpatialObjectBits(type)) {
            return SPTLAUDCLNT_E_STATIC_OBJECT_NOT_AVAILABLE;
        }
        for (auto* existing : state_->objects) {
            if (existing && existing->Type() == type && existing->IsActiveInternal()) {
                return SPTLAUDCLNT_E_OBJECT_ALREADY_ACTIVE;
            }
        }

        const UINT32 frameCount = state_->frameCount;
        lock.unlock();
        try {
            auto* created = new StaticProbeObject(state_, type, frameCount);
            *object = static_cast<ISpatialAudioObject*>(created);
            return S_OK;
        }
        catch (const std::bad_alloc&) {
            return E_OUTOFMEMORY;
        }
    }

private:
    std::atomic<ULONG> references_{1};
    std::shared_ptr<StaticStreamState> state_;
    std::shared_ptr<OmniphonySpatialStaticQuantumTransport> transport_;
    std::vector<AudioObjectType> roleOrder_;
    std::vector<float> planar_;
    std::vector<float> stereo_;
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
    const auto requestedMask = OmniphonySpatialObjectBits(params.StaticObjectTypeMask);
    const auto supportedMask = OmniphonySpatialObjectBits(OmniphonyCanonicalStaticMask());
    return requestedMask != 0 && (requestedMask & ~supportedMask) == 0;
}

HRESULT CreateValidatedStream(
    const SpatialAudioObjectRenderStreamActivationParams& params,
    std::shared_ptr<OmniphonySpatialStaticQuantumTransport> transport,
    ISpatialAudioObjectRenderStream** stream) {
    if (!stream) {
        return E_POINTER;
    }
    *stream = nullptr;
    if (!ValidActivationParams(params)) {
        return AUDCLNT_E_UNSUPPORTED_FORMAT;
    }

    try {
        auto* created = new StaticProbeStream(
            params.StaticObjectTypeMask,
            480,
            std::move(transport));
        *stream = static_cast<ISpatialAudioObjectRenderStream*>(created);
        return S_OK;
    }
    catch (const std::bad_alloc&) {
        return E_OUTOFMEMORY;
    }
}

} // namespace

HRESULT CreateOmniphonyStaticProbeStream(
    const SpatialAudioObjectRenderStreamActivationParams& params,
    ISpatialAudioObjectRenderStream** stream) {
    return CreateValidatedStream(params, nullptr, stream);
}

HRESULT CreateOmniphonyStaticProbeStreamWithTransport(
    const SpatialAudioObjectRenderStreamActivationParams& params,
    std::shared_ptr<OmniphonySpatialStaticQuantumTransport> transport,
    ISpatialAudioObjectRenderStream** stream) {
    if (!transport) {
        if (stream) {
            *stream = nullptr;
        }
        return E_INVALIDARG;
    }
    return CreateValidatedStream(params, std::move(transport), stream);
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
