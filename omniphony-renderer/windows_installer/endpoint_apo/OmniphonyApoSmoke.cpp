#include <windows.h>
#include <audioenginebaseapo.h>
#include <audioengineextensionapo.h>
#include <audiomediatype.h>
#include <ksmedia.h>

#include <array>
#include <cstdint>
#include <cstring>
#include <iostream>
#include <wrl/client.h>

using Microsoft::WRL::ComPtr;

namespace {
constexpr GUID kOmniphonyApoClsid = {
    0xa9333bfe, 0x39c1, 0x40fd, {0xb4, 0xb0, 0xec, 0xc5, 0x91, 0x41, 0x0b, 0x47}};

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

    constexpr UINT32 kFrames = 4;
    alignas(16) std::array<float, kFrames * 2> input = {
        0.0f, -0.25f,
        0.5f, 1.0f,
        -1.0f, 0.125f,
        -0.75f, 0.875f,
    };
    alignas(16) std::array<float, kFrames * 2> output = {};

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
    if (GetModuleHandleW(L"omniphony_realtime.dll") == nullptr) {
        std::wcerr << L"APO_REALTIME_BRIDGE_NOT_RESIDENT" << std::endl;
        result = 9;
    } else {
        APO_CONNECTION_PROPERTY inputProperty = {};
        inputProperty.pBuffer = reinterpret_cast<UINT_PTR>(input.data());
        inputProperty.u32ValidFrameCount = kFrames;
        inputProperty.u32BufferFlags = BUFFER_VALID;
        inputProperty.u32Signature = APO_CONNECTION_PROPERTY_SIGNATURE;

        APO_CONNECTION_PROPERTY outputProperty = {};
        outputProperty.pBuffer = reinterpret_cast<UINT_PTR>(output.data());
        outputProperty.u32ValidFrameCount = 0;
        outputProperty.u32BufferFlags = BUFFER_INVALID;
        outputProperty.u32Signature = APO_CONNECTION_PROPERTY_SIGNATURE;

        APO_CONNECTION_PROPERTY* inputProperties[] = {&inputProperty};
        APO_CONNECTION_PROPERTY* outputProperties[] = {&outputProperty};

        std::wcout << L"SMOKE_STAGE\tPROCESS_BEGIN" << std::endl;
        rt->APOProcess(1, inputProperties, 1, outputProperties);
        std::wcout << L"SMOKE_STAGE\tPROCESS_END" << std::endl;

        if (outputProperty.u32BufferFlags != BUFFER_VALID ||
            outputProperty.u32ValidFrameCount != kFrames) {
            std::wcerr << L"APO_PROCESS_METADATA_FAILED\tFLAGS="
                       << outputProperty.u32BufferFlags << L"\tFRAMES="
                       << outputProperty.u32ValidFrameCount << std::endl;
            result = 10;
        } else if (std::memcmp(input.data(), output.data(), sizeof(input)) != 0) {
            std::wcerr << L"APO_PROCESS_NOT_BIT_EXACT" << std::endl;
            result = 11;
        } else {
            std::wcout << L"APO_PROCESS_OK\tFRAMES=" << kFrames
                       << L"\tCHANNELS=2\tFORMAT=float32\tBIT_EXACT=1\tREALTIME_DLL_RESIDENT=1"
                       << std::endl;
        }
    }

    std::wcout << L"SMOKE_STAGE\tUNLOCK_BEGIN" << std::endl;
    const HRESULT unlockHr = configuration->UnlockForProcess();
    std::wcout << L"SMOKE_STAGE\tUNLOCK_END\t0x" << std::hex << unlockHr << std::endl;
    if (FAILED(unlockHr) && result == 0) {
        std::wcerr << L"APO_UNLOCK_FAILED\t0x" << std::hex << unlockHr << std::endl;
        result = 12;
    }
    return result;
}

