# Windows Spatial Sound provider transport

This document defines how a future Omniphony Windows Spatial Sound provider can coexist with the already-accepted Omniphony stream SFX without double rendering.

It separates two Windows paths:

```text
ordinary application PCM
        ↓
normal shared-mode stream
        ↓
Omniphony stream SFX
        ↓
Omniphony binaural output
        ↓
physical stereo endpoint

spatial application
        ↓
Windows Spatial Audio objects
        ↓
Omniphony Spatial Sound provider
        ↓
Omniphony object renderer
        ↓
RAW stereo render stream
        ↓
physical stereo endpoint
```

## Governing rule

> **The object provider's final binaural stereo must not pass through Omniphony's normal stream SFX again.**

The existing SFX remains the Windows-wide ingress for ordinary stereo and conventional authored surround. The Spatial Sound provider is a parallel richer ingress for static and dynamic objects. Both converge on the same portable renderer semantics but only one Omniphony spatial render may occur for any one application stream.

## Why RAW is the preferred egress seam

Microsoft defines `AUDCLNT_STREAMOPTIONS_RAW` as a stream option that bypasses signal processing except endpoint-specific always-on processing in the APO, driver, and hardware.

Microsoft's APO architecture further states that render SFX is not used for RAW streams. On Windows 10, RAW mode does not load RAW SFX. Endpoint effects remain the always-on layer.

This matches Omniphony's accepted endpoint topology:

```text
normal streams
→ Omniphony SFX present

steady-state EFX
→ absent
```

Therefore a RAW stereo stream is the preferred candidate for provider egress:

```text
Omniphony object renderer
→ RAW shared stereo WASAPI
→ Omniphony SFX bypassed by Windows
→ no Omniphony EFX exists
→ physical headphone endpoint
```

This avoids:

- double HRTF/spatial rendering;
- a virtual cable;
- a second user-visible playback endpoint;
- disabling the normal SFX whenever a spatial application starts;
- routing already-binaural provider output back through stereo inference.

## Endpoint capability is a hard gate

RAW processing is not assumed universally.

Windows exposes the device property:

```text
System.Devices.AudioDevice.RawProcessingSupported
PKEY_Devices_AudioDevice_RawProcessingSupported
FMTID 8943B373-388C-4395-B557-BC6DBAFFAFDB
PID   2
```

The provider transport is accepted only after a real endpoint proves both:

```text
RAW_PROCESSING_SUPPORTED 1
RAW_STEREO_CLIENT_INITIALIZE_OK 1
```

The strongest proof is not the property alone. A probe should:

1. activate the selected render endpoint;
2. query the RAW-processing support property;
3. create `IAudioClient2`/`IAudioClient3`;
4. set `AudioClientProperties.Options |= AUDCLNT_STREAMOPTIONS_RAW`;
5. request the actual stereo endpoint format;
6. initialize a shared RAW render stream without starting audible playback;
7. record the exact HRESULT and resulting format.

Property support and successful stream creation remain separate evidence states.

## Fallback boundary

If RAW egress cannot be created on a target endpoint, do not silently route final binaural audio through the normal Omniphony SFX.

That would create:

```text
objects
→ Omniphony binaural
→ Omniphony stereo processing again
```

and violate the one-render law.

A non-RAW fallback must therefore be independently proven to bypass the normal SFX or explicitly coordinate a bypass before it is accepted. Exclusive-mode device capture is not the default fallback because it can monopolize the physical endpoint and interfere with ordinary system audio.

## Processing-mode invariant

The Windows provider and SFX are different host adapters, not different Omniphony products.

```text
ordinary stereo / PCM surround
→ SFX ingress
→ canonical source semantics

Windows spatial objects
→ provider ingress
→ canonical source semantics

both
→ same source-aware renderer laws
→ one binaural output
```

The provider must never feed object PCM into stereo inference, and the SFX must never reinterpret final provider binaural output as ordinary stereo.

## Current evidence state

```text
Microsoft RAW/SFX bypass contract     DOCUMENTED
Omniphony steady-state SFX/no-EFX     ACCEPTED BASELINE
RAW endpoint support                  MACHINE PROOF PENDING
RAW stereo initialization             MACHINE PROOF PENDING
provider object stream                IMPLEMENTATION PENDING
provider → RAW egress                 IMPLEMENTATION PENDING
```

## Primary references

- `AUDCLNT_STREAMOPTIONS`: https://learn.microsoft.com/windows/win32/api/audioclient/ne-audioclient-audclnt_streamoptions
- Audio Processing Object Architecture: https://learn.microsoft.com/windows-hardware/drivers/audio/audio-processing-object-architecture
- Audio Signal Processing Modes: https://learn.microsoft.com/windows-hardware/drivers/audio/audio-signal-processing-modes
- `IAudioClient2::SetClientProperties`: https://learn.microsoft.com/windows/win32/api/audioclient/nf-audioclient-iaudioclient2-setclientproperties
- Low Latency Audio / `IAudioClient3`: https://learn.microsoft.com/windows-hardware/drivers/audio/low-latency-audio
- `System.Devices.AudioDevice.RawProcessingSupported`: https://learn.microsoft.com/windows/win32/properties/props-system-devices-audiodevice-rawprocessingsupported
