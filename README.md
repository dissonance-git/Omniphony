# Omniphony

Omniphony is a free and open-source spatial audio renderer for headphones.

Its goal is to occupy the same broad class of system audio role as proprietary headphone spatial renderers such as Dolby Atmos for Headphones, DTS Headphone:X, Windows Sonic, Sony 360-style rendering systems, and Waves Nx, while keeping the renderer, scene model, source-authority rules, DSP, validation, and research inspectable.

> **One open spatial renderer that enhances stereo, preserves native surround, accepts true spatial scenes when available, and performs the final headphone render itself.**

Windows is the first product host. The renderer, scene contract, and DSP core are designed to remain portable.

## Product law

Omniphony is not a stereo enhancer plus a separate surround renderer. It is one spatial renderer whose behavior becomes more source-authoritative as richer input becomes available.

```text
stereo
→ preserve the finished master
→ infer only spatial structure that the source does not explicitly contain
→ enhance through Omniphony

5.1 / 7.1 / height PCM
→ preserve authored channels and positions
→ infer less because more of the scene is already known
→ enhance through the same renderer

8.1.4.4 static spatial scene
→ preserve supplied fixed spatial roles
→ avoid reconstructing geometry already supplied by the source
→ enhance through the same renderer

8.1.4.4 + dynamic XYZ objects
→ preserve fixed scene structure and continuous object motion
→ give supplied geometry maximum authority
→ enhance through the same renderer
```

The richer the source truth, the less Omniphony invents.

Stereo is the hardest case because only two channels are available. Native surround should be a stronger input to the same enhancement system because authored direction replaces guesswork. Static and dynamic spatial objects are richer again.

Every path ends in one binaural render to an ordinary stereo headphone endpoint.

## Windows-wide architecture

The intended product experience is simple:

```text
Windows audio
     ↓
Omniphony
     ↓
headphones
```

Internally, Omniphony preserves the richest trustworthy representation supplied by the source:

```text
ordinary stereo ───────────────┐
5.1 / 7.1 PCM ─────────────────┤
height PCM ────────────────────┤
static spatial objects ────────┤
dynamic XYZ objects ───────────┤
                               ↓
                     canonical source scene
                               ↓
                       Omniphony renderer
                               ↓
                         binaural stereo
                               ↓
                           headphones
```

The Windows product is headless. Audio rendering does not depend on a resident foreground application, virtual cable, or loopback host. A small tray component may expose preferences, but it does not carry the audio stream.

## Current Windows baseline

The current Windows host accepts stereo and authored multichannel shared-mode PCM through a format-changing stream SFX while the physical headphone endpoint remains stereo.

The native surround baseline is:

```text
48 kHz / float32 / authored 7.1 client stream
        ↓
Omniphony stream SFX
        ↓
AUTHORED FL FR C LFE SL SR BL BR
        ↓
Omniphony source scene
        ↓
Current spatial renderer
        ↓
48 kHz / 32-bit / stereo endpoint
        ↓
headphones
```

The physical endpoint remaining stereo is intentional. Richer source geometry exists upstream of the final endpoint mix and is reduced to two channels by Omniphony.

A stereo endpoint EFX remains available as a transactional rollback and recovery floor. After successful native-surround promotion, the stream SFX is the steady-state path and the temporary stereo EFX is removed so the signal is rendered once.

## Canonical spatial scene

Omniphony uses a **17-position 8.1.4.4 static scene** as its canonical Windows spatial vocabulary:

```text
horizontal
FL FR C LFE SL SR BL BR BC

upper
TFL TFR TBL TBR

lower
BFL BFR BBL BBR
```

This is a coordinate vocabulary, not a claim that every source contains seventeen authored channels.

Every static lane has an authority state:

```text
AUTHORED  source or host supplied this signal / position
DERIVED   Omniphony inferred bounded support
EMPTY     no trustworthy signal is assigned
```

For a conventional authored 7.1 stream:

