# Omniphony practical rendering plan

> **Scope of this document:** the Windows stereo-music product and its binaural renderer.
>
> General AI-hearing research now belongs in the separate private [`dissonance-git/libaural`](https://github.com/dissonance-git/libaural) project.

Omniphony is the first practical consumer and testbed for libaural, but it is not the parent hearing project.

```text
libaural
what appears to be happening in the sound?
        ↓
objects / fields / relations / history / confidence
        ↓
Omniphony
how should that scene reach two ears?
```

The product target is:

> **Make ordinary Windows stereo music present as a stable, externalized, convincing full 360° auditory scene over headphones while preserving clarity, timbre, bass relationships, transients, dynamics, and the musical hierarchy of the master.**

---

## 1. Listener contract

The mature experience should approach:

```text
install once
choose output / headphones once
play music normally
```

Drag-and-drop or offline file rendering can exist for development, fixtures and regression testing. It is not the intended daily workflow.

The current established foobar2000 DSP + HeSuVi chain remains available while Omniphony develops beside it.

```text
CURRENT LISTENING PATH
foobar DSP + HeSuVi
        │
        │ remains available
        ▼
OMNIPHONY DEVELOPMENT
        │
        ├── deterministic tests
        ├── objective/artificial-listener tests
        ├── frozen A/B fixtures
        └── optional human listening
        │
        ▼
Omniphony becomes independently stable and clearly better
        │
        ▼
old pieces become redundant one by one
```

No cold-turkey migration.

The listener's valuable job is to hear and adjudicate meaningful perceptual differences, not to become the build engineer.

---

## 2. First-install bar

The first normal listening build should already have a reasonable expectation of being a **large audible improvement** over basic headphone playback.

Do not ask for a disruptive installation merely to expose an internal engineering baseline.

Internal controls, dry renders and narrow feature experiments can be automated. A user-facing build should combine enough surviving improvements to make the comparison worthwhile.

---

## 3. The 360° target

The goal is not:

```text
stereo
+ width
+ hall reverb
```

The goal is a scene with independent kinds of spatial entities:

```text
DIRECT OBJECT
narrow or moderately broad source-like identity

BROAD OBJECT
source-like identity with significant apparent extent

DIFFUSE / AMBIENT FIELD
field-like energy that should not be forced into a point

ROOM FIELD
shared acoustic context / reflections / late energy
```

A typical reconstructed scene may contain:

```text
                 FRONT

       object      lead      object

   broad source           direct source

LEFT          LISTENER          RIGHT

        field          object

             rear object

                  REAR
```

The renderer may use front, side, rear, height and radial/depth cues as evidence allows.

### Spatial specificity follows confidence

```text
high-confidence object
→ precise placement is allowed

medium-confidence object
→ broader / safer placement

low-confidence organization
→ preserve mixture or render as a field
```

The system should become spectacular by becoming more correct, not by becoming more arbitrary.

---

## 4. Rear objects are not rear reverb

This is a hard product distinction.

```text
DIRECT REAR OBJECT
identity + trajectory + rear spatial state
→ direct binaural cues
→ appropriate object-linked reflections

DIFFUSE REAR FIELD
ambience / room / decorrelated energy
→ distributed rear energy
→ early/late room response
```

A backing vocal, percussion stream, synth texture, guitar, effect or other secondary musical object may legitimately occupy rear or rear-lateral space when scene evidence and musical hierarchy support it.

Do not turn every secondary component into room wash merely because rear ambience is easy to synthesize.

---

## 5. Fidelity laws

### Clarity-preserving dimensionality

Every spatial addition has to earn its cost.

A wider/deeper scene fails if matched bypass restores:

- clearer transients;
- better bass timing;
- more natural timbre;
- stronger vocal identity;
- more intelligible mix relationships;
- less phasey coloration.

### Object integrity

A source may become spatially larger without ceasing to sound like itself.

### Cue agreement

ITD, ILD, HRTF spectrum, early reflections, distance, diffuseness and motion should describe compatible geometry. Contradictory cues often create inside-head localization, blur or instability.

### Preserve truth; infer only absence

When a source provides trustworthy authored scene information, retain it. Ordinary stereo is where reconstruction is needed most.

### Original master remains authority

Source separation, semantic models and scene estimates provide control evidence. They do not automatically replace the corresponding waveform in the master.

---

## 6. Current inherited renderer path

The upstream Omniphony foundation already contains valuable binaural and spatial machinery.

The current conceptual binaural route includes:

```text
source/object position
→ head-relative direction
→ azimuth / elevation / distance
→ air/distance processing
→ per-ear timing
→ HRTF
→ early reflections
→ late room field
→ [L, R]
```

The fork should improve this path rather than rebuild everything.

### Highest-priority renderer experiment: directional early reflections

The inherited path has historically included early-reflection geometry whose ear rendering can be less spatially specific than the direct source.

Preferred direction:

```text
image source
→ reflection direction
→ delay / attenuation
→ reflection-specific HRTF/ITD
→ ears
```

Acceptance criteria:

- externalization improves;
- source body/ASW improves appropriately;
- listener envelopment can improve independently;
- localization remains stable;
- no audible echo/doubling;
- no comb-filter coloration;
- transient envelope remains intact;
- bounded CPU cost;
- frozen legacy control remains available for A/B.

### ASW and LEV remain independent

```text
direct source + fused early reflections
→ apparent source width / source body

later distributed field
→ listener envelopment / room
```

Do not use one global "more spacious" knob internally when the perceptual jobs are different.

---

## 7. Stereo inference

Ordinary stereo is the primary practical source, so Omniphony needs a safe path from stereo evidence to scene control.

The first port from the older `spatial-dsp` experiment is:

```text
renderer::stereo_inference
```

It currently provides inspectable evidence for:

- stereo pan;
- phase coherence;
- channel asymmetry / pan intensity;
- hard-pan-safe directness;
- complementary diffuseness;
- time-constant-based persistence;
- lateral stability.

It deliberately **does not** decide that a frequency bin is an instrument or send it to a speaker by itself.

See [`SPATIAL_DSP_MIGRATION.md`](SPATIAL_DSP_MIGRATION.md).

As libaural matures, additional evidence can enter here:

- onset binding;
- temporal coherence / common fate;
- pitch continuity;
- timbre identity;
- masking / audibility;
- object memory;
- prediction;
- competing hypotheses.

---

## 8. HeSuVi as an oracle, not an endpoint

The existing listening chain is useful because it demonstrates that a large 360° headphone bubble is perceptually achievable and desirable for the target listener.

The goal is not to permanently stack Omniphony on top of HeSuVi.

```text
TODAY
stereo
→ foobar spatial DSP
→ virtual multichannel bed
→ HeSuVi HRIR
→ headphones

TARGET
stereo
→ Omniphony scene inference
→ Omniphony binaural renderer
→ headphones
```

We should reproduce the cues that survive controlled comparison, not reproduce every implementation detail of the old chain.

Rear energy in the old chain is particularly useful as evidence that the listener accepts meaningful behind-head presentation from stereo. Omniphony's job is to make that presentation scene-aware and musically justified.

---

## 9. Known-scene reconstruction tests

The inherited renderer can also be tested with sources whose spatial structure is already known.

```text
KNOWN RICH SCENE
       │
       ├── direct Omniphony binaural render ──► reference percept
       │
       └── controlled stereo collapse
                    ↓
             scene reconstruction
                    ↓
             same binaural renderer
                    ↓
             reconstructed percept
```

This measures how much perceptually useful spatial organization can be recovered after information is deliberately removed.

The objective is not perfect recovery of historical production metadata. It is recovery of perceptually important organization.

---

## 10. Development order

### Phase 0 · Reproducible Windows base

- Windows x64 GitHub Actions build;
- release artifact;
- deterministic file rendering;
- regression fixtures;
- simple smoke tests;
- clean repository scope.

### Phase 1 · Renderer improvement bundle

- directional early reflections;
- HRTF interpolation / coverage;
- direct-vs-field rendering;
- room/radial depth;
- bass/transient protection;
- reproduce the useful parts of the existing listening bubble without HeSuVi dependence.

### Phase 2 · Stereo scene inference

- integrate `stereo_inference` into a bounded scene path;
- add persistence and object/field confidence;
- consume libaural evidence as it stabilizes;
- allow real rear objects independently from room fields.

### Phase 3 · First coexisting listening build

- normal Windows/foobar listening path;
- simple enable/bypass;
- no drag-and-drop requirement;
- current chain remains available;
- useful defaults instead of a cockpit of controls.

### Phase 4 · Wean the old chain

Replace old components only after Omniphony demonstrates the same valuable cue/function with equal or better fidelity.

### Phase 5 · Mature Windows path

- always-on route if justified;
- headphone compensation/personalization;
- richer source formats;
- games/surround as secondary consumers.

---

## 11. Repository contraction rule

The upstream repository is broader than this fork needs.

During refactoring, keep code if it is one of:

```text
1. directly useful to Windows stereo→binaural playback
2. load-bearing for a retained renderer component
3. useful for deterministic regression / known-scene testing
4. useful as a temporary oracle during migration
```

Otherwise it is a candidate for removal.

Do not delete a large subsystem merely because its product surface is out of scope before CI proves no retained code depends on it.

Likely removal/retirement candidates include broad upstream product surfaces such as Studio-centric visualization, mpv distribution support, cross-platform product packaging and general speaker-authoring UX once their useful test/infrastructure pieces are separated.

---

## 12. North star

After the project matures, an ordinary song should be able to do this:

```text
play stereo music
        ↓
Omniphony understands enough of its organization
        ↓
front remains anchored where it should
objects can occupy real lateral/rear/depth positions
room and ambience surround rather than smear
bass remains physical and timed
transients remain sharp
headphones stop feeling like the source
```

Then matched-loudness bypass should collapse the space without revealing that the spatial version cheated by sacrificing the music.
