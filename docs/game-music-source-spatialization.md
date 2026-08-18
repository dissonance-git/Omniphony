# Game-music source spatialization

This note records research precedents and resulting policy for the VGM/SPC/source-aware path. It is not a license to replace source truth with generic spatial effects. It **is** a license to make a deliberate immersive mix from real recovered sources when `FullSphere` is selected.

The canonical realtime transport and authority rules live in [`game-music-realtime-source-contract.md`](game-music-realtime-source-contract.md). This document explains why those rules exist and what they imply for presentation.

## Research question

What previous work supports keeping musical sources separate until spatial rendering, deliberately using width/depth/height/extent as production dimensions, preserving direct versus diffuse structure, and making a large immersive mix without confusing modern presentation with historical authorship?

## Foundational boundary

Game-music source spatialization uses Omniphony's existing product architecture:

```text
historical / executable source truth
    ↓
causal source objects
    ↓
AUTHORED routing / timing / identity
    ↓
DERIVED musical evidence + deliberate FullSphere presentation
    ↓
Omniphony canonical 8.1.4.4 semantic world
    ↓
22-direction render shell
    ↓
cascaded binaural renderer
```

The 17-lane 8.1.4.4 scene is a **presentation vocabulary**, not a required source topology.

A six-channel FM chip remains six source objects. An eight-voice S-DSP remains eight dry voices plus any separately proven shared wet return. The source path does not manufacture `L/R/C/...` PCM lanes merely to match the static scene vocabulary.

```text
hardware/source topology
!= canonical speaker vocabulary
!= render-shell topology
```

The audible goal of `FullSphere` is intentionally analogous to an immersive remix from multitracks: preserve what the musical sources actually are, then use a larger modern spatial canvas than the historical delivery hardware provided.

## Strong precedents

### Tsingos, Gallo, and Drettakis (2003), *Breaking the 64 spatialized sources barrier*

The paper treats spatialized soundtracks and sound effects as ordinary constituents of a dynamic game-audio scene. Most relevantly, it proposes using 3-D audio channels for a restitution-independent representation of surround music tracks, leaving the final mix to the rendering API.

Transfer:

- keep soundtrack sources separate until the renderer;
- let the renderer own output-device translation;
- treat source-count pressure as an allocation problem rather than a reason to collapse music early.

Difference from this project:

- VGM/SPC source lanes are recovered from historical execution state rather than authored as modern stems;
- modern CPU rendering removes the old 16-64 hardware-source ceiling for the small source counts typical of chip music;
- source identity must survive physical-voice reuse.

### Jot, Carpentier, and Warusfel (2023), *Perceptually Motivated Spatial Audio Scene Description and Rendering for 6-DoF Immersive Music Experiences*

DOI `10.1109/I3DA57090.2023.10289196`.

This work is especially important for the corrected product target. It treats position, distance, orientation, presence and reverberance as object-scene production controls and explicitly allows auditory plausibility to outrank literal physical-room simulation.

Transfer:

- separate source description from renderer presentation;
- treat spatial placement as a legitimate creative dimension of an immersive music mix;
- do not require a fictitious historical room to justify a perceptually coherent larger scene;
- preserve source truth while allowing the renderer to choose modern presentation geometry.

### Ziemer (2017), *Source Width in Music Production*

DOI `10.1007/978-3-319-47292-8_10`.

Perceived source width is an established music-production dimension across stereo, surround, Ambisonics and wave-field synthesis. Interaural phase/correlation structure and source radiation behavior contribute to perceived extent.

Transfer:

- source width is not merely localization error;
- recovered chip voices need not remain perceptual points merely because the hardware exposed a single channel;
- width should remain controlled enough that source identity and localization are not lost.

### Potard and Burnett (2004), decorrelation for apparent source width

Their work reviews decorrelation techniques for reducing interaural cross-correlation and reports that controlled decorrelation can produce intended apparent source extent.

