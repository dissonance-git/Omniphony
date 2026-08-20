#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <audioclient.h>
#include <mmdeviceapi.h>
#include <mmreg.h>
#include <wrl/client.h>

#include "OmniphonySpatialRawOutputSink.h"

namespace {

WAVEFORMATEX Stereo48kFloat() noexcept {
    WAVEFORMATEX format{};
    format.wFormatTag = WAVE_FORMAT_IEEE_FLOAT;
    format.nChannels = 2;
    format.nSamplesPerSec = 48'000;
    format.wBitsPerSample = 32;
    format.nBlockAlign = static_cast<WORD>(format.nChannels * sizeof(float));
    format.nAvgBytesPerSec = format.nSamplesPerSec * format.nBlockAlign;
    format.cbSize = 0;
    return format;
}

bool IsLegalPeriod(
    UINT32 candidate,
    UINT32 fundamental,
    UINT32 minimum,
    UINT32 maximum) noexcept {
    return fundamental != 0 &&
           candidate >= minimum &&
           candidate <= maximum &&
           candidate % fundamental == 0;
}

} // namespace

OmniphonySpatialRawOutputSink::~OmniphonySpatialRawOutputSink() {
    Close();
}

HRESULT OmniphonySpatialRawOutputSink::Open(
    const wchar_t* physicalEndpointId,
    std::uint32_t requestedPeriodFrames) noexcept {
    Close();

    if (!physicalEndpointId || !physicalEndpointId[0]) {
        return E_INVALIDARG;
    }

    Microsoft::WRL::ComPtr<IMMDeviceEnumerator> enumerator;
    HRESULT hr = CoCreateInstance(
        __uuidof(MMDeviceEnumerator),
        nullptr,
        CLSCTX_INPROC_SERVER,
        IID_PPV_ARGS(&enumerator));
    if (FAILED(hr)) {
        return hr;
    }

    Microsoft::WRL::ComPtr<IMMDevice> endpoint;
    hr = enumerator->GetDevice(physicalEndpointId, &endpoint);
    if (FAILED(hr)) {
        return hr;
    }

    DWORD endpointState = 0;
    hr = endpoint->GetState(&endpointState);
    if (FAILED(hr)) {
        return hr;
    }
    if ((endpointState & DEVICE_STATE_ACTIVE) == 0) {
        return AUDCLNT_E_DEVICE_INVALIDATED;
    }

    Microsoft::WRL::ComPtr<IAudioClient3> client;
    hr = endpoint->Activate(
        __uuidof(IAudioClient3),
        CLSCTX_INPROC_SERVER,
        nullptr,
        reinterpret_cast<void**>(client.GetAddressOf()));
    if (FAILED(hr)) {
        return hr;
    }

    AudioClientProperties properties{};
    properties.cbSize = sizeof(properties);
    properties.bIsOffload = FALSE;
    properties.eCategory = AudioCategory_GameEffects;
    properties.Options = AUDCLNT_STREAMOPTIONS_RAW;
    hr = client->SetClientProperties(&properties);
    if (FAILED(hr)) {
        return hr;
    }

    auto desired = Stereo48kFloat();
    WAVEFORMATEX* closest = nullptr;
    const HRESULT support = client->IsFormatSupported(
        AUDCLNT_SHAREMODE_SHARED,
        &desired,
        &closest);
    if (closest) {
        CoTaskMemFree(closest);
    }
    if (support != S_OK) {
        return support == S_FALSE ? AUDCLNT_E_UNSUPPORTED_FORMAT : support;
    }

    UINT32 defaultPeriod = 0;
    UINT32 fundamentalPeriod = 0;
    UINT32 minimumPeriod = 0;
    UINT32 maximumPeriod = 0;
    hr = client->GetSharedModeEnginePeriod(
        &desired,
        &defaultPeriod,
        &fundamentalPeriod,
        &minimumPeriod,
        &maximumPeriod);
    if (FAILED(hr)) {
        return hr;
    }

    const UINT32 selectedPeriod = requestedPeriodFrames == 0
        ? defaultPeriod
        : requestedPeriodFrames;
    if (!IsLegalPeriod(
            selectedPeriod,
            fundamentalPeriod,
            minimumPeriod,
            maximumPeriod)) {
        return requestedPeriodFrames == 0 ? E_UNEXPECTED : E_INVALIDARG;
    }

    HANDLE sampleReadyEvent = CreateEventW(nullptr, FALSE, FALSE, nullptr);
    if (!sampleReadyEvent) {
        return HRESULT_FROM_WIN32(GetLastError());
    }

    hr = client->InitializeSharedAudioStream(
        AUDCLNT_STREAMFLAGS_EVENTCALLBACK,
        selectedPeriod,
        &desired,
        nullptr);
    if (FAILED(hr)) {
        CloseHandle(sampleReadyEvent);
        return hr;
    }

    UINT32 bufferFrames = 0;
    hr = client->GetBufferSize(&bufferFrames);
    if (FAILED(hr)) {
        CloseHandle(sampleReadyEvent);
        return hr;
    }

    Microsoft::WRL::ComPtr<IAudioRenderClient> renderClient;
    hr = client->GetService(
        __uuidof(IAudioRenderClient),
        reinterpret_cast<void**>(renderClient.GetAddressOf()));
    if (FAILED(hr)) {
        CloseHandle(sampleReadyEvent);
        return hr;
    }

    hr = client->SetEventHandle(sampleReadyEvent);
    if (FAILED(hr)) {
        CloseHandle(sampleReadyEvent);
        return hr;
    }

    audioClient_ = client;
    renderClient_ = renderClient;
    sampleReadyEvent_ = sampleReadyEvent;
    bufferFrames_ = bufferFrames;
    periodFrames_ = selectedPeriod;
    sampleRateHz_ = desired.nSamplesPerSec;
    started_ = false;
    return S_OK;
}

void OmniphonySpatialRawOutputSink::Close() noexcept {
    // The public sink has no Start() operation, but the separate closed-gate
    // pump may have started its client. Destruction/repair must still fail
    // closed if the caller forgets the explicit pump Stop().
    if (audioClient_ && started_) {
        (void)audioClient_->Stop();
    }
    started_ = false;

    renderClient_.Reset();
    audioClient_.Reset();
    if (sampleReadyEvent_) {
        CloseHandle(sampleReadyEvent_);
        sampleReadyEvent_ = nullptr;
    }
    bufferFrames_ = 0;
    periodFrames_ = 0;
    sampleRateHz_ = 0;
}
