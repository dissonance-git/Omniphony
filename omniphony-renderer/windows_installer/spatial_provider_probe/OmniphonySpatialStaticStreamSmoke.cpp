#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <mmreg.h>
#include <spatialaudioclient.h>

#include <algorithm>
#include <cmath>
#include <cstdint>
#include <iostream>
#include <memory>
#include <vector>

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

PROPVARIANT ActivationBlob(SpatialAudioObjectRenderStreamActivationParams& params) {
    PROPVARIANT activation{};
    activation.vt = VT_BLOB;
    activation.blob.cbSize = sizeof(params);
    activation.blob.pBlobData = reinterpret_cast<BYTE*>(&params);
    return activation;
}

bool Near(float actual, float expected) {
    return std::abs(actual - expected) <= 1.0e-6f;
}

class RecordingTransport final : public OmniphonySpatialStaticQuantumTransport {
public:
    RecordingTransport(std::size_t objectCount, std::size_t frames)
        : objectCount_(objectCount),
          frames_(frames),
          lastPlanar_(objectCount * frames, 0.0f) {}

    HRESULT Process(
        const float* inputPlanar,
        float* outputStereo,
        std::size_t frames) noexcept override {
        if (!inputPlanar || !outputStereo || frames != frames_) {
            return E_INVALIDARG;
        }
        std::copy(
            inputPlanar,
            inputPlanar + objectCount_ * frames_,
            lastPlanar_.begin());
        std::fill(outputStereo, outputStereo + frames_ * 2, 0.0f);
        ++calls_;
        return S_OK;
    }

    std::size_t Calls() const noexcept { return calls_; }

