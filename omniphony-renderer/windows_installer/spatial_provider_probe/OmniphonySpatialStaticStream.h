#pragma once

#ifndef WIN32_LEAN_AND_MEAN
#define WIN32_LEAN_AND_MEAN
#endif
#ifndef NOMINMAX
#define NOMINMAX
#endif
#include <windows.h>
#include <spatialaudioclient.h>

// Internal factory used by the provider probe and its registry-free smoke test.
// This stage implements the Windows static-object lifecycle only. It does not
// open an endpoint or render audio yet.
HRESULT CreateOmniphonyStaticProbeStream(
    const SpatialAudioObjectRenderStreamActivationParams& params,
    ISpatialAudioObjectRenderStream** stream);

// Parses the exact VT_BLOB activation shape documented for
// ISpatialAudioClient::ActivateSpatialAudioStream and creates the same inert
// static-object stream used by the registry-free lifecycle smoke test.
//
// This helper deliberately stays internal until the provider has a downstream
// transport into Omniphony. The public COM provider must continue to report the
// stream unavailable rather than accept application audio and drop it.
HRESULT CreateOmniphonyStaticProbeStreamFromActivation(
    const PROPVARIANT* activationParams,
    REFIID riid,
    void** stream);
