# Omniphony

Omniphony is an experimental, always-on spatial processor for headphones, built from the upstream `mgth/Omniphony` renderer and extended around one product rule:

> **Make the headphones disappear without making the recording disappear with them.**

The finished recording remains the musical authority. Omniphony may enlarge width, depth, height, distance, source extent and envelopment, but it must not need to sacrifice clarity, impact, center stability, timbre or rhythmic precision to do it.

Windows is the first product host. The renderer, scene contract and DSP core remain portable.

---

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
               │
               ├→ level / phase / M-S relation
               ├→ pan / coherence
               ├→ directness / diffuseness
               └→ temporal stability
                         │
                         ▼
             CANONICAL 8.1.4.4 SCENE
             17 semantic lanes
             L R C LFE Ls Rs Lb Rb Cb
             Tfl Tfr Tbl Tbr Bfl Bfr Bbl Bbr
                         │
                         ├→ only evidence-backed lanes become active
                         └→ missing authorship stays EMPTY
                         │
                         ▼
             CURRENT 22-DIRECTION SHELL
             System-H-derived full-sphere lattice
                         │
                         ▼
                CASCADED BINAURAL
              measured HRTF + ITD
              distance / air / room
                         │
                         ▼
                  binaural support
                         │
       protected master + foundation + support
                         │
                         ▼
                peak-safe stereo
                         │
                         ▼
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

That distinction is deliberate. A rich coordinate system must not become fake source metadata.

---

## What is implemented now

| Layer | Current state |
| --- | --- |
| Canonical static scene | **Implemented:** 17-lane 8.1.4.4 vocabulary |
| Stereo evidence mapping | **Implemented:** bounded stereo-derived support into earned lanes only |
| Current spatial shell | **Implemented:** 22-direction System-H-derived full-sphere lattice |
| Headphone renderer | **Implemented:** cascaded binaural with measured HRTF / ITD path |
| Directional early field | **Implemented:** bounded first-order timing and six HRTF reflection buses |
| Windows realtime ABI | **Implemented:** `omniphony_realtime.dll` |
| Windows endpoint APO | **Implemented for development bring-up:** stereo float32 Current path |
| Native authored 5.1 / 7.1 ingress through the APO | **Not implemented yet** |
| Raw Windows Spatial Audio object ingress | **Research frontier** |
| Production APO packaging | **In progress:** isolated component package exists; final target association, signing and protected-AudioDG proof remain |

A 7.1.4 fixture or layout is therefore a useful regression input, not the product base.

---

## Windows path

The intended installed topology is endpoint-native:

```text
applications / games / browsers / players
                 │
                 ▼
         Windows Audio Engine
                 │
                 ▼
        OMNIPHONY EFX APO
                 │
                 ├→ omniphony_realtime.dll
                 │      └→ Current scene → shell → binaural DSP
                 │
                 ▼
       physical endpoint driver
                 │
                 ▼
          DAC / headphones
```

The tray is preference-only. It does not host the audio engine.

The old process-loopback and virtual-device routes are migration history, not the product architecture.

### Development versus production

The repository intentionally keeps two Windows deployment concepts separate:

```text
DEVELOPMENT / BRING-UP
raw endpoint association
+ test-oriented AudioDG compatibility measures
+ rollback tooling

PRODUCTION TARGET
isolated componentized APO package
+ device-specific extension association
+ catalog / PE signing
+ protected AudioDG
+ upgrade / rollback / uninstall proof on physical hardware
```

Do not treat a successful development attach as proof of production deployment readiness.

See:

- [`docs/omniphony-for-windows.md`](docs/omniphony-for-windows.md)
- [`omniphony-renderer/windows_installer/endpoint_apo/README.md`](omniphony-renderer/windows_installer/endpoint_apo/README.md)
- [`omniphony-renderer/windows_installer/endpoint_apo/production/README.md`](omniphony-renderer/windows_installer/endpoint_apo/production/README.md)

---

## Fidelity laws

> **Dimension may not be purchased by damaging the music.**

Turning Omniphony off may collapse:

- width;
- front/back depth;
- height;
- radial distance;
- source extent;
- ambient continuity;
- envelopment.

Turning Omniphony off must **not** restore:

- clarity;
- kick impact;
- bass pressure;
- transient snap;
- tonal identity;
- center stability;
- microdetail;
- dynamics;
- comfortable spectral balance.

Shortest form:

> **OFF may collapse the world. It may not bring the rhythm section back to life.**

The protected stereo master never passes through the virtual room. FFT/STFT analysis may inform support decisions, but the master is not STFT-resynthesized.

---

## Source authority

The richer the source truth, the less Omniphony should infer.

```text
stereo
→ preserve the master + infer bounded presentation support

5.1 / 7.1 PCM
→ preserve authored directional channels
→ future native host path maps them into matching AUTHORED scene anchors

height beds
→ preserve supplied height when the host exposes it

object audio
→ preserve supplied object positions when available

Ambisonics / HOA
→ preserve the field representation rather than flattening early

already-binaural material
→ avoid destructive double HRTF virtualization
```

