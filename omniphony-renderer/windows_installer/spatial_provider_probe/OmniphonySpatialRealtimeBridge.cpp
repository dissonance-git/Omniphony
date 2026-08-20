#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>

#include <cstring>
#include <memory>
#include <new>
#include <utility>
#include <vector>

#include "OmniphonySpatialRealtimeBridge.h"
#include "OmniphonySpatialRoles.h"
#include "OmniphonySpatialStereoQueue.h"

namespace {

bool IsAbsoluteWindowsPath(const wchar_t* path) noexcept {
    if (!path || !path[0]) {
        return false;
    }
    if (path[0] == L'\\' && path[1] == L'\\') {
        return true;
    }
    const wchar_t drive = path[0];
    const bool asciiLetter =
        (drive >= L'A' && drive <= L'Z') || (drive >= L'a' && drive <= L'z');
    return asciiLetter &&
           path[1] == L':' &&
           (path[2] == L'\\' || path[2] == L'/');
}

template <typename T>
bool Resolve(HMODULE module, const char* name, T& target) noexcept {
    const FARPROC raw = GetProcAddress(module, name);
    if (!raw) {
        target = nullptr;
        return false;
    }
    static_assert(sizeof(raw) == sizeof(target));
    std::memcpy(&target, &raw, sizeof(target));
    return target != nullptr;
}

HRESULT LastErrorOrFail() noexcept {
    const DWORD error = GetLastError();
    return error == ERROR_SUCCESS ? E_FAIL : HRESULT_FROM_WIN32(error);
}

class RealtimeQuantumTransport final : public OmniphonySpatialStaticQuantumTransport {
public:
    HRESULT Open(
        const wchar_t* realtimeDllPath,
        const SpatialAudioObjectRenderStreamActivationParams& params,
        std::shared_ptr<OmniphonySpatialStereoQueue> stereoQueue = {}) {
        if (!params.ObjectFormat) {
            return AUDCLNT_E_UNSUPPORTED_FORMAT;
        }
        if (stereoQueue && !stereoQueue->IsOpen()) {
            return E_INVALIDARG;
        }

        std::vector<OmniphonySpatialStaticObjectDescriptor> descriptors;
        try {
            descriptors.reserve(OmniphonyStaticRoleCount(params.StaticObjectTypeMask));
            for (const auto& role : kOmniphonySpatialStaticRoles) {
                if ((OmniphonySpatialObjectBits(params.StaticObjectTypeMask) &
                     OmniphonySpatialObjectBits(role.audio_object_type)) == 0) {
                    continue;
                }
                descriptors.push_back({
                    role.omniphony_role,
                    role.x_right_m,
                    role.y_up_m,
                    role.z_back_m,
                });
            }
        }
        catch (const std::bad_alloc&) {
            return E_OUTOFMEMORY;
        }

        if (descriptors.empty()) {
            return AUDCLNT_E_UNSUPPORTED_FORMAT;
        }

        const HRESULT result = bridge_.Open(
            realtimeDllPath,
            params.ObjectFormat->nSamplesPerSec,
            480,
            descriptors.data(),
            static_cast<std::uint32_t>(descriptors.size()));
        if (FAILED(result)) {
            return result;
        }

        stereoQueue_ = std::move(stereoQueue);
        return S_OK;
    }

    HRESULT Process(
        const float* inputPlanar,
        float* outputStereo,
        std::size_t frames) noexcept override {
        const HRESULT result = bridge_.Process(inputPlanar, outputStereo, frames);
        if (FAILED(result)) {
            return result;
        }
        if (stereoQueue_ && !stereoQueue_->TryWrite(outputStereo, frames)) {
            return HRESULT_FROM_WIN32(ERROR_BUFFER_OVERFLOW);
        }
        return S_OK;
    }

private:
    OmniphonySpatialRealtimeBridge bridge_;
    std::shared_ptr<OmniphonySpatialStereoQueue> stereoQueue_;
};

HRESULT CreateTransportStream(
    const SpatialAudioObjectRenderStreamActivationParams& params,
    const wchar_t* realtimeDllPath,
    std::shared_ptr<OmniphonySpatialStereoQueue> stereoQueue,
    ISpatialAudioObjectRenderStream** stream) {
    if (!stream) {
        return E_POINTER;
    }
    *stream = nullptr;
    if (!IsAbsoluteWindowsPath(realtimeDllPath)) {
        return E_INVALIDARG;
    }
    if (stereoQueue && !stereoQueue->IsOpen()) {
        return E_INVALIDARG;
    }

    std::shared_ptr<RealtimeQuantumTransport> transport;
    try {
        transport = std::make_shared<RealtimeQuantumTransport>();
    }
    catch (const std::bad_alloc&) {
        return E_OUTOFMEMORY;
    }

    const HRESULT openResult = transport->Open(
        realtimeDllPath,
        params,
        std::move(stereoQueue));
    if (FAILED(openResult)) {
        return openResult;
    }

    return CreateOmniphonyStaticProbeStreamWithTransport(
        params,
        std::move(transport),
        stream);
}

} // namespace

