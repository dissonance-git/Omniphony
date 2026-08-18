# Game-music source spatialization

This note records research precedents and resulting policy for the VGM/SPC/source-aware path. It is not a license to replace source truth with generic spatial effects.

The canonical realtime transport and authority rules live in [`game-music-realtime-source-contract.md`](game-music-realtime-source-contract.md). This document explains why those rules exist and what they imply for presentation.

## Research question

What previous work has spatialized game audio or music before the final stereo mix, which auditory findings matter for source-object stability, and which ideas transfer to Omniphony's historical game-music source path?

## Foundational boundary

Game-music source spatialization uses Omniphony's existing product architecture:

```text
historical / executable source truth
    ↓
causal source objects
    ↓
AUTHORED routing / timing / identity
    ↓
DERIVED presentation evidence only where needed
    ↓
Omniphony canonical 8.1.4.4 semantic world
    ↓
22-direction render shell
    ↓
binaural renderer
```

The 17-lane 8.1.4.4 scene is a **presentation vocabulary**, not a required source topology.

A six-channel FM chip remains six source objects. An eight-voice S-DSP remains eight dry voices plus any separately proven shared wet return. The source path does not manufacture `L/R/C/...` PCM lanes merely to match the static scene vocabulary.

```text
hardware/source topology
!= canonical speaker vocabulary
!= render-shell topology
```

This is the key separation that lets Omniphony gain source truth without forcing historical game hardware into a modern speaker-bed fiction.

## Strong precedents

### Tsingos, Gallo, and Drettakis (2003), *Breaking the 64 spatialized sources barrier*

The paper treats spatialized soundtracks and sound effects as ordinary constituents of a dynamic game-audio scene. Most relevantly, it explicitly proposes using 3-D audio channels for a restitution-independent representation of surround music tracks, leaving the final mix to the rendering API.

Transfer:

- keep soundtrack sources separate until the renderer;
- let the renderer own output-device translation;
- treat source-count pressure as an allocation problem rather than as a reason to collapse the music early.

Difference from this project:

- VGM/SPC source lanes are recovered from historical execution state rather than authored as modern stems;
- modern CPU rendering removes the old 16-64 hardware-source ceiling for the small source counts typical of game-music chips;
- source identity must survive physical-voice reuse.

### Schissler and Manocha (2011), *GSound: Interactive Sound Propagation for Games*

This work demonstrates that per-source real-time propagation can be practical in game-like scenes, using cached reflection and diffraction paths rather than reducing the scene to one finished signal first.

Transfer:

- preserve per-source state long enough to apply source-specific presentation;
- cache slowly changing spatial state instead of recomputing every cue from scratch;
- separate direct sound from environmental propagation.

Non-transfer:

- VGM/SPC music has no historical room geometry to ray trace;
- Omniphony must not fabricate occlusion or architectural acoustics that the source artifact never encoded.

### Antani and Manocha (2013), *Aural Proxies and Directionally-Varying Reverberation for Interactive Sound Propagation in Virtual Environments*

This work reports improved localization and immersion from direction-dependent early reflections and reverberation in a real-time game-engine pipeline.

Transfer:

- early reflections can be a separate externalization control;
- environmental/wet energy should retain directional structure rather than becoming generic mono or identical-L/R reverb.

Constraint:

- historical shared DSP such as S-DSP echo remains source truth and must stay distinct from Omniphony's optional externalization reflections.

### Jot, Carpentier, and Warusfel (2023), *Perceptually Motivated Spatial Audio Scene Description and Rendering for 6-DoF Immersive Music Experiences*

The paper argues for a parametric object scene that can prioritize auditory plausibility over literal room simulation and gives independent control over source position, distance, orientation, presence, and reverberance.

Transfer:

- separate source description from renderer presentation;
- use perceptually meaningful controls rather than forcing every source through a physical-room fiction;
- share environmental processing where appropriate instead of manufacturing one reverb stem per source.

### Landschoot and Jot (2023), *Binaural externalization processing method for object-based audio rendering*

DOI `10.1121/10.0018389`.

This work treats externalization as an object-aware rendering problem rather than merely a global headphone effect. It reviews reflections/reverberation, head tracking, HRTF choice, and related cues for reducing inside-the-head localization.

Transfer:

- externalization is orthogonal to source geometry;
- source-aware externalization is preferable to one global stereo widening stage;
- frontal music objects deserve particular attention because they are prone to internalization.

### Menzies et al. (2021), object-based reverberant rendering

DOI `10.1109/TASLP.2020.3036781`.

This work explicitly separates dry/object source description from target reverberant response.

Transfer:

