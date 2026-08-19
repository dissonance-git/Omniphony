#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#include <spatialaudioclient.h>

#include <cstddef>
#include <memory>

// Internal transport boundary for one completed immutable-topology static
// Spatial Audio quantum. Implementations are created before stream processing
// begins; Process must not discover devices, open files, or allocate a renderer.
class OmniphonySpatialStaticQuantumTransport {
public:
    virtual ~OmniphonySpatialStaticQuantumTransport() = default;

    virtual HRESULT Process(
        const float* inputPlanar,
        float* outputStereo,
        std::size_t frames) noexcept = 0;
};

// Internal factory used by the provider probe and its registry-free smoke test.
// With no transport the object lifecycle is exercised without accepting a
// public Windows Spatial Audio stream or claiming audible output.
HRESULT CreateOmniphonyStaticProbeStream(
    const SpatialAudioObjectRenderStreamActivationParams& params,
    ISpatialAudioObjectRenderStream** stream);

// Transport-enabled variant used only behind the closed provider gate. Static
// descriptor order is derived once from params.StaticObjectTypeMask and remains
// immutable for the stream lifetime.
HRESULT CreateOmniphonyStaticProbeStreamWithTransport(
    const SpatialAudioObjectRenderStreamActivationParams& params,
    std::shared_ptr<OmniphonySpatialStaticQuantumTransport> transport,
    ISpatialAudioObjectRenderStream** stream);

// Parses the exact VT_BLOB activation shape documented for
// ISpatialAudioClient::ActivateSpatialAudioStream and creates the same inert
// static-object stream used by the registry-free lifecycle smoke test.
//
// This helper deliberately stays internal while the public provider gate is
// closed. The provider must not accept application audio until COM quanta,
// Current, and final endpoint output form one proven path.
HRESULT CreateOmniphonyStaticProbeStreamFromActivation(
    const PROPVARIANT* activationParams,
    REFIID riid,
    void** stream);