Transfer:

- interaural coherence is a useful control for making an object perceptually wider;
- source extent should be an explicit DSP mechanism, not just a metadata number.

### McCormack, Politis, and Pulkki (2021), *Rendering of Source Spread for Arbitrary Playback Setups Based on Spatial Covariance Matching*

DOI `10.1109/WASPAA52581.2021.9632724`.

The method combines centre-source signals with decorrelated variants and solves for a target diffuse spatial covariance while constraining signal distortion. It is evaluated for binaural playback and explicitly targets signal fidelity.

Transfer:

- covariance/coherence targets are a useful comparison against Omniphony's first shell-spread implementation;
- source width can be increased while treating fidelity as an optimization constraint;
- the centre/localization anchor and the spread field need not be the same component.

### Anemüller, Thiergart, and Habets (2024), *Binaural Rendering of Heterogeneous Sound Sources with Extent*

DOI `10.1109/ICASSP48485.2024.10448024`.

This work models extended binaural sources through target spatial covariance and evaluates the result against point-source and homogeneous-extent baselines.

Transfer:

- a binaural source can legitimately have extent independent of its centre position;
- size must influence the actual binaural ear signals rather than stop at scene metadata;
- covariance-based rendering remains a useful alternate implementation to compare with the current cascaded-shell method.

### Landschoot and Jot (2023), *Binaural externalization processing method for object-based audio rendering*

DOI `10.1121/10.0018389`.

This work treats externalization as an object-aware rendering problem rather than a global headphone effect.

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
- the wet field can contribute envelopment without masquerading as independent dry objects.

### Tohyama (2020), spatial impression and binaural sound fields

Direct sound and early-reflection regions carry strong localization information, while later reverberant energy contributes more strongly to diffuseness/spaciousness as interaural correlation falls.

Transfer:

- localizable dry source state should not be merged prematurely with diffuse/shared energy;
- diffuseness and point-source position are different presentation dimensions.

### Zannini, Parisi, and Uncini (2011), binaural localization in reverberation

DOI `10.1109/ICDSP.2011.6004954`.

Reverberation increases uncertainty in binaural localization.

Transfer:

- highly diffuse evidence should not make the dry-source centre wander;
- source position needs temporal stability rather than callback-by-callback guessing.

### Stecker (2023), RESTART theory

DOI `10.1121/10.0023479`.

The RESTART account emphasizes transient/envelope-triggered sampling of reliable spatial cues as part of stable auditory scene formation.

Transfer as a research pressure test:

- onsets/transients may be high-information moments for **updating** a derived source pose;
- stable steady-state periods can preserve the current pose instead of continually re-estimating it;
- onset alone does not supply a musical role or 3-D coordinate.

### Hedges, Sazdov, and Johnston, systematic reviews of audio in games/XR

Their reviews find that spatial fidelity generally contributes to immersion, but also report diminishing perceptual returns and inconsistent evaluation methodology.

Transfer:

- evaluate localization, externalization, stability, timbre preservation and musical coherence separately;
- do not equate more spatial movement or more reverberation with a better mix;
- keep a protected reference condition.

### Collins and Kapralos (2024), *Auditory Reality and Virtuality*

Their game-audio discussion warns that increasingly literal spatial simulation can reduce artistic effectiveness rather than improve it.

Transfer:

- the target is not physical realism for its own sake;
- musical hierarchy and expression can override naive geometric distribution;
- foundation material may remain frontal even when the renderer can place it anywhere.

## Direct/localizable versus shared/diffuse energy

```text
localizable dry source objects
!= shared/diffuse historical wet field
!= Omniphony externalization reflections
```

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

The shared echo network is historical common processing. It may be presented as a broad environmental field while the dry voices remain localizable objects. Omniphony's own early externalization field remains a separate modern presentation mechanism.

