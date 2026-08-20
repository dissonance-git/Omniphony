#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <audioclient.h>

#include <cstddef>
#include <cstdint>
#include <utility>

#include "OmniphonySpatialRawOutputPump.h"

namespace {

HRESULT WaitResultToHresult(DWORD waitResult) noexcept {
    if (waitResult == WAIT_TIMEOUT) {
        return HRESULT_FROM_WIN32(ERROR_TIMEOUT);
    }
    if (waitResult == WAIT_FAILED) {
        const DWORD error = GetLastError();
        return HRESULT_FROM_WIN32(error == ERROR_SUCCESS ? ERROR_GEN_FAILURE : error);
    }
    return waitResult == WAIT_OBJECT_0 ? S_OK : E_UNEXPECTED;
}

} // namespace

OmniphonySpatialRawOutputPump::~OmniphonySpatialRawOutputPump() {
    Close();
}

HRESULT OmniphonySpatialRawOutputPump::Open(
    const wchar_t* physicalEndpointId,
    std::shared_ptr<OmniphonySpatialStereoQueue> stereoQueue,
    std::uint32_t requestedPeriodFrames) noexcept {
    Close();

    if (!stereoQueue || !stereoQueue->IsOpen() ||
        stereoQueue->CapacityFrames() < 480) {
        return E_INVALIDARG;
    }

    const HRESULT result = sink_.Open(physicalEndpointId, requestedPeriodFrames);
    if (FAILED(result)) {
        return result;
    }

    stereoQueue_ = std::move(stereoQueue);
    drainCycles_.store(0, std::memory_order_relaxed);
    realFramesWritten_.store(0, std::memory_order_relaxed);
    silenceFramesWritten_.store(0, std::memory_order_relaxed);
    return S_OK;
}

HRESULT OmniphonySpatialRawOutputPump::PrimeSilence() noexcept {
    if (!sink_.audioClient_ || !sink_.renderClient_ || sink_.bufferFrames_ == 0) {
        return E_UNEXPECTED;
    }

    UINT32 padding = 0;
    HRESULT result = sink_.audioClient_->GetCurrentPadding(&padding);
    if (FAILED(result)) {
        return result;
    }
    if (padding > sink_.bufferFrames_) {
        return E_UNEXPECTED;
    }

    const UINT32 writable = sink_.bufferFrames_ - padding;
    if (writable == 0) {
        return S_OK;
    }

    BYTE* data = nullptr;
    result = sink_.renderClient_->GetBuffer(writable, &data);
    if (FAILED(result)) {
        return result;
    }

    result = sink_.renderClient_->ReleaseBuffer(
        writable,
        AUDCLNT_BUFFERFLAGS_SILENT);
    if (SUCCEEDED(result)) {
        silenceFramesWritten_.fetch_add(
            static_cast<std::uint64_t>(writable),
            std::memory_order_relaxed);
    }
    return result;
}

HRESULT OmniphonySpatialRawOutputPump::Start() noexcept {
    if (!sink_.IsInitialized() || !stereoQueue_ || !stereoQueue_->IsOpen()) {
        return E_UNEXPECTED;
    }
    if (sink_.started_) {
        return S_OK;
    }

    // Follow the event-driven WASAPI pattern: pre-roll before Start() so the
    // engine never exposes stale/uninitialized data on the first period.
    HRESULT result = PrimeSilence();
    if (FAILED(result)) {
        return result;
    }

    result = sink_.audioClient_->Start();
    if (FAILED(result)) {
        return result;
    }
    sink_.started_ = true;
    return S_OK;
}

HRESULT OmniphonySpatialRawOutputPump::DrainOnce() noexcept {
    if (!sink_.started_ || !sink_.audioClient_ || !sink_.renderClient_ ||
        !stereoQueue_ || !stereoQueue_->IsOpen()) {
        return E_UNEXPECTED;
    }

    UINT32 padding = 0;
    HRESULT result = sink_.audioClient_->GetCurrentPadding(&padding);
    if (FAILED(result)) {
        return result;
    }
    if (padding > sink_.bufferFrames_) {
        return E_UNEXPECTED;
    }

    const UINT32 writable = sink_.bufferFrames_ - padding;
    if (writable == 0) {
        drainCycles_.fetch_add(1, std::memory_order_relaxed);
        return S_OK;
    }

    BYTE* data = nullptr;
    result = sink_.renderClient_->GetBuffer(writable, &data);
    if (FAILED(result)) {
        return result;
    }
    if (!data) {
        // GetBuffer succeeded but did not provide storage. Release zero frames is
        // not a valid completion for a nonzero request, so fail closed and let
        // the owner tear down/re-preflight the endpoint.
        return E_UNEXPECTED;
    }

    const std::size_t realFrames = stereoQueue_->Read(
        reinterpret_cast<float*>(data),
        static_cast<std::size_t>(writable));
    const std::size_t silenceFrames =
        static_cast<std::size_t>(writable) - realFrames;

    const DWORD flags = realFrames == 0 ? AUDCLNT_BUFFERFLAGS_SILENT : 0;
    result = sink_.renderClient_->ReleaseBuffer(writable, flags);
    if (FAILED(result)) {
        return result;
    }

    drainCycles_.fetch_add(1, std::memory_order_relaxed);
    realFramesWritten_.fetch_add(
        static_cast<std::uint64_t>(realFrames),
        std::memory_order_relaxed);
    silenceFramesWritten_.fetch_add(
        static_cast<std::uint64_t>(silenceFrames),
        std::memory_order_relaxed);
    return S_OK;
}

HRESULT OmniphonySpatialRawOutputPump::WaitAndDrain(
    DWORD timeoutMilliseconds) noexcept {
    if (!sink_.started_ || !sink_.sampleReadyEvent_) {
        return E_UNEXPECTED;
    }

    const DWORD waitResult = WaitForSingleObject(
        sink_.sampleReadyEvent_,
        timeoutMilliseconds);
    const HRESULT waitStatus = WaitResultToHresult(waitResult);
    if (FAILED(waitStatus)) {
        return waitStatus;
    }
    return DrainOnce();
}

HRESULT OmniphonySpatialRawOutputPump::Stop() noexcept {
    if (!sink_.IsInitialized()) {
        return S_OK;
    }
    if (!sink_.started_) {
        return S_OK;
    }

    const HRESULT stopResult = sink_.audioClient_->Stop();
    if (FAILED(stopResult)) {
        return stopResult;
    }
    sink_.started_ = false;

    // Reset only after Stop so a later diagnostic restart begins from a known
    // empty endpoint buffer and repeats the explicit pre-roll sequence.
    return sink_.audioClient_->Reset();
}

void OmniphonySpatialRawOutputPump::Close() noexcept {
    if (sink_.IsInitialized() && sink_.started_) {
        const HRESULT stopResult = Stop();
        if (FAILED(stopResult)) {
            // sink_.Close() performs one final best-effort Stop before releasing
            // the client. There is intentionally no automatic device reopen here.
        }
    }
    sink_.Close();
    stereoQueue_.reset();
    drainCycles_.store(0, std::memory_order_relaxed);
    realFramesWritten_.store(0, std::memory_order_relaxed);
    silenceFramesWritten_.store(0, std::memory_order_relaxed);
}