```text
FL FR C LFE SL SR BL BR = AUTHORED
BC TFL TFR TBL TBR BFL BFR BBL BBR = EMPTY unless separately earned
```

For stereo, Omniphony may derive bounded spatial support while preserving the finished master as the musical authority.

Dynamic spatial objects sit beside the static scene rather than being forced into it:

```text
8.1.4.4 static scene
        +
continuous dynamic XYZ objects
        ↓
one Omniphony source scene
```

When exact object coordinates are supplied, they outrank inferred geometry and should remain continuous as far into rendering as possible.

## Renderer geometry

The canonical 8.1.4.4 scene and Omniphony's internal rendering geometry are deliberately different concepts.

```text
source truth
        ↓
8.1.4.4-capable semantic scene
+ continuous objects where supplied
        ↓
source authority / provenance
        ↓
22-direction Current support shell
        ↓
HRTF / ITD / distance / room
        ↓
binaural stereo
```

The **8.1.4.4 scene is the semantic skeleton**. The **22-direction shell is internal rendering geometry**. It does not represent twenty-two authored Windows input channels.

## Stereo enhancement

Stereo remains a first-class source type rather than a compatibility afterthought.

The finished stereo master remains protected. Omniphony may analyze it to infer bounded width, depth, height, ambience, source extent, and externalization support, but spatial dimension may not be purchased by damaging clarity, impact, center stability, timbre, dynamics, or rhythmic precision.

> **OFF may collapse the world. It may not bring the rhythm section back to life.**

The stereo path therefore combines protected source material with bounded evidence-derived spatial support rather than treating a two-channel master as if it were an authored object scene.

## Native surround and height

When Windows or another host supplies authored multichannel PCM, Omniphony maps the supplied channel mask directly into authored source positions and bypasses stereo spatial inference for those channels.

```text
5.1 / 7.1 / height bed
        ↓
authored channel identity
        ↓
canonical scene
        ↓
Omniphony spatial enhancement
        ↓
one binaural render
```

LFE remains semantically distinct from directional HRTF sources. Missing channels remain empty rather than being silently promoted to authored content.

The stream APO/native-bed path currently supports and regression-tests stereo, authored 7.1, and authored 7.1.4 processing.

## Spatial objects

The ideal Windows ingress is the richest spatial representation the operating system can expose before another headphone renderer collapses it to stereo:

```text
8.1.4.4 static spatial roles
        +
dynamic XYZ objects
        ↓
Omniphony source scene
        ↓
Omniphony spatial enhancement
        ↓
Omniphony binaural render
        ↓
headphones
```

Raw Windows Spatial Audio object ingress is not yet claimed as complete. A supported system boundary must first be demonstrated for receiving another application's static and dynamic spatial representation before Windows Sonic, Dolby, DTS, or another renderer performs the final headphone render.

Omniphony does not treat already-binaural stereo as raw objects, and it does not reconstruct object metadata from a final binaural mix and call that native spatial ingress.

## What is implemented

| Layer | State |
| --- | --- |
| Canonical static scene | **Implemented:** 17-position 8.1.4.4 vocabulary |
| Source authority | **Implemented:** AUTHORED / DERIVED / EMPTY semantics |
| Stereo evidence mapping | **Implemented:** bounded stereo-derived spatial support |
| Current support shell | **Implemented:** 22-direction full-sphere rendering lattice |
| Binaural renderer | **Implemented:** measured HRTF / ITD path with distance and room support |
| Windows realtime runtime | **Implemented:** `omniphony_realtime.dll` |
| Windows stereo ingress | **Implemented:** protected stereo Current path |
| Windows authored 7.1 ingress | **Implemented and physically verified:** shared 7.1 client → stream SFX → stereo endpoint |
| Authored 7.1.4 processing | **Implemented and regression-tested** in the stream APO/native-bed path |
| Endpoint continuity / rollback | **Implemented:** persistent endpoint identity, recovery, and stereo rollback floor |
| Headless Windows installer | **Implemented:** one installer, no virtual cable or resident audio host |
| Raw Windows Spatial Audio object ingress | **In progress:** static 8.1.4.4 + dynamic XYZ before third-party headphone rendering |
| Signed DriverStore deployment | **Optional future deployment route** |

