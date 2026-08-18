#include <windows.h>
#include <audioenginebaseapo.h>
#include <audioengineextensionapo.h>
#include <audiomediatype.h>
#include <ksmedia.h>

#include <algorithm>
#include <chrono>
#include <cmath>
#include <cstdint>
#include <iostream>
#include <thread>
#include <vector>
#include <wrl/client.h>

using Microsoft::WRL::ComPtr;

namespace {
constexpr GUID kOmniphonyStreamApoClsid = {
    0x07d403d9, 0x8a98, 0x43ef, {0x8c, 0x28, 0x86, 0x51, 0x75, 0x6d, 0x83, 0xbe}};
constexpr HNSTIME kExpectedLatencyHns = 400000; // 40 ms
constexpr UINT32 kFrames = 960; // 20 ms @ 48 kHz
constexpr DWORD kMask714 =
    SPEAKER_FRONT_LEFT | SPEAKER_FRONT_RIGHT | SPEAKER_FRONT_CENTER |
    SPEAKER_LOW_FREQUENCY | SPEAKER_BACK_LEFT | SPEAKER_BACK_RIGHT |
    SPEAKER_SIDE_LEFT | SPEAKER_SIDE_RIGHT | SPEAKER_TOP_FRONT_LEFT |
    SPEAKER_TOP_FRONT_RIGHT | SPEAKER_TOP_BACK_LEFT | SPEAKER_TOP_BACK_RIGHT;

ComPtr<IAudioMediaType> FloatMediaType(UINT32 channels, DWORD channelMask) {
    UNCOMPRESSEDAUDIOFORMAT format = {};
    format.guidFormatType = KSDATAFORMAT_SUBTYPE_IEEE_FLOAT;
    format.dwSamplesPerFrame = channels;
    format.dwBytesPerSampleContainer = sizeof(float);
    format.dwValidBitsPerSample = 32;
    format.fFramesPerSecond = 48000.0f;
    format.dwChannelMask = channelMask;
    ComPtr<IAudioMediaType> mediaType;
    if (FAILED(CreateAudioMediaTypeFromUncompressedAudioFormat(
            &format, mediaType.ReleaseAndGetAddressOf()))) {
        return nullptr;
    }
    return mediaType;
}

ComPtr<IAudioMediaType> Pcm16MediaType(UINT32 channels, DWORD channelMask) {
    UNCOMPRESSEDAUDIOFORMAT format = {};
    format.guidFormatType = KSDATAFORMAT_SUBTYPE_PCM;
    format.dwSamplesPerFrame = channels;
    format.dwBytesPerSampleContainer = sizeof(std::int16_t);
    format.dwValidBitsPerSample = 16;
    format.fFramesPerSecond = 48000.0f;
    format.dwChannelMask = channelMask;
    ComPtr<IAudioMediaType> mediaType;
    if (FAILED(CreateAudioMediaTypeFromUncompressedAudioFormat(
            &format, mediaType.ReleaseAndGetAddressOf()))) {
        return nullptr;
    }
    return mediaType;
}

struct ApoHandles {
    ComPtr<IAudioProcessingObject> apo;
    ComPtr<IAudioProcessingObjectRT> rt;
    ComPtr<IAudioProcessingObjectConfiguration> configuration;
};

bool Activate(ApoHandles& handles) {
    HRESULT hr = CoCreateInstance(
        kOmniphonyStreamApoClsid, nullptr, CLSCTX_INPROC_SERVER,
        IID_PPV_ARGS(handles.apo.ReleaseAndGetAddressOf()));
    if (FAILED(hr)) {
        std::wcerr << L"STREAM_APO_ACTIVATION_FAILED\t0x" << std::hex << hr << std::endl;
        return false;
    }
    if (FAILED(handles.apo.As(&handles.rt)) || FAILED(handles.apo.As(&handles.configuration))) {
        std::wcerr << L"STREAM_APO_INTERFACE_FAILED" << std::endl;
        return false;
    }

    APOInitSystemEffects initData = {};
    initData.APOInit.cbSize = sizeof(initData);
    initData.APOInit.clsid = kOmniphonyStreamApoClsid;
    hr = handles.apo->Initialize(sizeof(initData), reinterpret_cast<BYTE*>(&initData));
    if (FAILED(hr)) {
        std::wcerr << L"STREAM_APO_INITIALIZE_FAILED\t0x" << std::hex << hr << std::endl;
        return false;
    }
    return true;
}

bool NegotiateExactPair(
    IAudioProcessingObject* apo,
    IAudioMediaType* requestedInput,
    IAudioMediaType* requestedOutput,
    UINT32 expectedInputChannels,
    DWORD expectedInputMask,
    ComPtr<IAudioMediaType>& negotiatedInput,
    ComPtr<IAudioMediaType>& negotiatedOutput,
    const wchar_t* label) {
    HRESULT hr = apo->IsInputFormatSupported(
        requestedOutput, requestedInput, negotiatedInput.ReleaseAndGetAddressOf());
    if (hr != S_OK || !negotiatedInput) {
        std::wcerr << L"STREAM_APO_INPUT_NEGOTIATION_FAILED\t" << label
                   << L"\t0x" << std::hex << hr << std::endl;
        return false;
    }

    hr = apo->IsOutputFormatSupported(
        negotiatedInput.Get(), requestedOutput, negotiatedOutput.ReleaseAndGetAddressOf());
    if (hr != S_OK || !negotiatedOutput) {
        std::wcerr << L"STREAM_APO_OUTPUT_NEGOTIATION_FAILED\t" << label
                   << L"\t0x" << std::hex << hr << std::endl;
        return false;
    }

    UNCOMPRESSEDAUDIOFORMAT inputFormat = {};
    UNCOMPRESSEDAUDIOFORMAT outputFormat = {};
    if (FAILED(negotiatedInput->GetUncompressedAudioFormat(&inputFormat)) ||
        FAILED(negotiatedOutput->GetUncompressedAudioFormat(&outputFormat)) ||
        !IsEqualGUID(inputFormat.guidFormatType, KSDATAFORMAT_SUBTYPE_IEEE_FLOAT) ||
        !IsEqualGUID(outputFormat.guidFormatType, KSDATAFORMAT_SUBTYPE_IEEE_FLOAT) ||
        inputFormat.dwSamplesPerFrame != expectedInputChannels ||
        inputFormat.dwChannelMask != expectedInputMask ||
        outputFormat.dwSamplesPerFrame != 2 ||
        inputFormat.fFramesPerSecond != 48000.0f ||
        outputFormat.fFramesPerSecond != 48000.0f) {
        std::wcerr << L"STREAM_APO_NEGOTIATED_FORMAT_MISMATCH\t" << label << std::endl;
        return false;
    }

    std::wcout << L"STREAM_APO_NEGOTIATION_OK\t" << label
               << L"\tINPUT_CHANNELS=" << expectedInputChannels
               << L"\tOUTPUT_CHANNELS=2" << std::endl;
    return true;
}

int ExerciseFloatPath(UINT32 inputChannels, DWORD inputMask, const wchar_t* label) {
    ApoHandles handles;
    if (!Activate(handles)) return 3;

    auto inputType = FloatMediaType(inputChannels, inputMask);
    auto outputType = FloatMediaType(2, SPEAKER_FRONT_LEFT | SPEAKER_FRONT_RIGHT);
    if (!inputType || !outputType) return 4;

    ComPtr<IAudioMediaType> negotiatedInput;
    ComPtr<IAudioMediaType> negotiatedOutput;
    if (!NegotiateExactPair(
            handles.apo.Get(), inputType.Get(), outputType.Get(), inputChannels, inputMask,
            negotiatedInput, negotiatedOutput, label)) {
        return 5;
    }

    std::vector<float> input(static_cast<size_t>(kFrames) * inputChannels, 0.0f);
    std::vector<float> output(static_cast<size_t>(kFrames) * 2, 0.0f);
    for (UINT32 frame = 0; frame < kFrames; ++frame) {
        const float t = static_cast<float>(frame) / 48000.0f;
        for (UINT32 channel = 0; channel < inputChannels; ++channel) {
            const float frequency = 180.0f + static_cast<float>(channel) * 137.0f;
            input[static_cast<size_t>(frame) * inputChannels + channel] =
                0.015f * std::sin(6.28318530718f * frequency * t);
        }
    }

    APO_CONNECTION_DESCRIPTOR inputDescriptor = {};
    inputDescriptor.Type = APO_CONNECTION_BUFFER_TYPE_EXTERNAL;
    inputDescriptor.pBuffer = reinterpret_cast<UINT_PTR>(input.data());
    inputDescriptor.u32MaxFrameCount = kFrames;
    inputDescriptor.pFormat = negotiatedInput.Get();
    inputDescriptor.u32Signature = APO_CONNECTION_DESCRIPTOR_SIGNATURE;

    APO_CONNECTION_DESCRIPTOR outputDescriptor = {};
    outputDescriptor.Type = APO_CONNECTION_BUFFER_TYPE_EXTERNAL;
    outputDescriptor.pBuffer = reinterpret_cast<UINT_PTR>(output.data());
    outputDescriptor.u32MaxFrameCount = kFrames;
    outputDescriptor.pFormat = negotiatedOutput.Get();
    outputDescriptor.u32Signature = APO_CONNECTION_DESCRIPTOR_SIGNATURE;

    APO_CONNECTION_DESCRIPTOR* inputDescriptors[] = {&inputDescriptor};
    APO_CONNECTION_DESCRIPTOR* outputDescriptors[] = {&outputDescriptor};
    HRESULT hr = handles.configuration->LockForProcess(
        1, inputDescriptors, 1, outputDescriptors);
    if (FAILED(hr)) {
        std::wcerr << L"STREAM_APO_LOCK_FAILED\t" << label << L"\t0x" << std::hex << hr << std::endl;
        return 6;
    }

    int result = 0;
    HNSTIME latency = -1;
    hr = handles.apo->GetLatency(&latency);
    if (FAILED(hr) || latency != kExpectedLatencyHns) {
        std::wcerr << L"STREAM_APO_LATENCY_FAILED\t" << label << L"\tHR=0x" << std::hex << hr
                   << L"\tLATENCY=" << std::dec << latency << std::endl;
        result = 7;
    }

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
    if (result == 0) {
        for (int pass = 0; pass < 10; ++pass) {
            std::fill(output.begin(), output.end(), 0.0f);
            outputProperty.u32ValidFrameCount = 0;
            outputProperty.u32BufferFlags = BUFFER_INVALID;
            handles.rt->APOProcess(1, inputProperties, 1, outputProperties);

            if (outputProperty.u32ValidFrameCount != kFrames ||
                (outputProperty.u32BufferFlags != BUFFER_VALID &&
                 outputProperty.u32BufferFlags != BUFFER_SILENT)) {
                std::wcerr << L"STREAM_APO_METADATA_FAILED\t" << label << std::endl;
                result = 8;
                break;
            }
            if (!std::all_of(output.begin(), output.end(), [](float sample) {
                    return std::isfinite(sample);
                })) {
                std::wcerr << L"STREAM_APO_NONFINITE\t" << label << std::endl;
                result = 9;
                break;
            }
            sawNonSilent = sawNonSilent || std::any_of(output.begin(), output.end(), [](float sample) {
                return std::abs(sample) > 1.0e-6f;
            });
            std::this_thread::sleep_for(std::chrono::milliseconds(12));
        }
    }

    if (result == 0 && !sawNonSilent) {
        std::wcerr << L"STREAM_APO_NEVER_EMITTED_PCM\t" << label << std::endl;
        result = 10;
    }

    const HRESULT unlockHr = handles.configuration->UnlockForProcess();
    if (FAILED(unlockHr) && result == 0) result = 11;
    if (result == 0) {
        std::wcout << L"STREAM_APO_PROCESS_OK\t" << label
                   << L"\tINPUT_CHANNELS=" << inputChannels
                   << L"\tOUTPUT_CHANNELS=2\tLATENCY_HNS=" << kExpectedLatencyHns << std::endl;
    }
    return result;
}

int ExercisePcm16Rejection() {
    ApoHandles handles;
    if (!Activate(handles)) return 12;
    auto inputType = Pcm16MediaType(12, kMask714);
    auto outputType = Pcm16MediaType(2, SPEAKER_FRONT_LEFT | SPEAKER_FRONT_RIGHT);
    if (!inputType || !outputType) return 13;

    ComPtr<IAudioMediaType> negotiatedInput;
    const HRESULT negotiationHr = handles.apo->IsInputFormatSupported(
        outputType.Get(), inputType.Get(), negotiatedInput.ReleaseAndGetAddressOf());
    if (negotiationHr != APOERR_FORMAT_NOT_SUPPORTED || negotiatedInput) {
        std::wcerr << L"STREAM_APO_PCM16_NEGOTIATION_REJECTION_FAILED\t0x"
                   << std::hex << negotiationHr << std::endl;
        return 14;
    }

    APO_CONNECTION_DESCRIPTOR inputDescriptor = {};
    inputDescriptor.Type = APO_CONNECTION_BUFFER_TYPE_EXTERNAL;
    inputDescriptor.u32MaxFrameCount = 4;
    inputDescriptor.pFormat = inputType.Get();
    inputDescriptor.u32Signature = APO_CONNECTION_DESCRIPTOR_SIGNATURE;
    APO_CONNECTION_DESCRIPTOR outputDescriptor = {};
    outputDescriptor.Type = APO_CONNECTION_BUFFER_TYPE_EXTERNAL;
    outputDescriptor.u32MaxFrameCount = 4;
    outputDescriptor.pFormat = outputType.Get();
    outputDescriptor.u32Signature = APO_CONNECTION_DESCRIPTOR_SIGNATURE;
    APO_CONNECTION_DESCRIPTOR* inputs[] = {&inputDescriptor};
    APO_CONNECTION_DESCRIPTOR* outputs[] = {&outputDescriptor};
    const HRESULT hr = handles.configuration->LockForProcess(1, inputs, 1, outputs);
    if (hr == APOERR_FORMAT_NOT_SUPPORTED) {
        std::wcout << L"STREAM_APO_PCM16_REJECT_OK" << std::endl;
        return 0;
    }
    if (SUCCEEDED(hr)) handles.configuration->UnlockForProcess();
    std::wcerr << L"STREAM_APO_PCM16_REJECTION_FAILED\t0x" << std::hex << hr << std::endl;
    return 15;
}
} // namespace

int wmain() {
    const HRESULT init = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(init)) return 2;

    int result = ExerciseFloatPath(
        2, SPEAKER_FRONT_LEFT | SPEAKER_FRONT_RIGHT, L"stereo-current");
    if (result == 0) result = ExerciseFloatPath(12, kMask714, L"authored-7.1.4");
    if (result == 0) result = ExercisePcm16Rejection();

    if (result == 0) {
        std::wcout << L"STREAM_APO_COM_OK\tCLSID={07D403D9-8A98-43EF-8C28-8651756D83BE}"
                   << L"\tSTEREO_CURRENT=1\tNATIVE_7_1_4=1\tNEGOTIATION=1\tPCM16_REJECT=1"
                   << std::endl;
    }
    CoUninitialize();
    return result;
}
