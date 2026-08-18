# Omniphony

Omniphony is an experimental, always-on spatial processor for headphones, built from the upstream `mgth/Omniphony` renderer and extended around one product rule:

> **Make the headphones disappear without making the recording disappear with them.**

The finished recording remains the musical authority. Omniphony may enlarge width, depth, height, distance, source extent and envelopment, but it must not need to sacrifice clarity, impact, center stability, timbre or rhythmic precision to do it.

Windows is the first product host. The renderer, scene contract and DSP core remain portable.

## Windows 0.1 product

The normal Windows product is deliberately simple:

```text
OmniphonySetup.exe
        ↓
one UAC elevation
        ↓
attach the unsigned user-mode EFX APO to the current render endpoint
        ↓
restart Windows Audio
        ↓
tray icon appears
        ↓
Current renders system audio headlessly
```

Normal use has:

- one installer EXE;
- no virtual cable;
- no loopback host;
- no console;
- no taskbar window;
- no resident `Omniphony.exe` audio host;
- one small notification-area icon for preferences;
- rendering that continues even if the tray icon is closed.

The 0.1 installer uses Windows' unprotected AudioDG compatibility mode for the unsigned APO, records the previous machine value, and restores that state during rollback/uninstall. This is the supported quick-install product path, not a temporary bring-up mode.

The signed/componentized DriverStore work under `windows_installer/endpoint_apo/production/` remains an **optional future deployment experiment**. It is not required to install or use Omniphony 0.1.

## Current architecture

The normal stereo Current path is:

```text
finished stereo master
        │
        ├──────────────────────────────→ protected direct master
        │
        ├→ coherent music foundation
        │      └→ bounded pressure / punch / body support
        │
        └→ analysis-only stereo evidence
                         ↓
             CANONICAL 8.1.4.4 SCENE
             17 semantic lanes
             L R C LFE Ls Rs Lb Rb Cb
             Tfl Tfr Tbl Tbr Bfl Bfr Bbl Bbr
                         ↓
             CURRENT 22-DIRECTION SHELL
             System-H-derived full-sphere lattice
                         ↓
                CASCADED BINAURAL
              measured HRTF + ITD
              distance / air / room
                         ↓
       protected master + foundation + support
                         ↓
                peak-safe stereo
                         ↓
                    headphones
```

The **17-lane 8.1.4.4 scene is the foundational product vocabulary**. The **22-direction shell is an internal render lattice above it**. It does not replace the canonical scene.

For stereo-derived Current, these lanes are currently earned by evidence:

```text
L R Ls Rs Lb Rb Tfl Tfr Tbl Tbr
```

These canonical lanes remain EMPTY:

```text
C LFE Cb Bfl Bfr Bbl Bbr
```

For source-native game-music objects, Omniphony may create stable `DERIVED` immersive placement while preserving authored routing, timing and identity. That is a modern immersive remix decision, not a historical-authorship claim.

Source-aware game music uses the **same embedded 22-direction System-H-derived shell and cascaded binaural topology in both presentation modes**. `NativeRouting` closes creative rear/height/depth/extent while preserving source-native laterality and identity. `FullSphere` opens DERIVED immersive placement on that same renderer. Recovered objects carry a 3-D extent `[width, depth, height]`; in FullSphere that extent becomes size-aware constant-power VBAP spread over the shell before binaural rendering.

Keeping one physical renderer underneath both modes is intentional: switching NativeRouting ↔ FullSphere changes presentation policy rather than secretly changing the binaural algorithm, while the protected historical stereo remains the untouched reference beneath both.

For SNES/SPC, the final post-EVOL S-DSP echo is treated as its own historical shared stereo field. Its linked L/R identity stays intact, while Omniphony may independently control that field's rear bias, elevation, radial depth, strength and shell extent. Historical echo and Omniphony's optional externalization room remain separate layers.

## What is implemented now