- dry/localizable source state and reverberant field state are different objects in the model;
- environmental processing should not erase source identity;
- a historical shared wet return can remain a shared field rather than being decomposed into invented point-source reverberation.

### Greco et al. (2025), direct / early / diffuse decomposition

DOI `10.1186/s13636-025-00437-y`.

The ParaDER approach separates direct sound, early reflections and diffuse reverberation instead of forcing all acoustic energy through one localization model.

Transfer:

- direct/localizable game-music source objects should remain distinct from shared/diffuse energy;
- early/localizable and late/diffuse energy can use different spatial policies;
- the wet field can contribute envelopment without masquerading as a set of independent dry objects.

### Tohyama (2020), spatial impression and binaural sound fields

The direct sound and early-reflection region carries strong localization information, while later reverberant energy contributes more strongly to diffuseness/spaciousness as interaural correlation falls.

Transfer:

- localizable dry source state should not be merged prematurely with diffuse/shared energy;
- diffuseness is a presentation dimension distinct from point-source position.

### Zannini, Parisi, and Uncini (2011), binaural localization in reverberation

DOI `10.1109/ICDSP.2011.6004954`.

Reverberation increases uncertainty in binaural localization.

Transfer:

- low-confidence or highly diffuse evidence should not trigger aggressive source motion;
- source position needs temporal stability and confidence tracking rather than callback-by-callback guessing.

### Stecker (2023), RESTART theory

DOI `10.1121/10.0023479`.

The RESTART account emphasizes transient/envelope-triggered sampling of reliable spatial cues as part of stable auditory scene formation.

Transfer as a research pressure test:

- onsets/transients may be high-information moments for **updating** a derived source pose;
- stable steady-state periods can preserve the current pose instead of continually re-estimating it;
- onset alone does not supply a musical role or 3-D coordinate.

This motivates position inertia with evidence-sensitive update windows, not a hardcoded transient-to-position mapping.

### Hedges, Sazdov, and Johnston, systematic reviews of audio in games/XR

Their reviews find that spatial fidelity generally contributes to immersion, but also report diminishing perceptual returns and inconsistent evaluation methodology.

Transfer:

- evaluate localization, externalization, stability, timbre preservation, and musical coherence separately;
- do not equate more spatial movement or more reverberation with a better result;
- keep a protected reference condition in listening tests.

### Collins and Kapralos (2024), *Auditory Reality and Virtuality*

Their game-audio discussion warns that increasingly literal spatial simulation can reduce artistic effectiveness rather than improve it.

Transfer:

- the target is not physical realism for its own sake;
- musical hierarchy and expression can override a naive geometric distribution;
- a foundation part may remain frontal even when the renderer could place it anywhere.

## Direct/localizable versus shared/diffuse energy

The game-music path adopts a strong architectural distinction:

```text
localizable dry source objects
!= shared/diffuse historical wet field
!= Omniphony externalization reflections
```

These three layers have different provenance.

For SNES, for example:

```text
8 dry S-DSP voice witnesses
+ authored per-voice L/R route and echo-send state
+ one separately observed shared post-EVOL wet return
```

must not become:

```text
8 invented dry+wet stems
```

The shared echo network is historically common processing. It can be presented as a diffuse/environmental field while the dry voices remain localizable objects.

Omniphony's own early externalization field is a separate modern presentation mechanism and must not be mistaken for historical S-DSP echo.

## Source authority by device family

Current source-family expectations:

| Device/path | Preserve as source truth | AUTHORED spatial/routing evidence | DERIVED only |
| --- | --- | --- | --- |
| YM2612 | six complete FM channel identities, including truthful channel-6/DAC state where applicable | native L/R enables and exact timing | elevation, depth, radial position, extent and other unsupported dimensions |
| Genesis PSG | three tone voices + noise | ordinary stock Genesis PSG has no independent authored stereo pan register | nearly all geometric placement |
| Game Gear PSG | three tone voices + noise | explicit per-channel L/R routing register | unsupported vertical/depth/extent dimensions |
| YM2151 | eight complete FM channels | native channel L/R enables | unsupported elevation/depth/full-sphere placement |
| SNES S-DSP dry | eight voice episodes where exact dry capture is proven | signed per-voice L/R route + echo-send state | unauthored geometry and semantic role |
| SNES S-DSP wet | one shared wet return where directly observed | shared-return identity/provenance | diffuse/environmental presentation |

The important rule is not the particular device list. It is the authority boundary:

> **Preserve every source-native fact, derive only the missing presentation dimensions, and never relabel the derivation as source truth.**

## FM channel boundary

For ordinary YM2612/YM2151-style synthesis, spatial object identity begins at the **complete audible FM channel**.

Individual operators are synthesis internals participating in algorithms, modulation and feedback. They are not independently spatialized musical sources by default.

