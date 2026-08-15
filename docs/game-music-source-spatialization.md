# Game-music source spatialization

This note records research precedents for the VGM/SPC source-aware path. It is not a license to replace source truth with generic spatial effects.

## Research question

What previous work has spatialized game audio or music before the final stereo mix, and which ideas transfer to Omniphony's VGM/SPC source path?

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

This work treats externalization as an object-aware rendering problem rather than merely a global headphone effect. It reviews reflections/reverberation, head tracking, HRTF choice, and related cues for reducing inside-the-head localization.

Transfer:

- externalization is orthogonal to source geometry;
- source-aware externalization is preferable to one global stereo widening stage;
- frontal music objects deserve particular attention because they are prone to internalization.

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

## Music-specific adjacent work

Research on interactive spatial music, including SpatOSC, SCLiss, and VR music-spatialization interfaces, consistently treats spatialization as an independent compositional/presentation dimension operating on already-separated musical objects.

That supports the architectural boundary used here:

```text
historical source truth
    -> causal source lanes
    -> musical/presentation evidence
    -> Omniphony scene policy
    -> binaural renderer
```

It does **not** justify claiming that inferred 3-D positions were authored by the game.

## What appears novel in this project

The literature search did not surface a prior system that combines all of the following:

1. reads preserved retro game-music execution artifacts such as VGM or SPC;
2. reconstructs original chip/DSP source lanes and routing before final stereo collapse;
3. preserves historical shared-DSP structure such as S-DSP echo separately;
4. derives stable musical identity across physical voice reuse;
5. hands those causal sources to a modern object-based binaural renderer at playback time;
6. retains an exact/reference historical mix as a control.

Treat this as a research gap, not a novelty claim until a broader literature and implementation search fails to find prior art.

## Implementation decisions

### Borrow

- source-preserving rendering;
- object/source allocation instead of early downmix;
- direct/wet separation;
- stable source identity;
- perceptually motivated object presentation;
- optional early-reflection externalization;
- protected reference comparison.

### Reject

- finished-stereo pseudo-surround as the game-music path;
- automatically placing every chip voice at a different dramatic 3-D coordinate;
- room simulation without source evidence;
- treating more width, rear energy, or height as monotonically better;
- folding historical echo/reverb into new renderer reflections;
- allowing a temporary hardware channel number to own persistent scene position.

## VGM/SPC consequence

For VGM and SPC, the existing foobar **Surround** option should become the source-aware Omniphony path:

```text
Surround off
    -> protected historical/reference stereo

Surround on
    -> causal source lanes
    -> native routing constraints
    -> stable musical identity
    -> Omniphony full-sphere presentation
    -> binaural stereo
```

A separate externalization option may control Omniphony early reflections, because geometry and externalization are perceptually and architecturally distinct.

## Evaluation axes

Every spatial listening build should score at least:

- source localization;
- front/back discrimination;
- elevation discrimination;
- externalization;
- scene stability across note/voice transitions;
- native-routing preservation;
- bass/foundation stability;
- historical shared-effect integrity;
- timbral coloration;
- musical coherence;
- preference versus reference.