OmniphonySpatialRealtimeBridge::~OmniphonySpatialRealtimeBridge() {
    Close();
}

HRESULT OmniphonySpatialRealtimeBridge::Open(
    const wchar_t* realtimeDllPath,
    std::uint32_t sampleRateHz,
    std::uint32_t framesPerQuantum,
    const OmniphonySpatialStaticObjectDescriptor* descriptors,
    std::uint32_t objectCount) noexcept {
    Close();

    if (!IsAbsoluteWindowsPath(realtimeDllPath) ||
        sampleRateHz == 0 ||
        framesPerQuantum == 0 ||
        !descriptors ||
        objectCount == 0 ||
        objectCount > kOmniphonySpatialStaticRoles.size()) {
        return E_INVALIDARG;
    }

    module_ = LoadLibraryExW(
        realtimeDllPath,
        nullptr,
        LOAD_LIBRARY_SEARCH_DLL_LOAD_DIR | LOAD_LIBRARY_SEARCH_SYSTEM32);
    if (!module_) {
        return LastErrorOrFail();
    }

    AbiVersionFn abiMajor = nullptr;
    AbiVersionFn abiMinor = nullptr;
    CreateFn create = nullptr;
    if (!Resolve(module_, "omniphony_realtime_abi_major", abiMajor) ||
        !Resolve(module_, "omniphony_realtime_abi_minor", abiMinor) ||
        !Resolve(module_, "omniphony_spatial_static_create", create) ||
        !Resolve(module_, "omniphony_spatial_static_destroy", destroy_) ||
        !Resolve(module_, "omniphony_spatial_static_latency_frames", latency_) ||
        !Resolve(module_, "omniphony_spatial_static_processed_blocks", processedBlocks_) ||
        !Resolve(module_, "omniphony_spatial_static_process_f32", process_)) {
        Close();
        return HRESULT_FROM_WIN32(ERROR_PROC_NOT_FOUND);
    }

    if (abiMajor() != OMNIPHONY_REALTIME_ABI_MAJOR ||
        abiMinor() < OMNIPHONY_REALTIME_ABI_MINOR) {
        Close();
        return HRESULT_FROM_WIN32(ERROR_REVISION_MISMATCH);
    }

    OmniphonySpatialStaticConfig config{};
    config.sample_rate_hz = sampleRateHz;
    config.frames_per_quantum = framesPerQuantum;
    config.object_count = objectCount;
    config.objects = descriptors;

    processor_ = create(&config);
    if (!processor_) {
        Close();
        return E_FAIL;
    }
    return S_OK;
}

void OmniphonySpatialRealtimeBridge::Close() noexcept {
    if (processor_ && destroy_) {
        destroy_(processor_);
    }
    processor_ = nullptr;

    destroy_ = nullptr;
    latency_ = nullptr;
    processedBlocks_ = nullptr;
    process_ = nullptr;

    if (module_) {
        FreeLibrary(module_);
        module_ = nullptr;
    }
}

std::size_t OmniphonySpatialRealtimeBridge::LatencyFrames() const noexcept {
    return processor_ && latency_ ? latency_(processor_) : 0;
}

std::uint64_t OmniphonySpatialRealtimeBridge::ProcessedBlocks() const noexcept {
    return processor_ && processedBlocks_ ? processedBlocks_(processor_) : 0;
}

HRESULT OmniphonySpatialRealtimeBridge::Process(
    const float* inputPlanar,
    float* outputStereo,
    std::size_t frames) noexcept {
    if (!processor_ || !process_) {
        return E_UNEXPECTED;
    }
    if (!inputPlanar || !outputStereo || frames == 0) {
        return E_INVALIDARG;
    }

    const std::int32_t result = process_(processor_, inputPlanar, outputStereo, frames);
    return result == 0 ? S_OK : HRESULT_FROM_WIN32(ERROR_INVALID_DATA);
}

HRESULT CreateOmniphonyStaticProbeStreamWithRealtimeBridge(
    const SpatialAudioObjectRenderStreamActivationParams& params,
    const wchar_t* realtimeDllPath,
    ISpatialAudioObjectRenderStream** stream) {
    return CreateTransportStream(params, realtimeDllPath, {}, stream);
}

HRESULT CreateOmniphonyStaticProbeStreamWithRealtimeBridgeAndQueue(
    const SpatialAudioObjectRenderStreamActivationParams& params,
    const wchar_t* realtimeDllPath,
    std::shared_ptr<OmniphonySpatialStereoQueue> stereoQueue,
    ISpatialAudioObjectRenderStream** stream) {
    if (!stereoQueue) {
        return E_POINTER;
    }
    return CreateTransportStream(
        params,
        realtimeDllPath,
        std::move(stereoQueue),
        stream);
}