`AUTHORED`, `DERIVED` and `EMPTY` are not cosmetic labels. They are provenance states.

---

## Current perceptual model

Current combines three audible layers:

```text
1. protected master
   musical identity / center / transients / directness

2. coherent foundation
   bounded low-frequency and body reinforcement

3. spatial support
   evidence-driven external field
   → canonical scene
   → 22-direction shell
   → binaural rendering
   → early directional room support
```

The current early field uses lane-local transient evidence, first-order image timing and wall filtering, aggregates those contributions into six directional reflection buses, then applies measured HRTF rendering. The goal is directional structure with bounded cost, not a wash of generic reverb.

---

## Research grounding

Research is used to sharpen obligations and tests, not to overrule listening evidence.

The present architecture is consistent with several durable findings in binaural research:

- **Externalization is not one control.** Direct HRTF cues, room cues, interaural behavior and head motion can contribute differently.
- **Frontal externalization is especially difficult.** This justifies treating front scale and center directness as a dedicated frontier rather than assuming more surround energy solves it.
- **Binaural room cues matter.** Work on reverberation-related binaural cues shows that the relationship between direct and reflected interaural cues can strongly affect externalization, especially for frontal sources.
- **Timbral fidelity and spatial fidelity are coupled.** HRIR time-alignment and diffuse-field constraints have been shown to improve coloration, localization and externalization together in binaural Ambisonic rendering.
- **Head motion is a strong future lever.** Multiple controlled studies report improved externalization when virtual scenes remain stable relative to the world during listener motion.

Useful research anchors:

- Zaunschirm, Schörkhuber & Höldrich, *JASA* (2018), DOI `10.1121/1.5040489`
- Catic, Santurette & Dau, *JASA* (2015), DOI `10.1121/1.4928132`
- Leclère, Lavandier & Perrin, *JASA* (2019), DOI `10.1121/1.5128325`
- Brimijoin, Boyd & Akeroyd, *PLOS ONE* (2013), DOI `10.1371/journal.pone.0083068`
- Hendrickx et al., *JASA* (2017), DOI `10.1121/1.4978612`
- Landschoot & Jot, *JASA* (2023), DOI `10.1121/10.0018389`

These papers motivate the validation axes. They do not prove that any particular Omniphony tuning is perceptually superior.

---

## Validation

The repository separates source truth, rendering physics and product listening so one failure does not masquerade as another.

Engineering gates include:

- canonical 17-lane scene order and EMPTY-lane preservation;
- 17-lane scene reaching the 22-direction shell;
- final binaural stereo output;
- ITD / HRTF / diffuse-response checks;
- block-size and callback-invariance tests;
- transient and bass preservation;
- non-finite and peak-safety behavior;
- Windows APO ABI / manifest / import-table checks;
- production INF package-isolation checks.

Human listening remains the final gate for:

- externalization;
- front/back discrimination;
- elevation;
- source body and extent;
- envelopment;
- radial depth;
- center solidity;
- room naturalness;
- fatigue;
- groove and bass integrity.

---

## Repository map

```text
renderer/
  portable DSP, inference, HRTF and scene machinery

orender_engine/
  headless Current construction and rendering boundary

realtime_ffi/
  narrow realtime ABI used by the Windows APO

windows_installer/endpoint_apo/
  development endpoint APO, installer and diagnostics

windows_installer/endpoint_apo/production/
  isolated production-package scaffold and target-driver capture

layouts/
  reference and renderer geometry, including the Current 22-direction shell

docs/
  source authority, scene, Windows, listening and validation contracts
```

The inherited renderer remains the spatial core. Custom fork machinery should exist only where the product needs a capability the inherited core does not already own.

---

## Build and focused tests

From `omniphony-renderer/`:

```sh
cargo test -p renderer
cargo test -p orender_engine --test current_scene_geometry
cargo test -p realtime_ffi
```

Windows packaging and APO validation live in the repository-level GitHub Actions workflows.

---

## Relationship to libaural, VGM Tooling and Helix

These projects may exchange research and evidence, but they remain separate runtime systems.

```text
HELIX
research / provenance / method
        │
        ▼
libaural
experimental machine hearing
        │
        ├───────────────┐
        ▼               ▼
VGM Tooling         Omniphony
source truth        presentation / listening testbed
```

libaural may eventually provide source/activity evidence to Omniphony. That evidence must remain bounded control information unless a source-native path explicitly supplies authored audio structure. No project becomes a runtime dependency merely because it produced a useful experiment.

---

## Definition of success

> **A finished recording keeps its identity, weight, dynamics and clarity while gaining a stable external world with front distance, rear depth, extreme width, convincing height, continuous motion and enough radial scale that ordinary headphone playback feels dimensionally collapsed by comparison.**

That is the target. The scene model, the shell, the Windows APO and the research stack are all instruments for reaching it, not substitutes for it.
