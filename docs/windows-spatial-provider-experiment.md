# Windows Spatial Sound provider experiment

Status: **experimental observation path, not product source truth**

This experiment asks one narrow question:

> Can a third-party Windows Spatial Sound provider be selected by Windows so that a normal `ISpatialAudioClient` application delivers its static and dynamic objects to Omniphony before another binaural renderer destroys the object metadata?

A positive answer would give Omniphony the native system-level ingress it wants for Spatial Audio-aware games. A negative answer is equally useful because it prevents us from disguising post-render stereo as native object capture.

This note supplements `windows-spatial-input-contract.md`. The source-authority rules there remain governing.

## What is already proven

The current Omniphony Windows architecture has two distinct facts:

1. the endpoint/SFX path can receive conventional PCM beds and feed the portable renderer;
2. `OmniphonySpatialProbe.exe` can activate `ISpatialAudioClient` on the default endpoint and inspect the endpoint's native static-role mask, static positions, dynamic-object capacity, and object formats.

Neither fact proves that a normal APO can intercept another application's raw Spatial Audio objects.

The public `ISpatialAudioClient` API is an application-facing render API. Applications create static/dynamic objects, obtain buffers, update positions, and submit them to the active spatial renderer. The public APO contract separately describes processing audio buffers in the Windows audio engine. We therefore continue to treat raw cross-process object interception by an ordinary APO as **unproven**.

## New GitHub lead: provider registration surface

`ThreeDeeJay/MSSOAL` is a useful reverse-engineering lead because it attempts to implement an `ISpatialAudioClient`-compatible renderer and register it as a Windows Spatial Sound provider.

Its current code reports an observed provider-discovery registry surface:

```text
HKLM\SOFTWARE\Microsoft\Multimedia\Audio\Spatial\Encoder\{format-guid}
    (Default) = display name
    CLSID     = COM renderer CLSID
    IconPath  = optional icon
```

It also observes per-device state under:

```text
HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\MMDevices\SpatialAudioEndpoint
```

This is **not a public Microsoft provider-registration contract**. The MSSOAL repository itself is explicitly labeled an AI proof of concept that is not working yet. Its code is therefore hypothesis-generating evidence, not an implementation authority.

The useful part is the shape of the hypothesis: Windows may have a system-level spatial-format/provider seam distinct from an ordinary endpoint APO. If that seam really causes Windows to instantiate a third-party renderer for ordinary `ISpatialAudioClient` applications, it is exactly the class of ingress Omniphony needs.

## Read-only provider observation probe

Omniphony now builds:

```text
OmniphonySpatialProviderProbe.exe
```

The probe only reads the two observed registry areas above. It does not:

- create provider keys;
- register a COM server;
- alter the active spatial format;
- modify MMDevices state;
- change ACLs;
- take ownership;
- install a service;
- request SYSTEM or TrustedInstaller privileges.

Its output deliberately begins with:

```text
probe=omniphony_spatial_provider
mode=read_only_observation
source_truth=undocumented_registry_surface_not_public_api_contract
```

That label is important. Registry observation can tell us what this Windows installation is doing. It cannot turn an undocumented mechanism into a supported API contract.

## Experiment ladder

### P0: baseline capability

Run:

```powershell
.\OmniphonySpatialProbe.exe
```

Record:

- whether `ISpatialAudioClient` activates;
- native static object mask;
- all present static roles and positions;
- dynamic-object capacity;
- supported object format.

This proves endpoint Spatial Audio capability only.

### P1: provider inventory

Run:

```powershell
.\OmniphonySpatialProviderProbe.exe
```

Record the encoder/provider inventory and the shallow `SpatialAudioEndpoint` subtree while different installed spatial products are present.

High-value environments are:

```text
Windows Sonic only
Dolby Atmos for Headphones installed
DTS Headphone:X installed
Dolby + DTS installed together
```

We are looking for repeatable relationships among:

```text
format GUID
provider display name
COM CLSID
installed DLL/package
selected endpoint state
```

Do not write anything yet.

### P2: selection delta

Using only the normal Windows UI, switch the active spatial format among available providers and rerun the read-only probe after each change.

A useful result is a deterministic delta showing exactly which provider/endpoint state changes when the user selects Sonic, Dolby, or DTS.

A useless result is a collection of unrelated registry changes with no stable mapping to the selected renderer.

### P3: activation proof

Only after P1/P2 establish a stable provider mapping should Omniphony build a disposable experimental COM renderer/logger.

The required proof is not "the provider appears in Settings." The required proof is:

```text
normal unmodified Spatial Audio application
        ↓
Windows selects experimental provider
        ↓
Windows instantiates Omniphony COM renderer
        ↓
real ISpatialAudioClient stream arrives
        ↓
static-object activation is observed
and/or dynamic-object activation is observed
        ↓
per-update PCM + position calls are observed
```

If we cannot demonstrate that chain with a normal application, the provider hypothesis has failed.

### P4: native Omniphony ingress

A provider path earns product consideration only after it can convert live Windows objects into the existing portable scene contract without losing source truth:

```text
Windows static object
    -> canonical static role
    -> AUTHORED

Windows dynamic object
    -> stable object identity
    -> continuous x/y/z position
    -> AUTHORED

LFE
    -> semantically distinct non-directional low-frequency source
```