    float At(std::size_t object, std::size_t frame) const noexcept {
        return lastPlanar_[object * frames_ + frame];
    }

private:
    std::size_t objectCount_ = 0;
    std::size_t frames_ = 0;
    std::vector<float> lastPlanar_;
    std::size_t calls_ = 0;
};

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

    auto activation = ActivationBlob(params);

    void* rejected = reinterpret_cast<void*>(1);
    auto malformed = activation;
    --malformed.blob.cbSize;
    HRESULT hr = CreateOmniphonyStaticProbeStreamFromActivation(
        &malformed,
        __uuidof(ISpatialAudioObjectRenderStream),
        &rejected);
    if (hr != E_INVALIDARG || rejected != nullptr) {
        return Fail(L"activation-blob-size", hr);
    }

    rejected = reinterpret_cast<void*>(1);
    hr = CreateOmniphonyStaticProbeStreamFromActivation(
        &activation,
        __uuidof(ISpatialAudioClient),
        &rejected);
    if (hr != E_NOINTERFACE || rejected != nullptr) {
        return Fail(L"activation-iid", hr);
    }

    auto dynamicParams = params;
    dynamicParams.MaxDynamicObjectCount = 1;
    auto dynamicActivation = ActivationBlob(dynamicParams);
    rejected = reinterpret_cast<void*>(1);
    hr = CreateOmniphonyStaticProbeStreamFromActivation(
        &dynamicActivation,
        __uuidof(ISpatialAudioObjectRenderStream),
        &rejected);
    if (hr != AUDCLNT_E_UNSUPPORTED_FORMAT || rejected != nullptr) {
        return Fail(L"activation-dynamic-capacity", hr);
    }

    // Keep the VT_BLOB parser independently covered even though the detailed
    // lifecycle below uses the transport-enabled internal factory.
    void* inert = nullptr;
    hr = CreateOmniphonyStaticProbeStreamFromActivation(
        &activation,
        __uuidof(ISpatialAudioObjectRenderStream),
        &inert);
    if (FAILED(hr) || !inert) {
        return Fail(L"CreateOmniphonyStaticProbeStreamFromActivation", hr);
    }
    static_cast<ISpatialAudioObjectRenderStream*>(inert)->Release();

    std::shared_ptr<RecordingTransport> transport;
    try {
        transport = std::make_shared<RecordingTransport>(2, 480);
    }
    catch (...) {
        return Fail(L"RecordingTransport", E_OUTOFMEMORY);
    }

    ISpatialAudioObjectRenderStream* stream = nullptr;
    hr = CreateOmniphonyStaticProbeStreamWithTransport(params, transport, &stream);
    if (FAILED(hr) || !stream) {
        return Fail(L"CreateOmniphonyStaticProbeStreamWithTransport", hr);
    }

    UINT32 dynamicCount = 99;
    hr = stream->GetAvailableDynamicObjectCount(&dynamicCount);
    if (FAILED(hr) || dynamicCount != 0) {
        stream->Release();
        return Fail(L"GetAvailableDynamicObjectCount", FAILED(hr) ? hr : E_FAIL);
    }

    ISpatialAudioObject* front = nullptr;
    hr = stream->ActivateSpatialAudioObject(AudioObjectType_FrontLeft, &front);
    if (FAILED(hr) || !front) {
        stream->Release();
        return Fail(L"ActivateSpatialAudioObject(FL)", hr);
    }

    ISpatialAudioObject* top = nullptr;
    hr = stream->ActivateSpatialAudioObject(AudioObjectType_TopFrontLeft, &top);
    if (FAILED(hr) || !top) {
        front->Release();
        stream->Release();
        return Fail(L"ActivateSpatialAudioObject(TFL)", hr);
    }

    ISpatialAudioObject* duplicate = nullptr;
    hr = stream->ActivateSpatialAudioObject(AudioObjectType_TopFrontLeft, &duplicate);
    if (hr != SPTLAUDCLNT_E_OBJECT_ALREADY_ACTIVE || duplicate != nullptr) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"duplicate-static-role", hr);
    }

    ISpatialAudioObject* unavailable = nullptr;
    hr = stream->ActivateSpatialAudioObject(AudioObjectType_FrontRight, &unavailable);
    if (hr != SPTLAUDCLNT_E_STATIC_OBJECT_NOT_AVAILABLE || unavailable != nullptr) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"unrequested-static-role", hr);
    }

    ISpatialAudioObject* dynamic = nullptr;
    hr = stream->ActivateSpatialAudioObject(AudioObjectType_Dynamic, &dynamic);
    if (hr != SPTLAUDCLNT_E_NO_MORE_OBJECTS || dynamic != nullptr) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"dynamic-capacity-zero", hr);
    }

    BYTE* buffer = nullptr;
    UINT32 bufferBytes = 0;
    hr = top->GetBuffer(&buffer, &bufferBytes);
    if (hr != SPTLAUDCLNT_E_OUT_OF_ORDER || buffer != nullptr || bufferBytes != 0) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"GetBuffer-before-Begin", hr);
    }

    hr = stream->Start();
    if (FAILED(hr)) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"Start", hr);
    }
    const HRESULT secondStart = stream->Start();
    if (secondStart != SPTLAUDCLNT_E_STREAM_NOT_STOPPED) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"Start-twice", secondStart);
    }

    UINT32 available = 99;
    UINT32 frames = 0;
    hr = stream->BeginUpdatingAudioObjects(&available, &frames);
    if (FAILED(hr) || available != 0 || frames != 480) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"BeginUpdatingAudioObjects", FAILED(hr) ? hr : E_FAIL);
    }

    hr = top->SetPosition(-0.5f, 0.7f, -0.5f);
    if (hr != SPTLAUDCLNT_E_PROPERTY_NOT_SUPPORTED) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"static-SetPosition", hr);
    }
    if (FAILED(front->SetVolume(0.25f)) || FAILED(top->SetVolume(0.5f))) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"SetVolume", E_FAIL);
    }

    BYTE* frontBuffer = nullptr;
    UINT32 frontBytes = 0;
    hr = front->GetBuffer(&frontBuffer, &frontBytes);
    if (FAILED(hr) || !frontBuffer || frontBytes != 480u * sizeof(float)) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"GetBuffer(FL)", FAILED(hr) ? hr : E_FAIL);
    }
    std::fill(
        reinterpret_cast<float*>(frontBuffer),
        reinterpret_cast<float*>(frontBuffer) + frames,
        0.4f);

    BYTE* topBuffer = nullptr;
    UINT32 topBytes = 0;
    hr = top->GetBuffer(&topBuffer, &topBytes);
    if (FAILED(hr) || !topBuffer || topBytes != 480u * sizeof(float)) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"GetBuffer(TFL)", FAILED(hr) ? hr : E_FAIL);
    }
    std::fill(
        reinterpret_cast<float*>(topBuffer),
        reinterpret_cast<float*>(topBuffer) + frames,
        0.6f);

    // The final TFL quantum has only 120 valid frames. The fixed topology must
    // submit silence for the rest of that role's 480-frame slot.
    hr = top->SetEndOfStream(120);
    if (FAILED(hr)) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"SetEndOfStream(120)", hr);
    }

    hr = stream->EndUpdatingAudioObjects();
    if (FAILED(hr) || transport->Calls() != 1) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"EndUpdatingAudioObjects-first", FAILED(hr) ? hr : E_FAIL);
    }

    if (!Near(transport->At(0, 0), 0.1f) ||
        !Near(transport->At(0, 479), 0.1f) ||
        !Near(transport->At(1, 0), 0.3f) ||
        !Near(transport->At(1, 119), 0.3f) ||
        !Near(transport->At(1, 120), 0.0f) ||
        !Near(transport->At(1, 479), 0.0f)) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"planar-order-volume-partial-eos", E_FAIL);
    }

    BOOL active = TRUE;
    hr = top->IsActive(&active);
    if (FAILED(hr) || active) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"partial-eos-deactivates-object", FAILED(hr) ? hr : E_FAIL);
    }
    hr = front->IsActive(&active);
    if (FAILED(hr) || !active) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"front-survives-buffered-pass", FAILED(hr) ? hr : E_FAIL);
    }

    // A dead static role can be reactivated without changing descriptor order,
    // even while the caller still holds the old COM object reference.
    ISpatialAudioObject* replacement = nullptr;
    hr = stream->ActivateSpatialAudioObject(AudioObjectType_TopFrontLeft, &replacement);
    if (FAILED(hr) || !replacement) {
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"reactivate-static-role", hr);
    }

    hr = stream->BeginUpdatingAudioObjects(&available, &frames);
    if (FAILED(hr)) {
        replacement->Release();
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"BeginUpdatingAudioObjects-second", hr);
    }

    // Skip FL intentionally. That must implicitly end FL and submit silence in
    // its immutable slot. The replacement TFL object occupies the same slot as
    // the original TFL object rather than extending/reordering the topology.
    BYTE* replacementBuffer = nullptr;
    UINT32 replacementBytes = 0;
    hr = replacement->GetBuffer(&replacementBuffer, &replacementBytes);
    if (FAILED(hr) || !replacementBuffer || replacementBytes != 480u * sizeof(float)) {
        replacement->Release();
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"GetBuffer(replacement-TFL)", FAILED(hr) ? hr : E_FAIL);
    }
    std::fill(
        reinterpret_cast<float*>(replacementBuffer),
        reinterpret_cast<float*>(replacementBuffer) + frames,
        0.2f);

    hr = stream->EndUpdatingAudioObjects();
    if (FAILED(hr) || transport->Calls() != 2) {
        replacement->Release();
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"EndUpdatingAudioObjects-second", FAILED(hr) ? hr : E_FAIL);
    }

    if (!Near(transport->At(0, 0), 0.0f) ||
        !Near(transport->At(0, 479), 0.0f) ||
        !Near(transport->At(1, 0), 0.2f) ||
        !Near(transport->At(1, 479), 0.2f)) {
        replacement->Release();
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"immutable-role-slot-reuse", E_FAIL);
    }

    hr = front->IsActive(&active);
    if (FAILED(hr) || active) {
        replacement->Release();
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"implicit-end-of-stream", FAILED(hr) ? hr : E_FAIL);
    }

    hr = stream->Stop();
    if (FAILED(hr)) {
        replacement->Release();
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"Stop", hr);
    }
    hr = stream->Reset();
    if (FAILED(hr)) {
        replacement->Release();
        top->Release();
        front->Release();
        stream->Release();
        return Fail(L"Reset", hr);
    }

    replacement->Release();
    top->Release();
    front->Release();
    stream->Release();

    std::wcout << L"SPATIAL_STATIC_STREAM_COM_OK 1\n";
    std::wcout << L"SPATIAL_STATIC_STREAM_ACTIVATION_BLOB_OK 1\n";
    std::wcout << L"SPATIAL_STATIC_STREAM_DYNAMIC_CAPACITY 0\n";
    std::wcout << L"SPATIAL_STATIC_STREAM_QUANTUM_FRAMES 480\n";
    std::wcout << L"SPATIAL_STATIC_STREAM_TRANSPORT_CALLS 2\n";
    std::wcout << L"SPATIAL_STATIC_STREAM_PLANAR_ORDER_OK 1\n";
    std::wcout << L"SPATIAL_STATIC_STREAM_VOLUME_OK 1\n";
    std::wcout << L"SPATIAL_STATIC_STREAM_PARTIAL_EOS_OK 1\n";
    std::wcout << L"SPATIAL_STATIC_STREAM_IMPLICIT_EOS_OK 1\n";
    std::wcout << L"SPATIAL_STATIC_STREAM_SLOT_REUSE_OK 1\n";
    std::wcout << L"SPATIAL_STATIC_STREAM_LIFECYCLE_SMOKE_OK 1\n";
    return 0;
}
