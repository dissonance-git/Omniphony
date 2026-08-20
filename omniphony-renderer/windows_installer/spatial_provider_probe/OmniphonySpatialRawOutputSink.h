#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#include <audioclient.h>
#include <wrl/client.h>

#include <cstdint>

class OmniphonySpatialRawOutputPump;

// Inert provider-egress control lifecycle for one exact physical render
// endpoint.
//
// Open() may initialize a shared event-driven RAW stereo stream and acquire its
// IAudioRenderClient, but this class intentionally exposes no public Start()
// method. The separate closed-gate output pump is the only source component
// allowed to start/drain this client while the public Spatial Audio provider
// remains unavailable.
//
// requestedPeriodFrames == 0 means "use the endpoint's reported default shared
// engine period". Omniphony's 480-frame spatial render quantum is deliberately
// not imposed on the physical endpoint; a preallocated clock-domain queue owns
// that block-size adaptation instead.
class OmniphonySpatialRawOutputSink final {
public:
    OmniphonySpatialRawOutputSink() = default;
    ~OmniphonySpatialRawOutputSink();

    OmniphonySpatialRawOutputSink(const OmniphonySpatialRawOutputSink&) = delete;
    OmniphonySpatialRawOutputSink& operator=(const OmniphonySpatialRawOutputSink&) = delete;

    HRESULT Open(
        const wchar_t* physicalEndpointId,
        std::uint32_t requestedPeriodFrames = 0) noexcept;

    void Close() noexcept;

    bool IsInitialized() const noexcept { return audioClient_ != nullptr; }
    bool IsStarted() const noexcept { return started_; }
    bool HasRenderClient() const noexcept { return renderClient_ != nullptr; }
    HANDLE SampleReadyEvent() const noexcept { return sampleReadyEvent_; }
    std::uint32_t BufferFrames() const noexcept { return bufferFrames_; }
    std::uint32_t PeriodFrames() const noexcept { return periodFrames_; }
    std::uint32_t SampleRateHz() const noexcept { return sampleRateHz_; }

private:
    friend class OmniphonySpatialRawOutputPump;

    Microsoft::WRL::ComPtr<IAudioClient3> audioClient_;
    Microsoft::WRL::ComPtr<IAudioRenderClient> renderClient_;
    HANDLE sampleReadyEvent_ = nullptr;
    std::uint32_t bufferFrames_ = 0;
    std::uint32_t periodFrames_ = 0;
    std::uint32_t sampleRateHz_ = 0;
    bool started_ = false;
};