In FullSphere, the shared S-DSP field may spend substantially more extent over the 22-direction shell than individual dry voices. Its chosen center and native linked L/R identity remain separate from that extent decision.

## Source authority by device family

| Device/path | Preserve as source truth | AUTHORED spatial/routing evidence | FullSphere may DERIVE |
| --- | --- | --- | --- |
| YM2612 | six complete FM channels, including truthful channel-6/DAC state where applicable | native L/R enables + exact timing | width, rear depth, elevation, distance, extent |
| Genesis PSG | three tone voices + noise | ordinary stock Genesis PSG has no independent stereo pan register | complete immersive placement beyond identity |
| Game Gear PSG | three tone voices + noise | explicit per-channel L/R routing | unsupported height/depth/extent |
| YM2151 | eight complete FM channels | native channel L/R enables | width, rear depth, elevation, distance, extent |
| SNES S-DSP dry | eight voice episodes where dry capture is proven | signed per-voice L/R route + echo-send | unauthored immersive geometry |
| SNES S-DSP wet | one shared wet return where observed | shared-return identity/provenance | diffuse/environmental field presentation |

The authority rule is:

> **Preserve every source-native fact. Use the missing dimensions freely when FullSphere is deliberately selected, but keep those mix decisions DERIVED.**

## FM channel boundary

For ordinary YM2612/YM2151-style synthesis, spatial object identity begins at the **complete audible FM channel**.

Individual operators are synthesis internals participating in algorithms, modulation and feedback. They are not independently spatialized musical sources by default.

Likewise, a whole-chip enhanced renderer may sound or measure better without exposing exact additive per-channel enhanced stems. Shared serial mixer/DAC paths, clamps or other coupling require an explicit causal/additivity witness before independent enhanced lanes are admitted.

```text
FM operator != default spatial object
whole-chip fidelity != exact independent stems
```

## Temporal stability policy

FullSphere placement should behave like a mix layout, not a randomizer.

```text
stable source / persistent-part identity
→ stable creative baseline

stable musical evidence
→ stable evidence-shaped refinement
```

A neutral real source is allowed a deliberate rear, height or depth placement simply because FullSphere is the selected production mode. It must not receive a fresh dramatic coordinate every block.

The behavioral obligations are:

- block-size changes do not alter a stable source's trajectory;
- weak role/spectral fluctuations do not cause spatial jitter;
- source or persistent-part identity can seed repeatable layout coordinates;
- foundation and strong foreground evidence can pull the mix toward stable front anchors;
- a strong new onset plus corroborating evidence may permit a bounded pose update;
- authored timed route/position changes retain exact event time;
- unrelated sources reusing hardware slots do not inherit one another's pose ramps;
- persistent musical parts that migrate across hardware slots may retain continuity.

Derived position has **inertia, but not glue**.

## Source extent is a real renderer obligation

The source model carries 3-D `size`, and FullSphere now translates that metadata into the headphone signal through Omniphony's existing shell renderer:

```text
source centre + [width, depth, height]
→ size-aware object event
→ VBAP spread across 22-direction System-H-derived shell
→ fixed virtual-speaker field
→ cascaded binaural HRTF / ITD
```

The FullSphere precomputed evaluator guarantees at least five size states (`0, .25, .5, .75, 1`) and interpolates between them. This keeps extent independent from center position while avoiding a second bespoke stereo-widening stage.

The research suggests a useful conceptual split that the implementation should continue to preserve:

```text
source centre
→ localization anchor

source extent
→ bounded angular occupation of the shell

shared wet field
→ larger environmental envelopment budget

Omniphony early reflections
→ optional externalization support
```

A future direct-HRTF covariance/decorrelation renderer remains a valuable comparison against shell spread, especially for timbral coloration and apparent-width efficiency. It is now an alternate research path, not a missing prerequisite for source extent to be audible in FullSphere.

Do not buy width by smearing transients, destroying tonal identity or turning every source into diffuse ambience.

