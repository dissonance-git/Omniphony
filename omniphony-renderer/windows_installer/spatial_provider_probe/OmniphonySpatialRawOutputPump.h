#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>

#include <atomic>
#include <cstdint>
#include <memory>

#include "OmniphonySpatialRawOutputSink.h"
#include "OmniphonySpatialStereoQueue.h"

// Closed-gate active egress primitive.
//
// This class is intentionally separate from the inert RAW sink used by install
// preflight. It is not called by the public Spatial Audio provider yet. When a
// later diagnostic supplies a dedicated event worker, WaitAndDrain() lets the
// physical endpoint event become the sole downstream playback clock.
class OmniphonySpatialRawOutputPump final {
public:
    OmniphonySpatialRawOutputPump() = default;
    ~OmniphonySpatialRawOutputPump();

    OmniphonySpatialRawOutputPump(const OmniphonySpatialRawOutputPump&) = delete;
    OmniphonySpatialRawOutputPump& operator=(const OmniphonySpatialRawOutputPump&) = delete;

    HRESULT Open(
        const wchar_t* physicalEndpointId,
        std::shared_ptr<OmniphonySpatialStereoQueue> stereoQueue,
        std::uint32_t requestedPeriodFrames = 0) noexcept;

    // Pre-rolls the endpoint with silence using its currently writable buffer,
    // then starts the IAudioClient. This never opens the public provider gate.
    HRESULT Start() noexcept;

    // Call only after SampleReadyEvent() is signaled. Queries current padding,
    // drains exactly the writable frame count from the queue, zero-fills any
    // underrun tail, and releases the buffer to Windows.
    HRESULT DrainOnce() noexcept;

    // Convenience for a dedicated endpoint-event worker. No independent timer
    // is introduced: Windows' endpoint event remains the cadence owner.
    HRESULT WaitAndDrain(DWORD timeoutMilliseconds) noexcept;

    HRESULT Stop() noexcept;
    void Close() noexcept;

    bool IsOpen() const noexcept { return sink_.IsInitialized(); }
    bool IsStarted() const noexcept { return sink_.IsStarted(); }
    HANDLE SampleReadyEvent() const noexcept { return sink_.SampleReadyEvent(); }
    std::uint32_t BufferFrames() const noexcept { return sink_.BufferFrames(); }
    std::uint32_t PeriodFrames() const noexcept { return sink_.PeriodFrames(); }

    std::uint64_t DrainCycles() const noexcept {
        return drainCycles_.load(std::memory_order_relaxed);
    }
    std::uint64_t RealFramesWritten() const noexcept {
        return realFramesWritten_.load(std::memory_order_relaxed);
    }
    std::uint64_t SilenceFramesWritten() const noexcept {
        return silenceFramesWritten_.load(std::memory_order_relaxed);
    }

private:
    HRESULT PrimeSilence() noexcept;

    OmniphonySpatialRawOutputSink sink_;
    std::shared_ptr<OmniphonySpatialStereoQueue> stereoQueue_;
    std::atomic<std::uint64_t> drainCycles_{0};
    std::atomic<std::uint64_t> realFramesWritten_{0};
    std::atomic<std::uint64_t> silenceFramesWritten_{0};
};