Likewise, a whole-chip enhanced renderer may sound or measure better without exposing exact additive per-channel enhanced stems. Shared serial mixer/DAC paths, clamps or other coupling must be defeated by an explicit causal/additivity witness before independent enhanced lanes are admitted.

```text
FM operator != default spatial object
whole-chip fidelity != exact independent stems
```

## Temporal stability policy

A derived 3-D source presentation should behave as a tracked object.

```text
stable identity + stable evidence
→ stable position tendency
```

Do not recompute a fresh dramatic coordinate every audio block.

The renderer should carry at least the conceptual equivalents of:

```text
position
position / motion history
confidence
evidence age
persistent source identity
transition confidence
```

Not every quantity needs to cross the public ABI. Some can remain renderer-internal state.

The behavioral obligations are:

- block-size changes do not alter a stable source's trajectory;
- weak role/spectral fluctuations do not produce spatial jitter;
- confidence can decay without teleporting the source;
- a strong new onset plus corroborating evidence may permit a bounded pose update;
- authored timed route/position changes retain their exact event time;
- an unrelated source reusing a hardware slot does not inherit the outgoing source's pose ramp;
- a persistent musical part that migrates across hardware slots may retain ordinary continuity when identity evidence supports it.

Think of derived position as having **inertia, but not glue**.

## Music-specific adjacent work

Research on interactive spatial music, including SpatOSC, SCLiss, and VR music-spatialization interfaces, consistently treats spatialization as an independent compositional/presentation dimension operating on already-separated musical objects.

That supports the architectural boundary used here:

```text
historical source truth
    → causal source objects
    → musical/presentation evidence
    → Omniphony 8.1.4.4 semantic world
    → 22-direction shell
    → binaural renderer
```

It does **not** justify claiming that inferred 3-D positions were authored by the game.

## What appears novel in this project

The literature search did not surface a prior system that combines all of the following:

1. reads preserved retro game-music execution artifacts such as VGM or SPC;
2. reconstructs original chip/DSP source lanes and routing before final stereo collapse;
3. preserves historical shared-DSP structure such as S-DSP echo separately;
4. derives stable musical identity across physical voice reuse;
5. hands those causal sources to a modern object-based binaural renderer at playback time;
6. retains an exact/reference historical mix as a control;
7. maps only unsupported dimensions into a modern full-sphere presentation while preserving source authority.

Treat this as a research gap, not a novelty claim until a broader literature and implementation search fails to find prior art.

## Implementation decisions

### Borrow

- source-preserving rendering;
- object/source allocation instead of early downmix;
- direct/wet separation;
- stable source identity;
- confidence-weighted temporal continuity;
- perceptually motivated object presentation;
- optional early-reflection externalization;
- protected reference comparison.

### Reject

- forcing dynamic chip voices into seventeen fake PCM lanes because Omniphony uses an 8.1.4.4 semantic vocabulary;
- finished-stereo pseudo-surround as the game-music path;
- automatically placing every chip voice at a different dramatic 3-D coordinate;
- treating stable inferred coordinates as authored metadata;
- room simulation without source evidence;
- treating more width, rear energy, or height as monotonically better;
- folding historical echo/reverb into new renderer reflections;
- cloning one historical shared wet return into one wet stem per dry voice;
- allowing a temporary hardware channel number to own persistent scene position;
- exposing FM operators as independent spatial objects by default.

## VGM/SPC consequence

For VGM and SPC, the existing foobar **Surround** option should become the source-aware Omniphony path:

```text
Surround off
    → protected historical/reference stereo

Surround on
    → causal source-native topology
    → AUTHORED native routing / timing constraints
    → stable musical identity
    → DERIVED missing presentation dimensions
    → Omniphony 8.1.4.4 semantic world + dynamic objects
    → 22-direction full-sphere shell
    → binaural stereo
```

A separate externalization option may control Omniphony early reflections, because geometry and externalization are perceptually and architecturally distinct.

## Evaluation axes

Every spatial listening build should score at least:

- source localization;
- front/back discrimination;
- elevation discrimination;
- externalization;
- scene stability across note/voice transitions;
- callback/block-size invariance;
- native-routing preservation;
- AUTHORED/DERIVED/EMPTY provenance preservation;
- bass/foundation stability;
- historical shared-effect integrity;
- direct-versus-diffuse separation;
- timbral coloration;
- musical coherence;
- preference versus reference.

Engineering tests should additionally fail if unsupported scene dimensions are silently filled, a shared wet field is duplicated into fabricated stems, authored route evidence is retimed/overridden, or a whole-chip renderer is promoted to independent source stems without decomposition evidence.
