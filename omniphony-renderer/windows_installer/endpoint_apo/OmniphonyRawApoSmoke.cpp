#include <windows.h>
#include <audioenginebaseapo.h>
#include <audioengineextensionapo.h>
#include <audiomediatype.h>
#include <ksmedia.h>

#include <algorithm>
#include <array>
#include <cstdint>
#include <cstring>
#include <iostream>
#include <wrl/client.h>

using Microsoft::WRL::ComPtr;

namespace {

constexpr GUID kEndpointClsid = {
    0xa9333bfe, 0x39c1, 0x40fd, {0xb4, 0xb0, 0xec, 0xc5, 0x91, 0x41, 0x0b, 0x47}};
constexpr GUID kStreamClsid = {
    0x07d403d9, 0x8a98, 0x43ef, {0x8c, 0x28, 0x86, 0x51, 0x75, 0x6d, 0x83, 0xbe}};
constexpr DWORD kStereoMask = SPEAKER_FRONT_LEFT | SPEAKER_FRONT_RIGHT;
constexpr DWORD kSevenOneMask =
    SPEAKER_FRONT_LEFT | SPEAKER_FRONT_RIGHT | SPEAKER_FRONT_CENTER |
    SPEAKER_LOW_FREQUENCY | SPEAKER_BACK_LEFT | SPEAKER_BACK_RIGHT |
    SPEAKER_SIDE_LEFT | SPEAKER_SIDE_RIGHT;
constexpr UINT32 kFrames = 480;

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

bool IsStereoFloat48k(IAudioMediaType* mediaType) {
    UNCOMPRESSEDAUDIOFORMAT format = {};
    return mediaType &&
           SUCCEEDED(mediaType->GetUncompressedAudioFormat(&format)) &&
           IsEqualGUID(format.guidFormatType, KSDATAFORMAT_SUBTYPE_IEEE_FLOAT) &&
           format.dwSamplesPerFrame == 2 &&
           format.dwBytesPerSampleContainer == sizeof(float) &&
           format.dwValidBitsPerSample == 32 &&
           format.fFramesPerSecond == 48000.0f &&
           format.dwChannelMask == kStereoMask;
}

HRESULT InitializeRaw(IAudioProcessingObject* apo, REFGUID clsid) {
    APOInitSystemEffects2 init = {};
    init.APOInit.cbSize = sizeof(init);
    init.APOInit.clsid = clsid;
    init.AudioProcessingMode = AUDIO_SIGNALPROCESSINGMODE_RAW;
    return apo->Initialize(sizeof(init), reinterpret_cast<BYTE*>(&init));
}

struct ApoInterfaces {
    ComPtr<IAudioProcessingObject> apo;
    ComPtr<IAudioProcessingObjectRT> rt;
    ComPtr<IAudioProcessingObjectConfiguration> configuration;
};

bool ActivateRaw(REFGUID clsid, ApoInterfaces& handles) {
    HRESULT hr = CoCreateInstance(
        clsid,
        nullptr,
        CLSCTX_INPROC_SERVER,
        IID_PPV_ARGS(handles.apo.ReleaseAndGetAddressOf()));
    if (FAILED(hr)) {
        std::wcerr << L"RAW_APO_ACTIVATION_FAILED hr=0x" << std::hex << hr << std::endl;
        return false;
    }
    if (FAILED(handles.apo.As(&handles.rt)) ||
        FAILED(handles.apo.As(&handles.configuration))) {
        std::wcerr << L"RAW_APO_INTERFACE_FAILED" << std::endl;
        return false;
    }
    hr = InitializeRaw(handles.apo.Get(), clsid);
    if (FAILED(hr)) {
        std::wcerr << L"RAW_APO_INITIALIZE_FAILED hr=0x" << std::hex << hr << std::endl;
        return false;
    }
    return true;
}

void FillInput(std::array<float, kFrames * 2>& input) {
    for (UINT32 frame = 0; frame < kFrames; ++frame) {
        const int left = static_cast<int>(frame % 31) - 15;
        const int right = static_cast<int>(frame % 23) - 11;
        input[static_cast<size_t>(frame) * 2] = static_cast<float>(left) * 0.001f;
        input[static_cast<size_t>(frame) * 2 + 1] = static_cast<float>(right) * -0.0013f;
    }
}

int ExerciseRawProcessing(
    const wchar_t* label,
    ApoInterfaces& handles,
    IAudioMediaType* inputType,
    IAudioMediaType* outputType) {
    std::array<float, kFrames * 2> input = {};
    std::array<float, kFrames * 2> output = {};
    FillInput(input);

    APO_CONNECTION_DESCRIPTOR inputDescriptor = {};
    inputDescriptor.Type = APO_CONNECTION_BUFFER_TYPE_EXTERNAL;
    inputDescriptor.pBuffer = reinterpret_cast<UINT_PTR>(input.data());
    inputDescriptor.u32MaxFrameCount = kFrames;
    inputDescriptor.pFormat = inputType;
    inputDescriptor.u32Signature = APO_CONNECTION_DESCRIPTOR_SIGNATURE;

    APO_CONNECTION_DESCRIPTOR outputDescriptor = {};
    outputDescriptor.Type = APO_CONNECTION_BUFFER_TYPE_EXTERNAL;
    outputDescriptor.pBuffer = reinterpret_cast<UINT_PTR>(output.data());
    outputDescriptor.u32MaxFrameCount = kFrames;
    outputDescriptor.pFormat = outputType;
    outputDescriptor.u32Signature = APO_CONNECTION_DESCRIPTOR_SIGNATURE;

    APO_CONNECTION_DESCRIPTOR* inputDescriptors[] = {&inputDescriptor};
    APO_CONNECTION_DESCRIPTOR* outputDescriptors[] = {&outputDescriptor};
    HRESULT hr = handles.configuration->LockForProcess(
        1, inputDescriptors, 1, outputDescriptors);
    if (FAILED(hr)) {
        std::wcerr << L"RAW_APO_LOCK_FAILED label=" << label
                   << L" hr=0x" << std::hex << hr << std::endl;
        return 1;
    }

    int result = 0;
    HNSTIME latency = -1;
    hr = handles.apo->GetLatency(&latency);
    if (FAILED(hr) || latency != 0) {
        std::wcerr << L"RAW_APO_LATENCY_FAILED label=" << label
                   << L" hr=0x" << std::hex << hr
                   << L" latency=" << std::dec << latency << std::endl;
        result = 2;
    } else if (GetModuleHandleW(L"omniphony_realtime.dll") != nullptr) {
        std::wcerr << L"RAW_APO_REALTIME_DLL_UNEXPECTED label=" << label << std::endl;
        result = 3;
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

    if (result == 0) {
        std::fill(output.begin(), output.end(), 0.75f);
        handles.rt->APOProcess(1, inputProperties, 1, outputProperties);
        if (outputProperty.u32BufferFlags != BUFFER_VALID ||
            outputProperty.u32ValidFrameCount != kFrames ||
            std::memcmp(input.data(), output.data(), sizeof(input)) != 0) {
            std::wcerr << L"RAW_APO_BIT_TRANSPARENCY_FAILED label=" << label << std::endl;
            result = 4;
        }
    }

    if (result == 0) {
        std::fill(output.begin(), output.end(), 0.75f);
        inputProperty.pBuffer = 0;
        inputProperty.u32BufferFlags = BUFFER_SILENT;
        outputProperty.u32BufferFlags = BUFFER_INVALID;
        outputProperty.u32ValidFrameCount = 0;
        handles.rt->APOProcess(1, inputProperties, 1, outputProperties);
        const bool allZero = std::all_of(output.begin(), output.end(), [](float sample) {
            return sample == 0.0f;
        });
        if (outputProperty.u32BufferFlags != BUFFER_SILENT ||
            outputProperty.u32ValidFrameCount != kFrames ||
            !allZero) {
            std::wcerr << L"RAW_APO_SILENCE_FAILED label=" << label << std::endl;
            result = 5;
        }
    }

    const HRESULT unlockHr = handles.configuration->UnlockForProcess();
    if (FAILED(unlockHr) && result == 0) {
        std::wcerr << L"RAW_APO_UNLOCK_FAILED label=" << label
                   << L" hr=0x" << std::hex << unlockHr << std::endl;
        result = 6;
    }
    return result;
}

int ExerciseEndpointRaw() {
    ApoInterfaces handles;
    if (!ActivateRaw(kEndpointClsid, handles)) return 10;
    auto stereo = FloatMediaType(2, kStereoMask);
    if (!stereo) return 11;

    const int result = ExerciseRawProcessing(
        L"endpoint",
        handles,
        stereo.Get(),
        stereo.Get());
    if (result != 0) return 12 + result;

    std::wcout << L"ENDPOINT_APO_RAW_BYPASS_OK 1" << std::endl;
    return 0;
}

int ExerciseStreamRaw() {
    ApoInterfaces handles;
    if (!ActivateRaw(kStreamClsid, handles)) return 30;

    ComPtr<IAudioProcessingObjectPreferredFormatSupport> preferred;
    if (FAILED(handles.apo.As(&preferred))) {
        std::wcerr << L"STREAM_RAW_PREFERRED_INTERFACE_FAILED" << std::endl;
        return 31;
    }

    auto stereo = FloatMediaType(2, kStereoMask);
    auto sevenOne = FloatMediaType(8, kSevenOneMask);
    if (!stereo || !sevenOne) return 32;

    ComPtr<IAudioMediaType> preferredInput;
    HRESULT hr = preferred->GetPreferredInputFormat(
        stereo.Get(), preferredInput.ReleaseAndGetAddressOf());
    if (hr != S_OK || !IsStereoFloat48k(preferredInput.Get())) {
        std::wcerr << L"STREAM_RAW_PREFERRED_INPUT_NOT_IDENTITY hr=0x"
                   << std::hex << hr << std::endl;
        return 33;
    }

    ComPtr<IAudioMediaType> preferredOutput;
    hr = preferred->GetPreferredOutputFormat(
        stereo.Get(), preferredOutput.ReleaseAndGetAddressOf());
    if (hr != S_OK || !IsStereoFloat48k(preferredOutput.Get())) {
        std::wcerr << L"STREAM_RAW_PREFERRED_OUTPUT_NOT_IDENTITY hr=0x"
                   << std::hex << hr << std::endl;
        return 34;
    }

    ComPtr<IAudioMediaType> negotiatedInput;
    hr = handles.apo->IsInputFormatSupported(
        stereo.Get(),
        stereo.Get(),
        negotiatedInput.ReleaseAndGetAddressOf());
    if (hr != S_OK || !IsStereoFloat48k(negotiatedInput.Get())) {
        std::wcerr << L"STREAM_RAW_STEREO_INPUT_NEGOTIATION_FAILED hr=0x"
                   << std::hex << hr << std::endl;
        return 35;
    }

    ComPtr<IAudioMediaType> rejectedSevenOne;
    hr = handles.apo->IsInputFormatSupported(
        stereo.Get(),
        sevenOne.Get(),
        rejectedSevenOne.ReleaseAndGetAddressOf());
    if (hr != APOERR_FORMAT_NOT_SUPPORTED || rejectedSevenOne) {
        std::wcerr << L"STREAM_RAW_71_NOT_REJECTED hr=0x"
                   << std::hex << hr << std::endl;
        return 36;
    }

    const int result = ExerciseRawProcessing(
        L"stream",
        handles,
        stereo.Get(),
        stereo.Get());
    if (result != 0) return 37 + result;

    std::wcout << L"STREAM_APO_RAW_BYPASS_OK 1" << std::endl;
    return 0;
}

} // namespace

int wmain() {
    const HRESULT init = CoInitializeEx(nullptr, COINIT_MULTITHREADED);
    if (FAILED(init)) {
        std::wcerr << L"RAW_APO_COM_INIT_FAILED hr=0x" << std::hex << init << std::endl;
        return 2;
    }

    int result = ExerciseEndpointRaw();
    if (result == 0) {
        result = ExerciseStreamRaw();
    }

    if (result == 0) {
        std::wcout << L"OMNIPHONY_RAW_APO_BYPASS_OK 1" << std::endl;
        std::wcout << L"OMNIPHONY_RAW_APO_REALTIME_DLL_LOADED 0" << std::endl;
        std::wcout << L"OMNIPHONY_RAW_APO_LATENCY_HNS 0" << std::endl;
    }

    CoUninitialize();
    return result;
}