| Layer | Current state |
| --- | --- |
| Canonical static scene | **Implemented:** 17-lane 8.1.4.4 vocabulary |
| Stereo evidence mapping | **Implemented:** bounded stereo-derived support into earned lanes only |
| Source-aware game-music sphere | **Implemented:** stable DERIVED width/depth/height/extent constrained by source evidence and rendered through the 22-direction shell |
| Source-aware mode A/B | **Implemented:** NativeRouting and FullSphere share one extent-capable shell/cascade topology and differ by presentation policy |
| Source-aware shared wet layer | **Implemented:** historical shared fields such as S-DSP echo remain separate and receive independent strength/geometry/extent treatment |
| Current spatial shell | **Implemented:** 22-direction System-H-derived full-sphere lattice |
| Headphone renderer | **Implemented:** cascaded binaural with measured HRTF / ITD path |
| Directional early field | **Implemented:** bounded directional reflection support |
| Windows realtime ABI | **Implemented:** `omniphony_realtime.dll` |
| Windows endpoint APO | **Implemented:** stereo float32 Current path |
| Windows 0.1 quick installer | **Implemented:** unsigned endpoint APO + rollback + tray-only UI |
| Signed DriverStore deployment | **Optional future experiment:** repository tooling retained, not a product prerequisite |
| Native authored multichannel PCM ingress | **Implemented in renderer/stream-APO code:** `WAVEFORMATEXTENSIBLE` speaker beds preserve authored positions; Windows SFX format negotiation and installer promotion are still being hardened |
| Raw Windows Spatial Audio object ingress | **Required next host path:** preserve available static 8.1.4.4 objects and dynamic 3-D objects before Windows headphone rendering |

The native-bed path and the raw Spatial Audio object path are deliberately distinct. A 7.1.4 `WAVEFORMATEXTENSIBLE` fixture proves authored PCM-bed handling; it does not prove that Omniphony has received another application's Windows Spatial Audio object stream.

## Windows audio topology

```text
conventional PCM apps
        ↓
Windows Audio Engine
        ↓
Omniphony stream / endpoint APO
        ↓
      ┌──────────────────────────────────┐
      │                                  │
      │       omniphony_realtime.dll     │
      │  source scene → shell → binaural │
      │                                  │
      └──────────────────────────────────┘
        ↑
Omniphony spatial-object ingress
        ↑
Windows Spatial Audio static + dynamic objects
        ↑
spatial-aware applications / games
        ↓
physical endpoint driver
        ↓
DAC / headphones
```

The conventional APO path is the universal fallback. The Windows Spatial Audio path is a required richer ingress when the operating system exposes source-authored static or dynamic objects before headphone rendering. The exact supported system boundary for receiving another application's raw object stream must be proven rather than inferred; opening an `ISpatialAudioClient` by itself does not establish interception.

The tray is preference-only. It does not host the audio engine. The old process-loopback and virtual-device routes are migration history, not the product architecture.

See:

- [`docs/omniphony-for-windows.md`](docs/omniphony-for-windows.md)
- [`omniphony-renderer/windows_installer/endpoint_apo/README.md`](omniphony-renderer/windows_installer/endpoint_apo/README.md)
- [`omniphony-renderer/windows_installer/endpoint_apo/production/README.md`](omniphony-renderer/windows_installer/endpoint_apo/production/README.md) for the optional signed-package experiment

## Fidelity laws

> **Dimension may not be purchased by damaging the music.**

Turning Omniphony off may collapse width, depth, height, radial distance, source extent, ambient continuity and envelopment.

Turning Omniphony off must **not** restore clarity, kick impact, bass pressure, transient snap, tonal identity, center stability, microdetail, dynamics or comfortable spectral balance.

Shortest form:

> **OFF may collapse the world. It may not bring the rhythm section back to life.**

The protected stereo master never passes through the virtual room. FFT/STFT analysis may inform support decisions, but the master is not STFT-resynthesized.

## Source authority

The richer the source truth, the less Omniphony should infer about what the source actually was. That does not prevent deliberate `DERIVED` immersive presentation when the user asks for it.

```text
stereo
→ preserve the master + infer bounded presentation support

5.1 / 7.1 / height PCM
→ preserve authored directional channels when Windows exposes them

Windows Spatial Audio static objects
→ preserve supplied fixed spatial roles, including lower-hemisphere roles when present

Windows Spatial Audio dynamic objects
→ preserve object identity, audio and supplied 3-D position over time

height / objects / HOA from other hosts
→ preserve supplied geometry or field truth when available

source-native game music
→ preserve recovered voices / channels / shared wet fields
→ preserve authored route, timing and identity
→ create stable immersive placement for otherwise unauthored dimensions
→ label those choices DERIVED

already-binaural material
→ avoid destructive double HRTF virtualization
```

