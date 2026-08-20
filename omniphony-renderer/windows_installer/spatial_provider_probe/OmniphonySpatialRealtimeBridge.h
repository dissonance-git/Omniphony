#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>

#include <cstddef>
#include <cstdint>
#include <memory>

#include "omniphony_realtime.h"
#include "OmniphonySpatialStaticStream.h"

class OmniphonySpatialStereoQueue;

// Thin dynamic-loader boundary between the experimental Windows Spatial Sound
// provider and the existing omniphony_realtime.dll static-object ABI.
//
// The bridge owns the realtime processor and the exact module handle that
// supplied its function table. It never relies on PATH or the process working
// directory, which is important once this code is installed as an in-process
// COM provider.
class OmniphonySpatialRealtimeBridge final {
public:
    OmniphonySpatialRealtimeBridge() = default;
    ~OmniphonySpatialRealtimeBridge();

    OmniphonySpatialRealtimeBridge(const OmniphonySpatialRealtimeBridge&) = delete;
    OmniphonySpatialRealtimeBridge& operator=(const OmniphonySpatialRealtimeBridge&) = delete;

    HRESULT Open(
        const wchar_t* realtimeDllPath,
        std::uint32_t sampleRateHz,
        std::uint32_t framesPerQuantum,
        const OmniphonySpatialStaticObjectDescriptor* descriptors,
        std::uint32_t objectCount) noexcept;

    void Close() noexcept;

    bool IsOpen() const noexcept { return processor_ != nullptr; }
    std::size_t LatencyFrames() const noexcept;
    std::uint64_t ProcessedBlocks() const noexcept;

    HRESULT Process(
        const float* inputPlanar,
        float* outputStereo,
        std::size_t frames) noexcept;

private:
    using AbiVersionFn = std::uint32_t (*)();
    using CreateFn = OmniphonySpatialStaticProcessor* (*)(const OmniphonySpatialStaticConfig*);
    using DestroyFn = void (*)(OmniphonySpatialStaticProcessor*);
    using LatencyFn = std::size_t (*)(const OmniphonySpatialStaticProcessor*);
    using ProcessedBlocksFn = std::uint64_t (*)(const OmniphonySpatialStaticProcessor*);
    using ProcessFn = std::int32_t (*)(
        OmniphonySpatialStaticProcessor*,
        const float*,
        float*,
        std::size_t);

    HMODULE module_ = nullptr;
    OmniphonySpatialStaticProcessor* processor_ = nullptr;
    DestroyFn destroy_ = nullptr;
    LatencyFn latency_ = nullptr;
    ProcessedBlocksFn processedBlocks_ = nullptr;
    ProcessFn process_ = nullptr;
};

// Registry-free source factory for the internal provider path. It derives the
// immutable descriptor order from StaticObjectTypeMask, opens
// omniphony_realtime.dll before stream processing begins, and connects each
// completed COM quantum to the existing static-object Current worker.
//
// This does not open the public provider gate and does not own final endpoint
// playback. It exists so COM lifecycle -> Current can be proven independently
// before Windows is allowed to route application audio into Omniphony.
HRESULT CreateOmniphonyStaticProbeStreamWithRealtimeBridge(
    const SpatialAudioObjectRenderStreamActivationParams& params,
    const wchar_t* realtimeDllPath,
    ISpatialAudioObjectRenderStream** stream);

// Same closed-gate transport with a pre-opened stereo clock-domain queue on
// the output side. After Current produces one complete stereo quantum, the
// transport submits it to the queue as one non-blocking block. The queue must
// already be opened on the control path. Overflow is surfaced as a transport
// failure rather than blocking or overwriting unread endpoint audio.
HRESULT CreateOmniphonyStaticProbeStreamWithRealtimeBridgeAndQueue(
    const SpatialAudioObjectRenderStreamActivationParams& params,
    const wchar_t* realtimeDllPath,
    std::shared_ptr<OmniphonySpatialStereoQueue> stereoQueue,
    ISpatialAudioObjectRenderStream** stream);
