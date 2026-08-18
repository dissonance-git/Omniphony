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
| Native authored 5.1 / 7.1 ingress through the APO | **Not implemented yet** |
| Raw Windows Spatial Audio object ingress | **Research frontier** |

A 7.1.4 fixture or layout is therefore a useful regression input, not the current Windows ingress format.

## Windows audio topology

```text
applications / games / browsers / players
                 ↓
         Windows Audio Engine
                 ↓
        OMNIPHONY EFX APO
                 ↓
      omniphony_realtime.dll
                 ↓
 Current scene → shell → binaural DSP
                 ↓
       physical endpoint driver
                 ↓
          DAC / headphones
```

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

5.1 / 7.1 PCM
→ preserve authored directional channels when a future host exposes them

height / objects / HOA
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

The callback must not perform filesystem I/O, network activity, device discovery or research-time analysis.

## Validation

Engineering gates include canonical scene order, EMPTY-lane preservation, source identity stability, deterministic source-aware placement, shared 22-direction source topology, runtime NativeRouting ↔ FullSphere round-trip, constant-power shell spread across extent, source-extent audibility, shared-wet extent independence, HRTF/ITD checks, transient and bass preservation, non-finite/peak safety, Windows APO ABI/manifest/import checks, installer rollback behavior and real-endpoint WASAPI probes.

Human listening remains the final gate for externalization, front/back discrimination, elevation, source body, envelopment, radial depth, center solidity, room naturalness, fatigue, groove and bass integrity.

## Repository map

```text
omniphony-renderer/renderer/
  portable DSP, inference, HRTF and scene machinery

omniphony-renderer/orender_engine/
  headless Current construction and rendering boundary

omniphony-renderer/realtime_ffi/
  narrow realtime ABI used by the Windows APO

omniphony-renderer/windows_installer/endpoint_apo/
  normal Windows 0.1 endpoint APO, installer, tray and diagnostics

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