`AUTHORED`, `DERIVED` and `EMPTY` are provenance states, not cosmetic labels.

## Realtime architecture

The endpoint effect loads `omniphony_realtime.dll` through a narrow ABI. The AudioDG callback does not run the allocating renderer graph directly. A bounded, preallocated callback-facing path exchanges PCM with a dedicated Current worker.

The runtime retains:

- preallocated callback-facing rings;
- dedicated Current worker processing;
- time-aligned dry fallback;
- non-finite sanitization;
- linked peak safety;
- explicit create/destroy lifecycle tests;
- manifest/import/ABI checks in CI.

The callback must not perform filesystem I/O, network activity, device discovery or research-time analysis. A future Windows spatial-object callback must obey the same law and hand bounded object audio/state to the worker rather than moving the allocating renderer onto the OS realtime thread.

## Validation

Engineering gates include canonical scene order, EMPTY-lane preservation, source identity stability, deterministic source-aware placement, shared 22-direction source topology, runtime NativeRouting ↔ FullSphere round-trip, constant-power shell spread across extent, source-extent audibility, shared-wet extent independence, HRTF/ITD checks, transient and bass preservation, non-finite/peak safety, Windows APO ABI/manifest/import checks, native-bed channel-mask identity, installer rollback behavior and real-endpoint WASAPI probes.

Raw Windows Spatial Audio promotion additionally requires proof that Omniphony receives a real spatial-aware application's source representation before Windows headphone rendering, preserves every received static role, preserves dynamic-object identity/PCM/XYZ motion, and cleanly restores the ordinary Windows spatial path when Omniphony is disabled.

Human listening remains the final gate for externalization, front/back discrimination, elevation, source body, envelopment, radial depth, center solidity, room naturalness, fatigue, groove and bass integrity.

## Repository map

```text
omniphony-renderer/renderer/
  portable DSP, inference, HRTF and scene machinery

omniphony-renderer/orender_engine/
  headless Current construction and rendering boundary

omniphony-renderer/realtime_ffi/
  narrow realtime ABI used by Windows host paths, including authored native beds

omniphony-renderer/windows_installer/endpoint_apo/
  Windows endpoint/stream APOs, installer, tray and diagnostics

omniphony-renderer/windows_installer/endpoint_apo/production/
  optional signed/componentized DriverStore deployment research

layouts/
  reference and renderer geometry, including the Current 22-direction shell

docs/
  source authority, scene, Windows, listening and validation contracts
```

## Build and focused tests

From `omniphony-renderer/`:

```sh
cargo test -p renderer
cargo test -p renderer --test source_shell_spread_energy
cargo test -p orender_engine --lib --tests
cargo test -p orender_engine --test source_shared_wet_extent
cargo test -p source_ffi --lib --tests
cargo test -p source_ffi --test runtime_spatial_mode
cargo test -p realtime_ffi
```

`.github/workflows/source-aware-spatial-validation.yml` is the focused CI gate for the source-native renderer and source ABI. It runs the 22-direction constant-power extent test, the `orender_engine` source-path tests and the `source_ffi` ABI/mode tests independently of the broader renderer perf gate.

The Windows installer workflow builds and validates the full Current realtime path before producing `OmniphonySetup.exe`.

## Relationship to libaural, VGM Tooling and Helix

These projects may exchange research and evidence, but they remain separate runtime systems.

```text
HELIX
research / provenance / method
        ↓
libaural
experimental machine hearing
        ├───────────────┐
        ↓               ↓
VGM Tooling         Omniphony
source truth        presentation / listening testbed
```

No project becomes a runtime dependency merely because it produced a useful experiment.

## Definition of success

> **A finished recording keeps its identity, weight, dynamics and clarity while gaining a stable external world with front distance, rear depth, extreme width, convincing height, continuous motion and enough radial scale that ordinary headphone playback feels dimensionally collapsed by comparison.**

For source-native game music, success adds one more test: the enlarged result should feel less like an effect placed on an old stereo recording and more like discovering the immersive master that the original hardware never had enough dimensions to carry.

For Windows spatial-aware games, success adds the source-authority test: if the game supplies real spatial objects, Omniphony should hear those objects as objects rather than first flattening them into a headphone mix and attempting to infer the world again.
