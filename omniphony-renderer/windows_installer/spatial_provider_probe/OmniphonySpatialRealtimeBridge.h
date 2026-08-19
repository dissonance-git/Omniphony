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

#include "omniphony_realtime.h"

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
