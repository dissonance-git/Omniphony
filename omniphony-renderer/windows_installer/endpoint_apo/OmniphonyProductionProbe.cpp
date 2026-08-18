#include <windows.h>
#include <audioclient.h>
#include <mmdeviceapi.h>
#include <mmreg.h>
#include <ksmedia.h>

#include <iomanip>
#include <iostream>

namespace {

void PrintHr(const wchar_t* stage, HRESULT hr) {
    std::wcerr << L"ERROR\t" << stage << L"\t0x"
               << std::hex << std::uppercase << static_cast<unsigned long>(hr)
               << std::dec << std::nouppercase << L"\n";
}

bool IsFloat32Stereo(const WAVEFORMATEX* format) {
    if (!format || format->nChannels != 2 || format->wBitsPerSample != 32) {
        return false;
    }
    if (format->wFormatTag == WAVE_FORMAT_IEEE_FLOAT) {
        return true;
    }
    if (format->wFormatTag != WAVE_FORMAT_EXTENSIBLE ||
        format->cbSize < (sizeof(WAVEFORMATEXTENSIBLE) - sizeof(WAVEFORMATEX))) {
        return false;
    }
    const auto* extensible = reinterpret_cast<const WAVEFORMATEXTENSIBLE*>(format);
    return IsEqualGUID(extensible->SubFormat, KSDATAFORMAT_SUBTYPE_IEEE_FLOAT) != FALSE &&
           extensible->Samples.wValidBitsPerSample == 32;
}

}  // namespace

int wmain(int argc, wchar_t** argv) {
    if (argc != 2 || !argv[1] || argv[1][0] == L'\0') {
        std::wcerr << L"Usage: OmniphonyProductionProbe.exe <exact-MMDevice-ID>\n";
        return 2;
    }

    const HRESULT initHr = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(initHr)) {
        PrintHr(L"CoInitializeEx", initHr);
        return 3;
    }

    IMMDeviceEnumerator* enumerator = nullptr;
    IMMDevice* device = nullptr;
    IAudioClient* audioClient = nullptr;
    IAudioRenderClient* renderClient = nullptr;
    WAVEFORMATEX* mix = nullptr;
    int result = 1;

    do {
        HRESULT hr = CoCreateInstance(
            __uuidof(MMDeviceEnumerator), nullptr, CLSCTX_INPROC_SERVER,
            __uuidof(IMMDeviceEnumerator), reinterpret_cast<void**>(&enumerator));
        if (FAILED(hr)) {
            PrintHr(L"MMDeviceEnumerator", hr);
            result = 4;
            break;
        }

        hr = enumerator->GetDevice(argv[1], &device);
        if (FAILED(hr)) {
            PrintHr(L"GetDevice", hr);
            result = 5;
            break;
        }
        std::wcout << L"ENDPOINT_OPEN_OK\t" << argv[1] << L"\n";

        hr = device->Activate(
            __uuidof(IAudioClient), CLSCTX_ALL, nullptr,
            reinterpret_cast<void**>(&audioClient));
        if (FAILED(hr)) {
            PrintHr(L"Activate(IAudioClient)", hr);
            result = 6;
            break;
        }

        hr = audioClient->GetMixFormat(&mix);
        if (FAILED(hr)) {
            PrintHr(L"GetMixFormat", hr);
            result = 7;
            break;
        }
        std::wcout << L"GET_MIX_FORMAT_OK\t"
                   << mix->nSamplesPerSec << L"\t"
                   << mix->nChannels << L"\t"
                   << mix->wBitsPerSample << L"\t"
                   << mix->nBlockAlign << L"\n";

        if (!IsFloat32Stereo(mix)) {
            std::wcerr << L"ERROR\tCurrentMixContract\trequires stereo float32 mix format\n";
            result = 8;
            break;
        }
        std::wcout << L"CURRENT_MIX_CONTRACT_OK\tstereo-float32\n";

        constexpr REFERENCE_TIME kRequestedBuffer = 1'000'000;  // 100 ms.
        hr = audioClient->Initialize(
            AUDCLNT_SHAREMODE_SHARED,
            0,
            kRequestedBuffer,
            0,
            mix,
            nullptr);
        if (FAILED(hr)) {
            PrintHr(L"IAudioClient::Initialize", hr);
            result = 9;
            break;
        }

        UINT32 bufferFrames = 0;
        hr = audioClient->GetBufferSize(&bufferFrames);
        if (FAILED(hr) || bufferFrames == 0) {
            if (FAILED(hr)) {
                PrintHr(L"GetBufferSize", hr);
            } else {
                std::wcerr << L"ERROR\tGetBufferSize\tzero frames\n";
            }
            result = 10;
            break;
        }

        hr = audioClient->GetService(
            __uuidof(IAudioRenderClient), reinterpret_cast<void**>(&renderClient));
        if (FAILED(hr)) {
            PrintHr(L"GetService(IAudioRenderClient)", hr);
            result = 11;
            break;
        }

        BYTE* buffer = nullptr;
        hr = renderClient->GetBuffer(bufferFrames, &buffer);
        if (FAILED(hr)) {
            PrintHr(L"IAudioRenderClient::GetBuffer", hr);
            result = 12;
            break;
        }
        hr = renderClient->ReleaseBuffer(bufferFrames, AUDCLNT_BUFFERFLAGS_SILENT);
        if (FAILED(hr)) {
            PrintHr(L"IAudioRenderClient::ReleaseBuffer", hr);
            result = 13;
            break;
        }

        hr = audioClient->Start();
        if (FAILED(hr)) {
            PrintHr(L"IAudioClient::Start", hr);
            result = 14;
            break;
        }
        std::wcout << L"SHARED_RENDER_START_OK\t" << bufferFrames << L"\n";

        Sleep(250);
        UINT32 padding = 0;
        hr = audioClient->GetCurrentPadding(&padding);
        const HRESULT stopHr = audioClient->Stop();
        if (FAILED(hr)) {
            PrintHr(L"GetCurrentPadding", hr);
            result = 15;
            break;
        }
        if (FAILED(stopHr)) {
            PrintHr(L"IAudioClient::Stop", stopHr);
            result = 16;
            break;
        }

        std::wcout << L"SHARED_RENDER_PROGRESSED_OK\tpadding=" << padding << L"\n";
        std::wcout << L"OMNIPHONY_PRODUCTION_WASAPI_PROBE_OK\t1\n";
        result = 0;
    } while (false);

    if (mix) {
        CoTaskMemFree(mix);
    }
    if (renderClient) {
        renderClient->Release();
    }
    if (audioClient) {
        audioClient->Release();
    }
    if (device) {
        device->Release();
    }
    if (enumerator) {
        enumerator->Release();
    }
    CoUninitialize();
    return result;
}
