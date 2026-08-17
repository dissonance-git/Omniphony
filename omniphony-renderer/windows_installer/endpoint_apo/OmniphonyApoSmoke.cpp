#include <windows.h>
#include <audioenginebaseapo.h>
#include <audioengineextensionapo.h>
#include <audiomediatype.h>
#include <ksmedia.h>

#include <algorithm>
#include <array>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <iostream>
#include <thread>
#include <vector>
#include <wrl/client.h>

using Microsoft::WRL::ComPtr;

namespace {
constexpr GUID kOmniphonyApoClsid = {
    0xa9333bfe, 0x39c1, 0x40fd, {0xb4, 0xb0, 0xec, 0xc5, 0x91, 0x41, 0x0b, 0x47}};
constexpr HNSTIME kExpectedCurrentLatencyHns = 400000; // 40 ms

int ExerciseProcessing(
    IAudioProcessingObject* apo,
    IAudioProcessingObjectRT* rt,
    IAudioProcessingObjectConfiguration* configuration) {
    APOInitSystemEffects initData = {};
    initData.APOInit.cbSize = sizeof(initData);
    initData.APOInit.clsid = kOmniphonyApoClsid;

    std::wcout << L"SMOKE_STAGE\tINITIALIZE_BEGIN" << std::endl;
    HRESULT hr = apo->Initialize(
        sizeof(initData), reinterpret_cast<BYTE*>(&initData));
    std::wcout << L"SMOKE_STAGE\tINITIALIZE_END\t0x" << std::hex << hr << std::endl;
    if (FAILED(hr)) {
        std::wcerr << L"APO_INITIALIZE_FAILED\t0x" << std::hex << hr << std::endl;
        return 6;
    }

    UNCOMPRESSEDAUDIOFORMAT format = {};
    format.guidFormatType = KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
    format.dwSamplesPerFrame = 2;
    format.dwBytesPerSampleContainer = sizeof(float);
    format.dwValidBitsPerSample = 32;
    format.fFramesPerSecond = 48000.0f;
    format.dwChannelMask = SPEAKER_FRONT_LEFT | SPEAKER_FRONT_RIGHT;

    ComPtr<IAudioMediaType> mediaType;
    hr = CreateAudioMediaTypeFromUncompressedAudioFormat(
        &format, mediaType.ReleaseAndGetAddressOf());
    if (FAILED(hr)) {
        std::wcerr << L"APO_MEDIA_TYPE_FAILED\t0x" << std::hex << hr << std::endl;
        return 7;
    }

    constexpr UINT32 kFrames = 960; // one 20 ms Current worker block at 48 kHz
    std::vector<float> input(kFrames * 2);
    std::vector<float> output(kFrames * 2);
    for (UINT32 frame = 0; frame < kFrames; ++frame) {
        input[frame * 2] = 0.035f;
        input[frame * 2 + 1] = -0.020f;
    }

    APO_CONNECTION_DESCRIPTOR inputDescriptor = {};
    inputDescriptor.Type = APO_CONNECTION_BUFFER_TYPE_EXTERNAL;
    inputDescriptor.pBuffer = reinterpret_cast<UINT_PTR>(input.data());
    inputDescriptor.u32MaxFrameCount = kFrames;
    inputDescriptor.pFormat = mediaType.Get();
    inputDescriptor.u32Signature = APO_CONNECTION_DESCRIPTOR_SIGNATURE;

    APO_CONNECTION_DESCRIPTOR outputDescriptor = {};
    outputDescriptor.Type = APO_CONNECTION_BUFFER_TYPE_EXTERNAL;
    outputDescriptor.pBuffer = reinterpret_cast<UINT_PTR>(output.data());
    outputDescriptor.u32MaxFrameCount = kFrames;
    outputDescriptor.pFormat = mediaType.Get();
    outputDescriptor.u32Signature = APO_CONNECTION_DESCRIPTOR_SIGNATURE;

    APO_CONNECTION_DESCRIPTOR* inputDescriptors[] = {&inputDescriptor};
    APO_CONNECTION_DESCRIPTOR* outputDescriptors[] = {&outputDescriptor};

    std::wcout << L"SMOKE_STAGE\tLOCK_BEGIN" << std::endl;
    hr = configuration->LockForProcess(1, inputDescriptors, 1, outputDescriptors);
    std::wcout << L"SMOKE_STAGE\tLOCK_END\t0x" << std::hex << hr << std::endl;
    if (FAILED(hr)) {
        std::wcerr << L"APO_LOCK_FAILED\t0x" << std::hex << hr << std::endl;
        return 8;
    }

    int result = 0;
    HNSTIME latency = -1;
    hr = apo->GetLatency(&latency);
    if (FAILED(hr) || latency != kExpectedCurrentLatencyHns) {
        std::wcerr << L"APO_CURRENT_LATENCY_FAILED\tHR=0x" << std::hex << hr
                   << L"\tLATENCY=" << std::dec << latency << std::endl;
        result = 9;
    } else if (GetModuleHandleW(L"omniphony_realtime.dll") == nullptr) {
        std::wcerr << L"APO_REALTIME_BRIDGE_NOT_RESIDENT" << std::endl;
        result = 10;
    } else {
        APO_CONNECTION_PROPERTY inputProperty = {};
        inputProperty.pBuffer = reinterpret_cast<UINT_PTR>(input.data());
        inputProperty.u32ValidFrameCount = kFrames;
        inputProperty.u32BufferFlags = BUFFER_VALID;
        inputProperty.u32Signature = APO_CONNECTION_PROPERTY_SIGNATURE;

        APO_CONNECTION_PROPERTY outputProperty = {};
        outputProperty.pBuffer = reinterpret_cast<UINT_PTR>(output.data());
        outputProperty.u32Signature = APO_CONNECTION_PROPERTY_SIGNATURE;

        APO_CONNECTION_PROPERTY* inputProperties[] = {&inputProperty};
        APO_CONNECTION_PROPERTY* outputProperties[] = {&outputProperty};

        bool sawNonSilent = false;
        for (int pass = 0; pass < 8; ++pass) {
            std::fill(output.begin(), output.end(), 0.0f);
            outputProperty.u32ValidFrameCount = 0;
            outputProperty.u32BufferFlags = BUFFER_INVALID;
            rt->APOProcess(1, inputProperties, 1, outputProperties);

            if (outputProperty.u32BufferFlags != BUFFER_VALID ||
                outputProperty.u32ValidFrameCount != kFrames) {
                std::wcerr << L"APO_PROCESS_METADATA_FAILED\tFLAGS="
                           << outputProperty.u32BufferFlags << L"\tFRAMES="
                           << outputProperty.u32ValidFrameCount << std::endl;
                result = 11;
                break;
            }
            if (!std::all_of(output.begin(), output.end(), [](float sample) {
                    return std::isfinite(sample);
                })) {
                std::wcerr << L"APO_PROCESS_NONFINITE" << std::endl;
                result = 12;
                break;
            }
            sawNonSilent = sawNonSilent || std::any_of(output.begin(), output.end(), [](float sample) {
                return std::abs(sample) > 1.0e-6f;
            });
            std::this_thread::sleep_for(std::chrono::milliseconds(10));
        }

        if (result == 0 && !sawNonSilent) {
            std::wcerr << L"APO_CURRENT_NEVER_EMITTED_PCM" << std::endl;
            result = 13;
        } else if (result == 0) {
            std::wcout << L"APO_PROCESS_OK\tFRAMES=" << kFrames
                       << L"\tCHANNELS=2\tFORMAT=float32\tMODE=current"
                       << L"\tLATENCY_HNS=" << kExpectedCurrentLatencyHns
                       << L"\tREALTIME_DLL_RESIDENT=1" << std::endl;
        }
    }

    std::wcout << L"SMOKE_STAGE\tUNLOCK_BEGIN" << std::endl;
    const HRESULT unlockHr = configuration->UnlockForProcess();
    std::wcout << L"SMOKE_STAGE\tUNLOCK_END\t0x" << std::hex << unlockHr << std::endl;
    if (FAILED(unlockHr) && result == 0) {
        std::wcerr << L"APO_UNLOCK_FAILED\t0x" << std::hex << unlockHr << std::endl;
        result = 14;
    }
    return result;
}

int ExerciseUnsupportedPcm16Contract(
    IAudioProcessingObjectConfiguration* configuration) {
    UNCOMPRESSEDAUDIOFORMAT format = {};
    format.guidFormatType = KSDATAFORMAT_SUBTYPE_PCM;
    format.dwSamplesPerFrame = 2;
    format.dwBytesPerSampleContainer = sizeof(std::int16_t);
    format.dwValidBitsPerSample = 16;
    format.fFramesPerSecond = 48000.0f;
    format.dwChannelMask = SPEAKER_FRONT_LEFT | SPEAKER_FRONT_RIGHT;

    ComPtr<IAudioMediaType> mediaType;
    HRESULT hr = CreateAudioMediaTypeFromUncompressedAudioFormat(
        &format, mediaType.ReleaseAndGetAddressOf());
    if (FAILED(hr)) {
        std::wcerr << L"APO_PCM16_MEDIA_TYPE_FAILED\t0x" << std::hex << hr << std::endl;
        return 15;
    }

    constexpr UINT32 kFrames = 4;
    alignas(16) std::array<std::int16_t, kFrames * 2> input = {};
    alignas(16) std::array<std::int16_t, kFrames * 2> output = {};

    APO_CONNECTION_DESCRIPTOR inputDescriptor = {};
    inputDescriptor.Type = APO_CONNECTION_BUFFER_TYPE_EXTERNAL;
    inputDescriptor.pBuffer = reinterpret_cast<UINT_PTR>(input.data());
    inputDescriptor.u32MaxFrameCount = kFrames;
    inputDescriptor.pFormat = mediaType.Get();
    inputDescriptor.u32Signature = APO_CONNECTION_DESCRIPTOR_SIGNATURE;

    APO_CONNECTION_DESCRIPTOR outputDescriptor = {};
    outputDescriptor.Type = APO_CONNECTION_BUFFER_TYPE_EXTERNAL;
    outputDescriptor.pBuffer = reinterpret_cast<UINT_PTR>(output.data());
    outputDescriptor.u32MaxFrameCount = kFrames;
    outputDescriptor.pFormat = mediaType.Get();
    outputDescriptor.u32Signature = APO_CONNECTION_DESCRIPTOR_SIGNATURE;

    APO_CONNECTION_DESCRIPTOR* inputDescriptors[] = {&inputDescriptor};
    APO_CONNECTION_DESCRIPTOR* outputDescriptors[] = {&outputDescriptor};

    std::wcout << L"SMOKE_STAGE\tPCM16_LOCK_BEGIN" << std::endl;
    hr = configuration->LockForProcess(1, inputDescriptors, 1, outputDescriptors);
    std::wcout << L"SMOKE_STAGE\tPCM16_LOCK_END\t0x" << std::hex << hr << std::endl;

    if (hr == APOERR_FORMAT_NOT_SUPPORTED) {
        std::wcout << L"APO_PCM16_REJECT_OK\tFORMAT=pcm16\tNEGOTIATION=float32-only"
                   << std::endl;
        return 0;
    }

    if (SUCCEEDED(hr)) {
        const HRESULT unlockHr = configuration->UnlockForProcess();
        if (FAILED(unlockHr)) {
            std::wcerr << L"APO_PCM16_UNEXPECTED_UNLOCK_FAILED\t0x" << std::hex << unlockHr
                       << std::endl;
        }
        std::wcerr << L"APO_PCM16_UNEXPECTEDLY_ACCEPTED" << std::endl;
        return 16;
    }

    std::wcerr << L"APO_PCM16_WRONG_REJECTION\t0x" << std::hex << hr << std::endl;
    return 17;
}
} // namespace