int ExercisePcm16IdentityFallback(
    IAudioProcessingObjectRT* rt,
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
        return 13;
    }

    constexpr UINT32 kFrames = 4;
    alignas(16) std::array<std::int16_t, kFrames * 2> input = {
        0, -8192,
        16384, 32767,
        -32768, 4096,
        -24576, 28672,
    };
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
    if (FAILED(hr)) {
        std::wcerr << L"APO_PCM16_LOCK_FAILED\t0x" << std::hex << hr << std::endl;
        return 14;
    }

    int result = 0;
    if (GetModuleHandleW(L"omniphony_realtime.dll") != nullptr) {
        std::wcerr << L"APO_PCM16_UNEXPECTED_REALTIME_BRIDGE" << std::endl;
        result = 15;
    } else {
        APO_CONNECTION_PROPERTY inputProperty = {};
        inputProperty.pBuffer = reinterpret_cast<UINT_PTR>(input.data());
        inputProperty.u32ValidFrameCount = kFrames;
        inputProperty.u32BufferFlags = BUFFER_VALID;
        inputProperty.u32Signature = APO_CONNECTION_PROPERTY_SIGNATURE;

        APO_CONNECTION_PROPERTY outputProperty = {};
        outputProperty.pBuffer = reinterpret_cast<UINT_PTR>(output.data());
        outputProperty.u32ValidFrameCount = 0;
        outputProperty.u32BufferFlags = BUFFER_INVALID;
        outputProperty.u32Signature = APO_CONNECTION_PROPERTY_SIGNATURE;

        APO_CONNECTION_PROPERTY* inputProperties[] = {&inputProperty};
        APO_CONNECTION_PROPERTY* outputProperties[] = {&outputProperty};
        rt->APOProcess(1, inputProperties, 1, outputProperties);

        if (outputProperty.u32BufferFlags != BUFFER_VALID ||
            outputProperty.u32ValidFrameCount != kFrames) {
            std::wcerr << L"APO_PCM16_METADATA_FAILED" << std::endl;
            result = 16;
        } else if (std::memcmp(input.data(), output.data(), sizeof(input)) != 0) {
            std::wcerr << L"APO_PCM16_NOT_BIT_EXACT" << std::endl;
            result = 17;
        } else {
            std::wcout << L"APO_PCM16_FALLBACK_OK\tFRAMES=" << kFrames
                       << L"\tCHANNELS=2\tFORMAT=pcm16\tBIT_EXACT=1\tREALTIME_DLL_RESIDENT=0"
                       << std::endl;
        }
    }

    std::wcout << L"SMOKE_STAGE\tPCM16_UNLOCK_BEGIN" << std::endl;
    const HRESULT unlockHr = configuration->UnlockForProcess();
    std::wcout << L"SMOKE_STAGE\tPCM16_UNLOCK_END\t0x" << std::hex << unlockHr << std::endl;
    if (FAILED(unlockHr) && result == 0) {
        std::wcerr << L"APO_PCM16_UNLOCK_FAILED\t0x" << std::hex << unlockHr << std::endl;
        result = 18;
    }
    return result;
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

            std::wcout << L"SMOKE_STAGE\tQI_RT_BEGIN" << std::endl;
            const HRESULT rtHr = apo.As(&rt);
            std::wcout << L"SMOKE_STAGE\tQI_RT_END\t0x" << std::hex << rtHr << std::endl;

            std::wcout << L"SMOKE_STAGE\tQI_CFG_BEGIN" << std::endl;
            const HRESULT cfgHr = apo.As(&configuration);
            std::wcout << L"SMOKE_STAGE\tQI_CFG_END\t0x" << std::hex << cfgHr << std::endl;

            std::wcout << L"SMOKE_STAGE\tQI_FX_BEGIN" << std::endl;
            const HRESULT fxHr = apo.As(&effects);
            std::wcout << L"SMOKE_STAGE\tQI_FX_END\t0x" << std::hex << fxHr << std::endl;

            if (FAILED(rtHr) || FAILED(cfgHr) || FAILED(fxHr)) {
                std::wcerr << L"APO_INTERFACE_FAILED\tRT=0x" << std::hex << rtHr
                           << L"\tCFG=0x" << cfgHr << L"\tFX=0x" << fxHr << std::endl;
                result = 4;
            } else {
                HNSTIME latency = -1;
                std::wcout << L"SMOKE_STAGE\tLATENCY_BEGIN" << std::endl;
                hr = apo->GetLatency(&latency);
                std::wcout << L"SMOKE_STAGE\tLATENCY_END\t0x" << std::hex << hr
                           << L"\t" << std::dec << latency << std::endl;
                if (FAILED(hr) || latency != 0) {
                    std::wcerr << L"APO_LATENCY_FAILED\tHR=0x" << std::hex << hr
                               << L"\tLATENCY=" << std::dec << latency << std::endl;
                    result = 5;
                } else {
                    result = ExerciseProcessing(apo.Get(), rt.Get(), configuration.Get());
                    if (result == 0) {
                        result = ExercisePcm16IdentityFallback(rt.Get(), configuration.Get());
                    }
                    if (result == 0) {
                        std::wcout << L"APO_COM_OK\tCLSID={A9333BFE-39C1-40FD-B4B0-ECC591410B47}"
                                   << L"\tLATENCY_HNS=0\tPROCESSING_SMOKE=1\tPCM16_FALLBACK=1"
                                   << std::endl;
                    }
                }
            }
        }
    }

    CoUninitialize();
    return result;
}
