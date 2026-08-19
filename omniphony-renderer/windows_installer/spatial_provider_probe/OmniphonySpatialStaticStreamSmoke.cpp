#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <mmreg.h>
#include <spatialaudioclient.h>

#include <algorithm>
#include <cstdint>
#include <iostream>

#include "OmniphonySpatialStaticStream.h"

namespace {

int Fail(const wchar_t* stage, HRESULT hr) {
    std::wcerr << L"SPATIAL_STATIC_STREAM_SMOKE_FAIL stage=" << stage
               << L" hr=0x" << std::hex << std::uppercase
               << static_cast<unsigned long>(hr) << std::dec << L"\n";
    return 1;
}

WAVEFORMATEX ObjectFormat() {
    WAVEFORMATEX format{};
    format.wFormatTag = WAVE_FORMAT_IEEE_FLOAT;
    format.nChannels = 1;
    format.nSamplesPerSec = 48'000;
    format.wBitsPerSample = 32;
    format.nBlockAlign = sizeof(float);
    format.nAvgBytesPerSec = format.nSamplesPerSec * format.nBlockAlign;
    return format;
}

} // namespace

int wmain() {
    auto format = ObjectFormat();
    SpatialAudioObjectRenderStreamActivationParams params{};
    params.ObjectFormat = &format;
    params.StaticObjectTypeMask = static_cast<AudioObjectType>(
        static_cast<std::uint32_t>(AudioObjectType_FrontLeft) |
        static_cast<std::uint32_t>(AudioObjectType_TopFrontLeft));
    params.MinDynamicObjectCount = 0;
    params.MaxDynamicObjectCount = 0;
    params.Category = AudioCategory_GameEffects;
    params.EventHandle = nullptr;
    params.NotifyObject = nullptr;

    ISpatialAudioObjectRenderStream* stream = nullptr;
    HRESULT hr = CreateOmniphonyStaticProbeStream(params, &stream);
    if (FAILED(hr) || !stream) {
        return Fail(L"CreateOmniphonyStaticProbeStream", hr);
    }

    UINT32 dynamicCount = 99;
    hr = stream->GetAvailableDynamicObjectCount(&dynamicCount);
    if (FAILED(hr) || dynamicCount != 0) {
        stream->Release();
        return Fail(L"GetAvailableDynamicObjectCount", FAILED(hr) ? hr : E_FAIL);
    }

    ISpatialAudioObject* top = nullptr;
    hr = stream->ActivateSpatialAudioObject(AudioObjectType_TopFrontLeft, &top);
    if (FAILED(hr) || !top) {
        stream->Release();
        return Fail(L"ActivateSpatialAudioObject(TFL)", hr);
    }

    ISpatialAudioObject* duplicate = nullptr;
    hr = stream->ActivateSpatialAudioObject(AudioObjectType_TopFrontLeft, &duplicate);
    if (hr != SPTLAUDCLNT_E_OBJECT_ALREADY_ACTIVE || duplicate != nullptr) {
        top->Release();
        stream->Release();
        return Fail(L"duplicate-static-role", hr);
    }

    ISpatialAudioObject* unavailable = nullptr;
    hr = stream->ActivateSpatialAudioObject(AudioObjectType_FrontRight, &unavailable);
    if (hr != SPTLAUDCLNT_E_STATIC_OBJECT_NOT_AVAILABLE || unavailable != nullptr) {
        top->Release();
        stream->Release();
        return Fail(L"unrequested-static-role", hr);
    }

    ISpatialAudioObject* dynamic = nullptr;
    hr = stream->ActivateSpatialAudioObject(AudioObjectType_Dynamic, &dynamic);
    if (hr != SPTLAUDCLNT_E_NO_MORE_OBJECTS || dynamic != nullptr) {
        top->Release();
        stream->Release();
        return Fail(L"dynamic-capacity-zero", hr);
    }

    BYTE* buffer = nullptr;
    UINT32 bufferBytes = 0;
    hr = top->GetBuffer(&buffer, &bufferBytes);
    if (hr != SPTLAUDCLNT_E_OUT_OF_ORDER || buffer != nullptr || bufferBytes != 0) {
        top->Release();
        stream->Release();
        return Fail(L"GetBuffer-before-Begin", hr);
    }

    hr = stream->Start();
    if (FAILED(hr)) {
        top->Release();
        stream->Release();
        return Fail(L"Start", hr);
    }
    const HRESULT secondStart = stream->Start();
    if (secondStart != SPTLAUDCLNT_E_STREAM_NOT_STOPPED) {
        top->Release();
        stream->Release();
        return Fail(L"Start-twice", secondStart);
    }

    UINT32 available = 99;
    UINT32 frames = 0;
    hr = stream->BeginUpdatingAudioObjects(&available, &frames);
    if (FAILED(hr) || available != 0 || frames != 480) {
        top->Release();
        stream->Release();
        return Fail(L"BeginUpdatingAudioObjects", FAILED(hr) ? hr : E_FAIL);
    }

    hr = top->SetPosition(-0.5f, 0.7f, -0.5f);
    if (hr != SPTLAUDCLNT_E_PROPERTY_NOT_SUPPORTED) {
        top->Release();
        stream->Release();
        return Fail(L"static-SetPosition", hr);
    }
    hr = top->SetVolume(0.5f);
    if (FAILED(hr)) {
        top->Release();
        stream->Release();
        return Fail(L"SetVolume", hr);
    }

    hr = top->GetBuffer(&buffer, &bufferBytes);
    if (FAILED(hr) || !buffer || bufferBytes != 480u * sizeof(float)) {
        top->Release();
        stream->Release();
        return Fail(L"GetBuffer", FAILED(hr) ? hr : E_FAIL);
    }
    auto* samples = reinterpret_cast<float*>(buffer);
    std::fill(samples, samples + frames, 0.125f);

    hr = stream->EndUpdatingAudioObjects();
    if (FAILED(hr)) {
        top->Release();
        stream->Release();
        return Fail(L"EndUpdatingAudioObjects", hr);
    }

    BOOL active = FALSE;
    hr = top->IsActive(&active);
    if (FAILED(hr) || !active) {
        top->Release();
        stream->Release();
        return Fail(L"object-survives-buffered-pass", FAILED(hr) ? hr : E_FAIL);
    }

    // A pass in which GetBuffer is omitted must implicitly end the object.
    hr = stream->BeginUpdatingAudioObjects(&available, &frames);
    if (FAILED(hr)) {
        top->Release();
        stream->Release();
        return Fail(L"BeginUpdatingAudioObjects-second-pass", hr);
    }
    hr = stream->EndUpdatingAudioObjects();
    if (FAILED(hr)) {
        top->Release();
        stream->Release();
        return Fail(L"EndUpdatingAudioObjects-second-pass", hr);
    }
    hr = top->IsActive(&active);
    if (FAILED(hr) || active) {
        top->Release();
        stream->Release();
        return Fail(L"implicit-end-of-stream", FAILED(hr) ? hr : E_FAIL);
    }

    // The inactive static slot can be allocated again while the old COM object
    // remains referenced by the caller.
    ISpatialAudioObject* replacement = nullptr;
    hr = stream->ActivateSpatialAudioObject(AudioObjectType_TopFrontLeft, &replacement);
    if (FAILED(hr) || !replacement) {
        top->Release();
        stream->Release();
        return Fail(L"reactivate-static-role", hr);
    }

    hr = stream->Stop();
    if (FAILED(hr)) {
        replacement->Release();
        top->Release();
        stream->Release();
        return Fail(L"Stop", hr);
    }
    hr = stream->Reset();
    if (FAILED(hr)) {
        replacement->Release();
        top->Release();
        stream->Release();
        return Fail(L"Reset", hr);
    }

    replacement->Release();
    top->Release();
    stream->Release();

    std::wcout << L"SPATIAL_STATIC_STREAM_COM_OK 1\n";
    std::wcout << L"SPATIAL_STATIC_STREAM_DYNAMIC_CAPACITY 0\n";
    std::wcout << L"SPATIAL_STATIC_STREAM_QUANTUM_FRAMES 480\n";
    std::wcout << L"SPATIAL_STATIC_STREAM_IMPLICIT_EOS_OK 1\n";
    std::wcout << L"SPATIAL_STATIC_STREAM_LIFECYCLE_SMOKE_OK 1\n";
    return 0;
}