int wmain() {
    const HRESULT init = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(init)) {
        std::wcerr << L"COM_INIT_FAILED\t0x" << std::hex << init << std::endl;
        return 2;
    }

    int result = 0;
    {
        std::wcout << L"SMOKE_STAGE\tCOCREATE_BEGIN" << std::endl;
        ComPtr<IAudioProcessingObject> apo;
        HRESULT hr = CoCreateInstance(kOmniphonyApoClsid, nullptr, CLSCTX_INPROC_SERVER,
                                      IID_PPV_ARGS(apo.ReleaseAndGetAddressOf()));
        if (FAILED(hr)) {
            std::wcerr << L"APO_ACTIVATION_FAILED\t0x" << std::hex << hr << std::endl;
            result = 3;
        } else {
            std::wcout << L"SMOKE_STAGE\tCOCREATE_OK" << std::endl;

            ComPtr<IAudioProcessingObjectRT> rt;
            ComPtr<IAudioProcessingObjectConfiguration> configuration;
            ComPtr<IAudioSystemEffects> effects;

            const HRESULT rtHr = apo.As(&rt);
            const HRESULT cfgHr = apo.As(&configuration);
            const HRESULT fxHr = apo.As(&effects);

            if (FAILED(rtHr) || FAILED(cfgHr) || FAILED(fxHr)) {
                std::wcerr << L"APO_INTERFACE_FAILED\tRT=0x" << std::hex << rtHr
                           << L"\tCFG=0x" << cfgHr << L"\tFX=0x" << fxHr << std::endl;
                result = 4;
            } else {
                HNSTIME preLockLatency = -1;
                hr = apo->GetLatency(&preLockLatency);
                if (FAILED(hr) || preLockLatency != 0) {
                    std::wcerr << L"APO_PRELOCK_LATENCY_FAILED\tHR=0x" << std::hex << hr
                               << L"\tLATENCY=" << std::dec << preLockLatency << std::endl;
                    result = 5;
                } else {
                    result = ExerciseProcessing(apo.Get(), rt.Get(), configuration.Get());
                    if (result == 0) {
                        result = ExerciseUnsupportedPcm16Contract(configuration.Get());
                    }
                    if (result == 0) {
                        std::wcout << L"APO_COM_OK\tCLSID={A9333BFE-39C1-40FD-B4B0-ECC591410B47}"
                                   << L"\tMODE=current\tLATENCY_HNS=" << kExpectedCurrentLatencyHns
                                   << L"\tPROCESSING_SMOKE=1\tPCM16_REJECT=1" << std::endl;
                    }
                }
            }
        }
    }

    CoUninitialize();
    return result;
}