Coordinate mapping remains:

```text
Windows:   +X right, +Y up,      +Z behind
Omniphony: +X right, +Y forward, +Z up

[x, y, z] -> [x, -z, y]
```

### P5: final rendering proof

The provider experiment is not complete until a real game/application scene reaches the same Omniphony renderer used by conventional PCM and survives the following checks:

```text
front / rear separation
height / lower separation
moving-object continuity
stable object identity
source extent
LFE semantics
binaural output exactly 2 channels
no double virtualization
bypass determinism
no new virtual cable / second endpoint
```

## Hard falsifiers

Stop treating this route as a candidate native ingress if any of the following survives careful reproduction:

1. Windows will enumerate a third-party provider but never instantiate it for a normal `ISpatialAudioClient` application.
2. Provider activation requires a private/licensed interface unavailable to an ordinary third-party implementation.
3. Windows gives the provider only an already-rendered bed or binaural stream rather than the source objects.
4. Required registration depends on unsupported ownership/ACL surgery that is too brittle for a normal audio product.
5. The route works only by injecting or hooking the game process.
6. The route requires reviving the virtual-cable architecture Omniphony is replacing.

A failed provider experiment does not invalidate Omniphony. It means the safe system-wide floor remains authored PCM via the pre-mix SFX/endpoint path, while lossless objects require a cooperative application/plugin seam.

## Renderer consequences from SciSpace

The literature pass sharpens what we should do **if** P3/P4 succeed.

### Preserve object identity and continuous motion

Queiroz and de Sousa, *Efficient Binaural Rendering of Moving Sound Sources Using HRTF Interpolation* (2011, DOI `10.1080/09298215.2011.594894`), shows that interpolation can support real-time continuously moving binaural sources while preserving spatial impression. Omniphony should therefore interpolate HRTF/transfer state across object motion rather than snap dynamic objects to the nearest static 8.1.4.4 anchor.

### Treat externalization as its own problem

Landschoot and Jot, *Binaural externalization processing method for object-based audio rendering* (2023, DOI `10.1121/10.0018389`), directly addresses the tendency of frontal binaural objects to collapse toward or inside the head. Their review also reinforces that HRTF selection alone is not the whole externalization problem. Omniphony's distance/room/externalization layer should remain distinct from basic directional HRTF selection.

### Preserve source extent explicitly

Anemüller, Thiergart, and Habets, *Binaural Rendering of Heterogeneous Sound Sources with Extent* (ICASSP 2024, DOI `10.1109/ICASSP48485.2024.10448024`), treats source extent as a spatial property that should be preserved rather than collapsing every source to a point. This supports Omniphony's explicit extent metadata and the existing rule that extent redistributes spatial energy instead of becoming a hidden gain control.

### Use scene fields for diffuse structure, not as an excuse to destroy objects

Recent higher-order Ambisonic work supports head-tracked full-3D field rendering and shows diminishing perceptual returns at very high spatial orders in tested conditions. That makes HOA/field machinery attractive for diffuse ambience, late room energy, or scene support. It does **not** justify replacing a precise Windows dynamic object with a quantized diffuse field before the final renderer.

The resulting hierarchy is:

```text
precise authored object       -> keep precise object
static authored Windows role  -> keep authored role
spatially extended source     -> keep explicit extent
ambient/diffuse energy        -> field/HOA representation may help
stereo evidence               -> bounded inference only
```

## Current decision

The repository should now pursue two tracks in parallel without conflating them:

```text
TRACK A, proven compatibility
Windows PCM 2.0 / 5.1 / 7.1 / richer WAVEFORMATEXTENSIBLE when supplied
        -> Omniphony SFX/APO
        -> portable scene
        -> binaural headphones

TRACK B, falsifiable native-object experiment
Windows Spatial Sound provider seam
        -> only if P3 proves real object delivery
        -> static + dynamic object ingress
        -> same portable scene
        -> same binaural renderer
```

Track B is allowed to fail cleanly. Track A must remain stable throughout the experiment.

## References

- Microsoft Spatial Sound overview: <https://learn.microsoft.com/windows/win32/coreaudio/spatial-sound>
- Microsoft spatial-object rendering: <https://learn.microsoft.com/windows/win32/coreaudio/render-spatial-sound-using-spatial-audio-objects>
- Microsoft APO architecture: <https://learn.microsoft.com/windows-hardware/drivers/audio/audio-processing-object-architecture>
- Microsoft Windows Driver Samples, SysVAD APO: <https://github.com/microsoft/Windows-driver-samples/tree/main/audio/sysvad/APO>
- Microsoft SpatialSound sample: <https://github.com/microsoft/Windows-universal-samples/tree/main/Samples/SpatialSound>
- MSSOAL experimental provider implementation: <https://github.com/ThreeDeeJay/MSSOAL>
- Landschoot & Jot 2023: DOI `10.1121/10.0018389`
- Queiroz & de Sousa 2011: DOI `10.1080/09298215.2011.594894`
- Anemüller, Thiergart & Habets 2024: DOI `10.1109/ICASSP48485.2024.10448024`