## Source authority

The central rule is simple:

> **Preserve the richest source representation available and invent only what is missing.**

```text
stereo
→ preserve master + infer bounded spatial support

5.1 / 7.1 / height PCM
→ preserve authored channels and supplied geometry

static spatial objects
→ preserve fixed spatial roles and identity

dynamic spatial objects
→ preserve object identity, PCM, and continuous 3-D position

already-binaural material
→ avoid destructive double HRTF virtualization
```

`AUTHORED`, `DERIVED`, and `EMPTY` are provenance states, not cosmetic labels.

## Realtime architecture

The Windows APOs load `omniphony_realtime.dll` through a narrow ABI. Windows realtime callbacks do not run the allocating renderer graph directly. A bounded, preallocated callback-facing path exchanges PCM with a dedicated Current worker.

The runtime includes:

- preallocated callback-facing rings;
- dedicated Current worker processing;
- time-aligned dry fallback;
- non-finite sanitization;
- linked peak safety;
- explicit create/destroy lifecycle tests;
- manifest, import, and ABI checks in CI.

Realtime callbacks must not perform filesystem I/O, network activity, device discovery, or research-time analysis. Any future spatial-object host must obey the same rule.

## Validation

Engineering gates cover:

- canonical scene order and authority preservation;
- authored channel-mask identity;
- source identity stability;
- deterministic spatial placement;
- constant-power shell spread;
- HRTF / ITD behavior;
- transient and bass preservation;
- non-finite and peak safety;
- realtime ABI and lifecycle behavior;
- Windows APO registration and manifest contracts;
- endpoint continuity and rollback;
- shared-client multichannel initialization;
- exact two-channel physical output.

Human listening remains the final gate for externalization, front/back discrimination, elevation, source body, envelopment, radial depth, center solidity, room naturalness, fatigue, groove, and bass integrity.

## Windows deployment

The Windows installer configures the selected physical render endpoint directly.

Normal use has:

- one installer executable;
- one UAC elevation;
- no virtual cable;
- no loopback host;
- no console;
- no resident audio-host application;
- a preference-only tray icon;
- rendering that continues if the tray UI is closed.

The current unsigned user-mode APO deployment uses Windows' unprotected AudioDG compatibility mode and records previous machine state for rollback and uninstall.

A componentized signed DriverStore route remains available as a separate deployment research track without changing the renderer architecture.

## Repository map

```text
omniphony-renderer/renderer/
  portable DSP, HRTF, inference, scene, and source-rendering machinery

omniphony-renderer/orender_engine/
  headless renderer construction and execution boundary

omniphony-renderer/realtime_ffi/
  realtime ABI used by Windows host paths

omniphony-renderer/windows_installer/endpoint_apo/
  Windows stream / endpoint APOs, installer, tray, and diagnostics

layouts/
  canonical and internal rendering geometry

docs/
  source authority, Windows ingress, spatial scene, and validation contracts
```

## Build and tests

From `omniphony-renderer/`:

```sh
cargo test -p renderer
cargo test -p orender_engine --lib --tests
cargo test -p source_ffi --lib --tests
cargo test -p realtime_ffi
```

Focused CI additionally validates source-aware spatial behavior, the realtime Windows path, APO lifecycle contracts, endpoint tooling, and installer packaging.

## Definition of success

> **A finished source keeps its identity, weight, dynamics, clarity, and authored spatial truth while gaining a stable external world with convincing width, depth, height, distance, motion, source extent, and envelopment.**

For stereo, Omniphony should create a richer spatial presentation without pretending inferred geometry was authored.

For native surround, height, and object sources, Omniphony should become progressively less inferential and more authoritative, because the source has already supplied more of the world.

The long-term target is a transparent, inspectable, open spatial renderer that can sit at the Windows audio boundary, receive whatever spatial truth an application can provide, and perform the final headphone render itself.
