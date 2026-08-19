#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <audioclient.h>
#include <mmdeviceapi.h>
#include <mmreg.h>
#include <wrl/client.h>

#include <cstdint>
#include <iomanip>
#include <iostream>

using Microsoft::WRL::ComPtr;

namespace {

class CoInit final {
public:
    CoInit() noexcept : hr_(CoInitializeEx(nullptr, COINIT_MULTITHREADED)) {}
    ~CoInit() {
        if (SUCCEEDED(hr_)) {
            CoUninitialize();
        }
    }
    HRESULT Result() const noexcept { return hr_; }

private:
    HRESULT hr_ = E_FAIL;
};

int Fail(const wchar_t* stage, HRESULT hr) {
    std::wcerr << L"SPATIAL_RAW_OUTPUT_PROBE_FAIL stage=" << stage
               << L" hr=0x" << std::hex << std::uppercase
               << static_cast<unsigned long>(hr) << std::dec << L"\n";
    return 1;
}

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

void PrintFormat(const wchar_t* prefix, const WAVEFORMATEX* format) {
    if (!format) {
        std::wcout << prefix << L"_PRESENT 0\n";
        return;
    }
    std::wcout << prefix << L"_PRESENT 1\n";
    std::wcout << prefix << L"_TAG " << format->wFormatTag << L"\n";
    std::wcout << prefix << L"_CHANNELS " << format->nChannels << L"\n";
    std::wcout << prefix << L"_SAMPLE_RATE " << format->nSamplesPerSec << L"\n";
    std::wcout << prefix << L"_BITS " << format->wBitsPerSample << L"\n";
    std::wcout << prefix << L"_BLOCK_ALIGN " << format->nBlockAlign << L"\n";
}

} // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 2 || !argv[1] || !argv[1][0]) {
        std::wcerr << L"usage: OmniphonySpatialRawOutputProbe.exe <physical-endpoint-id>\n";
        return 2;
    }

    CoInit co;
    if (FAILED(co.Result()) && co.Result() != RPC_E_CHANGED_MODE) {
        return Fail(L"CoInitializeEx", co.Result());
    }

    ComPtr<IMMDeviceEnumerator> enumerator;
    HRESULT hr = CoCreateInstance(
        __uuidof(MMDeviceEnumerator),
        nullptr,
        CLSCTX_INPROC_SERVER,
        IID_PPV_ARGS(&enumerator));
    if (FAILED(hr)) {
        return Fail(L"MMDeviceEnumerator", hr);
    }

    ComPtr<IMMDevice> endpoint;
    hr = enumerator->GetDevice(argv[1], &endpoint);
    if (FAILED(hr)) {
        return Fail(L"GetDevice", hr);
    }

    LPWSTR canonicalEndpointId = nullptr;
    hr = endpoint->GetId(&canonicalEndpointId);
    if (FAILED(hr) || !canonicalEndpointId) {
        return Fail(L"GetId", FAILED(hr) ? hr : E_FAIL);
    }

    DWORD state = 0;
    hr = endpoint->GetState(&state);
    if (FAILED(hr)) {
        CoTaskMemFree(canonicalEndpointId);
        return Fail(L"GetState", hr);
    }

    ComPtr<IAudioClient3> client;
    hr = endpoint->Activate(
        __uuidof(IAudioClient3),
        CLSCTX_INPROC_SERVER,
        nullptr,
        reinterpret_cast<void**>(client.GetAddressOf()));
    if (FAILED(hr)) {
        CoTaskMemFree(canonicalEndpointId);
        return Fail(L"Activate(IAudioClient3)", hr);
    }

    AudioClientProperties properties{};
    properties.cbSize = sizeof(properties);
    properties.bIsOffload = FALSE;
    properties.eCategory = AudioCategory_GameEffects;
    properties.Options = AUDCLNT_STREAMOPTIONS_RAW;
    hr = client->SetClientProperties(&properties);
    if (FAILED(hr)) {
        CoTaskMemFree(canonicalEndpointId);
        return Fail(L"SetClientProperties(RAW)", hr);
    }

    WAVEFORMATEX* mixFormat = nullptr;
    hr = client->GetMixFormat(&mixFormat);
    if (FAILED(hr) || !mixFormat) {
        CoTaskMemFree(canonicalEndpointId);
        return Fail(L"GetMixFormat", FAILED(hr) ? hr : E_FAIL);
    }

    auto desired = Stereo48kFloat();
    WAVEFORMATEX* closest = nullptr;
    const HRESULT support = client->IsFormatSupported(
        AUDCLNT_SHAREMODE_SHARED,
        &desired,
        &closest);

    UINT32 defaultPeriod = 0;
    UINT32 fundamentalPeriod = 0;
    UINT32 minPeriod = 0;
    UINT32 maxPeriod = 0;
    HRESULT periodResult = E_FAIL;
    if (support == S_OK) {
        periodResult = client->GetSharedModeEnginePeriod(
            &desired,
            &defaultPeriod,
            &fundamentalPeriod,
            &minPeriod,
            &maxPeriod);
    }

    WAVEFORMATEX* currentFormat = nullptr;
    UINT32 currentPeriod = 0;
    const HRESULT currentResult = client->GetCurrentSharedModeEnginePeriod(
        &currentFormat,
        &currentPeriod);

    std::wcout << L"SPATIAL_RAW_OUTPUT_PROBE_OK 1\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_ENDPOINT_ID " << canonicalEndpointId << L"\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_ENDPOINT_STATE 0x"
               << std::hex << std::uppercase << state << std::dec << L"\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_CLIENT_PROPERTIES_OK 1\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_MODE RAW\n";
    PrintFormat(L"SPATIAL_RAW_OUTPUT_MIX_FORMAT", mixFormat);
    PrintFormat(L"SPATIAL_RAW_OUTPUT_DESIRED_FORMAT", &desired);

    std::wcout << L"SPATIAL_RAW_OUTPUT_DESIRED_SUPPORTED "
               << (support == S_OK ? 1 : 0) << L"\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_DESIRED_SUPPORT_HR 0x"
               << std::hex << std::uppercase
               << static_cast<unsigned long>(support) << std::dec << L"\n";
    if (closest) {
        PrintFormat(L"SPATIAL_RAW_OUTPUT_CLOSEST_FORMAT", closest);
    } else {
        std::wcout << L"SPATIAL_RAW_OUTPUT_CLOSEST_FORMAT_PRESENT 0\n";
    }

    if (SUCCEEDED(periodResult)) {
        std::wcout << L"SPATIAL_RAW_OUTPUT_PERIOD_QUERY_OK 1\n";
        std::wcout << L"SPATIAL_RAW_OUTPUT_PERIOD_DEFAULT_FRAMES " << defaultPeriod << L"\n";
        std::wcout << L"SPATIAL_RAW_OUTPUT_PERIOD_FUNDAMENTAL_FRAMES " << fundamentalPeriod << L"\n";
        std::wcout << L"SPATIAL_RAW_OUTPUT_PERIOD_MIN_FRAMES " << minPeriod << L"\n";
        std::wcout << L"SPATIAL_RAW_OUTPUT_PERIOD_MAX_FRAMES " << maxPeriod << L"\n";
        std::wcout << L"SPATIAL_RAW_OUTPUT_480_PERIOD_LEGAL "
                   << (IsLegalPeriod(480, fundamentalPeriod, minPeriod, maxPeriod) ? 1 : 0)
                   << L"\n";
    } else {
        std::wcout << L"SPATIAL_RAW_OUTPUT_PERIOD_QUERY_OK 0\n";
        std::wcout << L"SPATIAL_RAW_OUTPUT_PERIOD_QUERY_HR 0x"
                   << std::hex << std::uppercase
                   << static_cast<unsigned long>(periodResult) << std::dec << L"\n";
    }

    std::wcout << L"SPATIAL_RAW_OUTPUT_CURRENT_PERIOD_QUERY_OK "
               << (SUCCEEDED(currentResult) ? 1 : 0) << L"\n";
    if (SUCCEEDED(currentResult)) {
        PrintFormat(L"SPATIAL_RAW_OUTPUT_CURRENT_FORMAT", currentFormat);
        std::wcout << L"SPATIAL_RAW_OUTPUT_CURRENT_PERIOD_FRAMES "
                   << currentPeriod << L"\n";
    } else {
        std::wcout << L"SPATIAL_RAW_OUTPUT_CURRENT_PERIOD_QUERY_HR 0x"
                   << std::hex << std::uppercase
                   << static_cast<unsigned long>(currentResult) << std::dec << L"\n";
    }

    // Capability probe only. This executable intentionally never initializes,
    // starts, or obtains IAudioRenderClient, so running it cannot create a new
    // playback stream or alter the endpoint's current audio graph.
    std::wcout << L"SPATIAL_RAW_OUTPUT_STREAM_INITIALIZED 0\n";
    std::wcout << L"SPATIAL_RAW_OUTPUT_STREAM_STARTED 0\n";

    if (currentFormat) {
        CoTaskMemFree(currentFormat);
    }
    if (closest) {
        CoTaskMemFree(closest);
    }
    CoTaskMemFree(mixFormat);
    CoTaskMemFree(canonicalEndpointId);
    return 0;
}
