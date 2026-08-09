# Omniphony

**Omniphony is a Windows-first stereo music enhancer and binaural spatial renderer.**

The practical goal of this fork is deliberately narrow:

> **Take ordinary stereo music and present it over headphones as a stable, externalized, convincing full 360° auditory scene without sacrificing musical identity, clarity, timbre, bass relationships, transients, dynamics, or mix hierarchy.**

This is not intended to be "stereo + wider crossfeed + room reverb." The target scene can contain direct auditory objects in front, beside, above/below where supportable, and **behind the listener**, alongside broad objects and diffuse room/ambient fields.

Rear auditory objects and rear reverberation are different things.

```text
ordinary stereo music
        ↓
signal-derived + libaural-informed scene inference
        ↓
persistent auditory objects / broad sources / fields
        ↓
Omniphony binaural scene renderer
        ↓
headphones
        ↓
full 360° acoustic presentation
```

The mature user experience should be boring in the best possible way:

```text
install once
choose the output/headphones once
play music normally
```

No drag-and-drop ritual is intended for normal listening.

---

## Origin and attribution

This repository is a fork and research/product refactor built on the original **[mgth/Omniphony](https://github.com/mgth/Omniphony)** project.

The upstream project supplied the foundational spatial renderer, binaural/HRTF path, object and speaker-scene machinery, audio I/O, bridge architecture, room/reflection work, and a large amount of engineering that this fork did not originate.

The fork keeps the original Git history and GPL-3.0-or-later licensing. See [`NOTICE.md`](NOTICE.md) for the project boundary and attribution statement.

The direction of this fork is different from upstream: instead of remaining a broad spatial-audio suite, it is being reduced around a specific Windows headphone-music product and used as the first practical testbed for [`libaural`](https://github.com/dissonance-git/libaural).

The name **Omniphony** is retained for now because the original renderer is still the substrate. The product may receive a different name later if the code and identity diverge far enough from upstream.

---

# What this fork is trying to build

## Listener-facing target

For ordinary stereo music:

```text
                 FRONT

          direct object
      object           object

  broad source       direct source

LEFT        LISTENER        RIGHT

       field       object

            object
            BEHIND

                 REAR
```

The system should be able to represent and render:

- stable frontal lead objects;
- lateral objects;
- **direct rear objects when the evidence supports them**;
- broad/extended auditory objects;
- diffuse ambience and room fields;
- depth and distance relationships;
- height/elevation cues when defensible;
- persistent object identity through time;
- a stable bass/groove floor;
- musical hierarchy rather than arbitrary surround spectacle.

The headphones should gradually stop feeling like the apparent source of the sound.

## What it must preserve

Spatial enlargement is a failure if it buys dimension by damaging the recording.

Non-negotiable preservation targets:

```text
clarity
transient shape
bass timing and weight
timbre
vocal/instrument identity
stereo relationships
microdynamics
macrodynamics
mix hierarchy
rhythmic precision
musical continuity
```

The north-star listening test is matched-loudness bypass:

> After acclimation, bypassing Omniphony should make ordinary playback feel dimensionally collapsed, **without bypass restoring clarity, punch, timbre, bass definition, or transient precision that Omniphony damaged.**

---

# Rear objects are not reverb

A major product requirement is that the 360° field is not faked solely by sending decorrelated/reverberant energy behind the listener.

These are separate scene entities:

```text
DIRECT / OBJECT-LIKE REAR SOUND
source identity
+ persistent trajectory
+ rear spatial state
+ appropriate binaural cues

DIFFUSE REAR FIELD
ambience / room / decorrelated field
+ broad directional distribution
+ early/late acoustic response
```

A backing vocal, percussion element, effect, texture, guitar, synth, or other secondary stream may become a real rear-lateral auditory object if doing so is supported by the scene evidence and does not destroy the original musical hierarchy.

Low-confidence evidence should produce conservative spatial behavior rather than arbitrary object placement.

```text
high confidence
→ spatially specific

medium confidence
→ broader / safer placement

low confidence
→ preserve mixture / field
```

---

# Relationship to libaural

Omniphony is the first practical consumer and testbed for **libaural**, the separate parent auditory-intelligence project.

```text
libaural
"what auditory organization is supported by this sound?"
        ↓
objects / fields / relations / history / confidence
        ↓
Omniphony
"how should that scene reach two ears?"
```

libaural owns the general research problem:

- cochlear/peripheral representations;
- multiresolution spectrotemporal analysis;
- temporal coherence and grouping;
- pitch formation;
- timbre/identity constancy;
- onset binding;
- masking and audibility;
- attention;
- auditory memory;
- predictive processing;
- persistent auditory objects;
- uncertainty and competing hypotheses;
- general and music-specific auditory understanding.

Omniphony owns the practical rendering problem:

- Windows playback integration;
- realtime scene transformation;
- binaural HRTF/ITD rendering;
- direct and diffuse spatial presentation;
- early reflections and room/depth cues;
- headphone translation;
- latency/stability;
- product defaults;
- human listening validation.

A rendering trick does not become a libaural law merely because it sounds good here. A general hearing mechanism can graduate upward after controlled evidence.

---

# Spatial DSP inheritance

The earlier [`dissonance-git/spatial-dsp`](https://github.com/dissonance-git/spatial-dsp) foobar2000 experiment is being mined for useful behavior, not copied as the final architecture.

That project demonstrated several practical ideas worth carrying forward:

- direct-vs-diffuse analysis from stereo phase and level relationships;
- treating hard-panned dry energy as source-like instead of falsely diffuse;
- coherent-center preservation;
- side-difference evidence;
- temporal memory so spatial state does not chase transients;
- stable lateral object evidence;
- bass anchoring;
- independent source/body and ambient/rear energy;
- decorrelation as an acoustic cue rather than as the definition of space;
- exaggerated 360° presentation that can still retain a frontal/bass anchor.

Omniphony should implement the useful ideas directly inside its scene and binaural renderer rather than reproduce the old chain:

```text
stereo
→ pseudo 7.1 bed
→ HeSuVi HRTF
```

The intended endpoint is:

```text
stereo
→ scene inference
→ Omniphony binaural rendering
→ headphones
```

The first clean Rust inference primitive ported from that work lives in `renderer::stereo_inference`.

See [`docs/SPATIAL_DSP_MIGRATION.md`](docs/SPATIAL_DSP_MIGRATION.md).

---

# Current renderer substrate

The inherited Omniphony renderer already contains valuable machinery we should reuse rather than rewrite casually:

- object/spatial scene representation;
- VBAP and speaker geometry;
- binaural HRTF + ITD rendering;
- early reflections and room processing;
- distance/spread logic;
- realtime/offline rendering paths;
- Windows audio-output support;
- reference bridge and file paths useful for deterministic tests;
- runtime control and measurement infrastructure.

The fork is being reduced around the pieces that help the Windows music product.

Some inherited upstream subsystems may remain temporarily while dependency and CI tests establish whether they are load-bearing. Their presence in the tree does **not** mean they remain product scope.

---

# Scope

## In scope

- Windows 10/11 x64 first;
- ordinary stereo music as the primary source;
- common decoded audio formats through normal playback integration;
- realtime headphone output;
- deterministic file rendering for tests;
- full-sphere binaural presentation;
- direct rear objects as well as diffuse rear space;
- room/depth/externalization;
- HRTF work and eventual headphone compensation/personalization;
- scene inference using signal-derived and libaural-derived evidence;
- simple enable/bypass comparison during migration;
- automatic build artifacts through GitHub Actions;
- strong regression and listening controls.

## Not current product scope

These may survive temporarily as inherited infrastructure, fixtures, or research tools, but they are not what this fork is trying to ship:

- a general speaker-layout authoring suite;
- a cross-platform desktop visualization product;
- an mpv distribution;
- a universal ADM production environment;
- a generic Ambisonics workstation;
- a plugin ecosystem for arbitrary decoder formats;
- requiring head tracking for the core effect;
- exposing dozens of DSP controls to the normal listener;
- reproducing HeSuVi internally as a fixed matrix trick;
- solving all of AI hearing inside this repository.

Ambisonics, HOA, VBAP, SAF, HRTFs, neural acoustic fields and related techniques are tools. None of them is the product identity.

---

# Transition from the current listening chain

Development must coexist with the established working playback chain until Omniphony earns replacement.

```text
current foobar DSP + HeSuVi chain
        │
        │ remains available
        ▼
Omniphony develops beside it
        ↓
simple enable / bypass / alternate route
        ↓
normal music listening
        ↓
Omniphony proves stability + quality
        ↓
old components become redundant one by one
        ↓
eventual path:
Windows / player
→ Omniphony
→ audio device
→ headphones
```

The first user-facing listening build should already contain enough high-confidence improvements to be worth installing. Engineering baselines can be tested automatically and offline.

---

# Development order

## 0. Make the fork reproducibly buildable

- clean Windows CI;
- downloadable x64 artifact;
- deterministic file-output path;
- frozen regression fixtures;
- preserve a known-good control.

## 1. Improve the inherited binaural renderer

Priority areas:

- directional early reflections rather than broadband pan approximations;
- HRTF interpolation/coverage;
- direct-vs-diffuse rendering paths;
- source body vs listener envelopment;
- radial depth / externalization;
- cue consistency;
- no transient smear or comb coloration.

## 2. Stereo scene inference

Start with inspectable evidence:

- phase coherence;
- channel asymmetry/pan intensity;
- M/S relationships;
- temporal persistence;
- spectral/modulation evidence;
- onset/transient stability;
- directness/diffuseness;
- broad vs object-like behavior.

Then consume increasingly mature libaural scene state.

## 3. First practical Windows listening build

- ordinary playback path;
- simple enable/bypass;
- sensible automatic defaults;
- first-run improvement should be obvious enough to justify testing;
- no drag-and-drop requirement.

## 4. Wean the old chain

Only remove an existing component from normal use after Omniphony demonstrably replaces the cue/function the listener values.

## 5. Broader Windows integration

- always-on system route if justified;
- game/surround support as secondary use;
- richer authored source formats;
- headphone translation/personalization.

---

# Build

The fork now has a Windows x64 GitHub Actions path at:

```text
.github/workflows/windows-renderer.yml
```

It builds the Rust renderer with the MSVC toolchain and ASIO support, smoke-tests the CLI, generates SHA-256 hashes, and uploads the Windows binaries as an Actions artifact.

For local development, the current renderer workspace remains under:

```text
omniphony-renderer/
```

See the inherited build documentation there while the Windows-first docs are being simplified.

---

# Repository refactor status

This fork is currently in a **scope-contraction refactor**.

The immediate rule is:

> Keep inherited code when it is useful to the Windows stereo→binaural product, a deterministic test, or a clearly load-bearing dependency. Remove or archive the rest once CI proves it is safe to do so.

This avoids throwing away excellent upstream engineering merely because its original application was broader than this fork.

The research that used to live partly inside this repository is being split correctly:

```text
GENERAL AI HEARING RESEARCH
→ libaural

WINDOWS MUSIC / 360° BINAURAL PRODUCT
→ Omniphony
```

See [`docs/headphone-rendering-research.md`](docs/headphone-rendering-research.md) for the practical renderer plan.

---

# License

GPL-3.0-or-later, inherited from the original Omniphony project. See [`LICENSE`](LICENSE) and [`NOTICE.md`](NOTICE.md).
