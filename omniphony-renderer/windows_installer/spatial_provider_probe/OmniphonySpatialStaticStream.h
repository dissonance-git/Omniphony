#pragma once

#define WIN32_LEAN_AND_MEAN
#define NOMINMAX
#include <windows.h>
#include <spatialaudioclient.h>

// Internal factory used by the provider probe and its registry-free smoke test.
// This stage implements the Windows static-object lifecycle only. It does not
// open an endpoint or render audio yet.
HRESULT CreateOmniphonyStaticProbeStream(
    const SpatialAudioObjectRenderStreamActivationParams& params,
    ISpatialAudioObjectRenderStream** stream);