## Music-specific adjacent work

Interactive spatial-music systems consistently treat spatialization as an independent compositional/presentation dimension operating on separated musical objects.

That supports:

```text
historical source truth
    → causal source objects
    → authored constraints + musical evidence
    → deliberate Omniphony FullSphere mix
    → 8.1.4.4 semantic world
    → size-aware 22-direction shell
    → cascaded binaural renderer
```

It does **not** justify claiming that the resulting modern 3-D positions were authored by the game.

## What appears novel in this project

The literature search did not surface a prior system that combines all of the following:

1. reads preserved retro game-music execution artifacts such as VGM or SPC;
2. reconstructs original chip/DSP source lanes and routing before final stereo collapse;
3. preserves historical shared-DSP structure such as S-DSP echo separately;
4. derives stable musical identity across physical voice reuse;
5. hands those causal sources to a modern object-based binaural renderer at playback time;
6. retains an exact/reference historical mix as a control;
7. deliberately remixes unsupported dimensions into a stable modern full-sphere presentation while preserving authority labels.

Treat this as a research gap, not a novelty claim until a broader literature and implementation search fails to find prior art.

## Implementation decisions

### Borrow

- source-preserving rendering;
- object/source allocation instead of early downmix;
- direct/wet separation;
- stable source identity;
- perceptually motivated object placement;
- apparent-source-width / spread mechanisms with fidelity constraints;
- confidence-weighted temporal continuity;
- optional early-reflection externalization;
- protected reference comparison.

### Reject

- forcing dynamic chip voices into seventeen fake PCM lanes because Omniphony uses an 8.1.4.4 semantic vocabulary;
- finished-stereo pseudo-surround as the source-native game-music path;
- callback-random or purely spectacular 3-D scattering unrelated to stable identity and musical hierarchy;
- treating creative FullSphere coordinates as authored metadata;
- refusing to use rear/height/depth merely because historical hardware did not encode them;
- room simulation without a clearly separated presentation purpose;
- treating more width, rear energy or height as monotonically better;
- folding historical echo/reverb into new renderer reflections;
- cloning one historical shared wet return into one wet stem per dry voice;
- allowing a temporary hardware channel number to own persistent scene position;
- exposing FM operators as independent spatial objects by default.

## VGM/SPC consequence

For VGM and SPC, the foobar **Surround** option should mean source-aware Omniphony FullSphere:

```text
Surround off
    → protected historical/reference stereo

Surround on
    → causal source-native topology
    → AUTHORED routing / timing / identity constraints
    → stable source / persistent-part layout
    → musical evidence shapes the layout
    → DERIVED width / rear depth / height / distance / extent
    → Omniphony 8.1.4.4 semantic world + dynamic objects
    → size-aware 22-direction full-sphere shell
    → cascaded binaural stereo
```

A separate externalization option may control Omniphony early reflections because geometry, extent and externalization are distinct controls.

## Evaluation axes

Every source-aware spatial listening build should score at least:

- source localization;
- front/back discrimination;
- elevation discrimination;
- apparent source width / extent;
- envelopment;
- externalization;
- scene stability across note/voice transitions;
- callback/block-size invariance;
- native-routing preservation;
- AUTHORED versus DERIVED provenance preservation;
- bass/foundation stability;
- historical shared-effect integrity;
- direct-versus-diffuse separation;
- timbral coloration;
- transient integrity;
- musical coherence;
- whether the scene feels intentionally mixed rather than algorithmically scattered;
- preference versus reference.

Engineering tests should fail if FullSphere creative coordinates are promoted to authored source facts, NativeRouting leaks creative rear/height/depth, source extent does not alter FullSphere headphone output, shared wet is fabricated into per-source stems, shared-wet extent moves the field center instead of only changing its occupation, authored route evidence is retimed/overridden, or a whole-chip renderer is promoted to independent source stems without decomposition evidence.