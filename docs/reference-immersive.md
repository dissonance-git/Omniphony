# Reference Immersive

> Experimental research track for the `dissonancehelix/Omniphony` fork.
>
> **North star:** make headphones stop presenting themselves as the apparent source of the music. Ordinary recordings should be capable of becoming a stable, externalized, volumetric acoustic world around a fixed listener, while preserving the musical identity, clarity, timbre, transients, dynamics, and intent of the source.

This document is the execution charter for that work inside the Omniphony repository. It is intentionally narrower than a general spatial-audio roadmap: it defines what we are trying to prove, what must not be sacrificed, how experiments are compared, and which architectural seams should remain open while the research is still uncertain.

The project is not trying to make a louder, wider, wetter stereo effect. It is not trying to imitate one commercial virtualizer. It is not trying to claim recovery of unknowable original spatial intent from a stereo master.

The target is a different perceptual class of headphone playback: **the music should appear to occupy an acoustic volume rather than the space between two earcups.**

---

## 1. Product thesis

Headphones are excellent transducers but conventional stereo playback presents only a restricted projection of a musical scene. A recording may contain evidence of foreground and background, source grouping, direct and diffuse energy, apparent width, reverberant structure, motion, mix hierarchy, and temporal identity, yet conventional playback largely compresses that organization into a left/right image.

Reference Immersive asks whether a renderer can re-expand the surviving organization into a coherent three-dimensional percept without destroying the source.

A useful conceptual model is a **latent immersive master**:

```text
musical performances / tracks / rooms / effects / mix hierarchy
                         ↓
                  stereo projection
                         ↓
                       L / R

Reference Immersive:

                       L / R
                         ↓
            infer only supported structure
                         ↓
         construct a plausible 3D acoustic scene
                         ↓
               binaurally render the scene
                         ↓
                    headphones
```

The latent immersive master is not claimed to be a uniquely recoverable historical object. In general it is not. It is a disciplined reconstruction problem: preserve what the recording supports, infer cautiously where evidence exists, and avoid false precision where it does not.

### Desired first-listen reaction

The strongest aspirational test is simple: play one familiar song with Reference Immersive, then bypass it at matched loudness.

The desired loss on bypass is **acoustic volume**. The room should collapse. Radial depth, vertical organization, externalization, separation, and surrounding continuity should disappear together while the underlying music remains recognizably the same.

If bypass mainly sounds cleaner, sharper, more natural, or less phasey, the processing failed.

If processed playback mainly sounds louder, bassier, wetter, or wider, the processing failed.

---

## 2. Non-negotiable perceptual constraints

### 2.1 Clarity-preserving dimensionality

Spatial dimensionality may increase only while direct-source clarity remains intact.

The optimization target is approximately:

```text
maximize:
  externalization
  angular separation
  radial depth
  elevation
  source extent
  envelopment
  room scale
  stable spatial identity
  ambient continuity

subject to bounded change in:
  timbre
  transient shape
  direct-source coherence
  bass stability
  loudness
  spectral balance
  phase / group delay
  source identity
```

No amount of space compensates for smeared vocals, watery cymbals, softened attacks, unstable bass, or wandering objects.

### 2.2 Cue agreement

A believable source is not just an HRTF coordinate. Direction, distance, apparent size, direct/reverberant ratio, early reflections, spectral cues, and room response should describe the same imaginary acoustic event.

Contradictory cues create an effect. Consistent cues create a world.

### 2.3 Spatial specificity follows confidence

The scene model should not pretend every time-frequency patch is a point source.

```text
high confidence   → precise persistent object
medium confidence → wider spatial region
low confidence    → diffuse / ambient field
no evidence       → do not invent unnecessary precision
```

This is both a fidelity rule and a stability rule.

### 2.4 Spatial persistence

Human auditory organization develops over time. Reference Immersive should therefore prefer persistent interpretations over frame-by-frame rearrangement.

An object's position, extent, and identity should remain stable until new evidence is strong enough to justify a change. Motion must be earned.

### 2.5 One binaural transform

Reference Immersive output is already binaural stereo. It must not then be passed through HeSuVi, Windows Spatial Audio, another HRTF virtualizer, or any other second spatial transform during controlled evaluation.

Double virtualization is an invalid test condition.

---

## 3. Preserve truth; infer only absence

Long-term, the renderer should accept sources with different amounts of native spatial structure through one canonical scene interface.

```text
explicit authored objects / ADM / PMD
              ↓
native Ambisonics / HOA
              ↓
discrete surround channels
              ↓
structured source layers
              ↓
ordinary stereo
              ↓
mono
```

The more structure survives upstream, the less inference Reference Immersive should perform.

For the current project stage, **ordinary stereo music is the primary target**. Native structured game-music, VGM, SPC, sequence, and emulator integrations are deliberately later expansions. Those ecosystems are useful now because they reveal what pre-flattened musical structure can look like; they are not part of the initial implementation scope.

---

## 4. Why Omniphony is the base

Omniphony already has several architectural properties that are unusually well aligned with this research:

- an independent binaural path that bypasses the VBAP/speaker chain;
- per-channel/object 3D positions;
- separate ITD and HRIR processing;
- measured KEMAR and SOFA HRTF sources;
- first-order shoebox early reflections;
- a shared stereo FDN late-reverb field;
- distance and air-absorption behavior;
- real-time Rust rendering;
- ASIO output on Windows;
- non-realtime file output;
- a stable decoder bridge ABI;
- OSC supervision and an existing 3D Studio;
- a self-contained multichannel demo suitable for repeatable tests.

The intention is to raise the ceiling of this architecture before adding a complicated stereo scene-inference frontend.

---

## 5. Canonical internal scene direction

The exact API is intentionally not frozen yet, but the renderer should evolve toward three conceptually distinct signal classes:

```text
                     SCENE
                       │
          ┌────────────┼────────────┐
          │            │            │
     DIRECT OBJECTS  AMBIENT FIELD  ROOM FIELD
          │            │            │
      precise HRTF    HOA / SH      early + late
      stable XYZ      diffuse       source-linked
      radial depth    extent        externalization
      sharp attacks   envelopment   environment
          │            │            │
          └────────────┼────────────┘
                       ↓
                 binaural output
```

These classes should not be forced through the same algorithm merely for implementation convenience.

### Direct objects

Direct objects carry localization-critical material. They should retain sharp attacks and use the highest-precision practical binaural path. A discrete per-source HRIR path remains the default reference for important objects.

### Ambient field

Diffuse, broad, or low-confidence material should not be fabricated into point sources. A third-order HOA field is the leading internal representation to test. If introduced, use conventional **ACN channel ordering and SN3D normalization** so the field remains interoperable with Google spatial-media, libspatialaudio, and other standards-oriented tooling.

### Room field

Room response is not decorative reverb. Early reflections and late energy provide evidence about externalization, distance, enclosure, and source/room relationship.

The early field should remain directionally informative and source-linked. The late field should become increasingly diffuse after the room mixing time.

---

## 6. Immediate renderer research frontier

These experiments are ordered because later scene intelligence cannot compensate for a mediocre binaural renderer.

### R1. Reproducible baseline first

Before changing spatial acoustics, preserve known configurations and render identical fixtures through every candidate path.

The frozen configs in `omniphony-renderer/assets/reference-immersive/` exist for this purpose:

- `baseline-room.yaml`: current room-assisted KEMAR baseline;
- `dry-binaural.yaml`: same basic binaural geometry without early or late room contribution.

They are comparison anchors, not claims of final tuning.

Every major renderer experiment should be capable of producing deterministic offline output from the same input fixture.

### R2. Directional early reflections

The current reflection bank uses correct image-source geometry and relative propagation delay, but the six reflection returns are broadband ILD-panned copies rather than fully direction-dependent binaural events.

This is one of the clearest high-value experiments.

Candidate implementations:

1. spatialize every first-order image with an HRIR;
2. encode early images into a compact HOA bus and binauralize the bus once;
3. use a hybrid strategy where the earliest/highest-energy images receive precise HRTFs and weaker reflections enter the field representation.

Acceptance criteria:

- better externalization and source/room geometry;
- preserved azimuth precision;
- no audible transient doubling or comb-like coloration;
- bounded CPU cost;
- old reflection path remains available as the baseline during evaluation.

### R3. HRTF geometry and interpolation

Current `HrirSet` precomputes a 10° regular azimuth/elevation grid from -40° to +90° elevation and bilinearly interpolates aligned HRIRs. The separate analytic ITD is an important strength and should be preserved unless evidence shows otherwise.

Experiments:

- 5° grid;
- full lower-hemisphere coverage;
- spherical-neighbor rather than rectangular az/el interpolation where appropriate;
- compare measured-grid preprocessing against libmysofa-style nearest-neighbor interpolation;
- evaluate diffuse-field equalization and other direction-independent spectral normalization strategies;
- test higher-resolution grids for audible benefit before making them permanent.

Do not expose grid resolution as a normal listener control.

### R4. Direct-object path vs HOA field path

OpenAL Soft provides a useful control architecture: unique per-source HRIRs maximize directional response, while higher-order Ambisonic HRTF modes progressively trade CPU for directional clarity.

Reference Immersive should test a **hybrid** rather than choosing one globally:

- high-confidence direct objects → direct HRTF;
- diffuse / ambient material → fixed-order HOA;
- room tail → diffuse field.

The leading starting point is third-order HOA (16 channels), not because order 3 is sacred but because it is a practical point already used by multiple mature renderers.

### R5. Psychoacoustic HOA optimization

A mathematically valid HOA decoder is not automatically the best perceptual decoder.

Compare at least:

- basic HOA binauralization;
- max-rE / frequency-dependent high-frequency optimization;
- diffuse-field equalization;
- MagLS-style approaches where implementation and licensing permit;
- direct-object reference renders.

The test criterion is perceptual localization and timbral fidelity, not elegance of the equations.

### R6. Radial depth and near field

Distance must become an independent dimension rather than a synonym for lower gain or more reverb.

Investigate:

- direct-to-reverberant ratio;
- source-linked early-reflection geometry;
- air absorption;
- near-field / proximity filtering;
- apparent source extent vs distance;
- stable low-frequency anchoring;
- whether explicit near-field HRTF compensation materially improves the 0.2–1.0 m region.

### R7. Late field refinement

The existing FDN is a strong base and should be refined rather than casually replaced.

Potential experiments:

- frequency-dependent decay fitting;
- physically plausible transition from directional early field to diffuse late field;
- room-size dependent mixing time;
- interaural-coherence targets based on measured BRIRs;
- offline comparison with BinauralSDM and measured-room datasets;
- optional learned/offline room models such as DiffRIR as an oracle, not initially as a realtime dependency.

---

## 7. Stereo scene frontend comes after the renderer ceiling rises

The future stereo frontend should analyze the mix as a perceptual scene rather than route FFT bins to speakers.

Useful grouping evidence from auditory-scene research includes:

- harmonicity;
- onset synchrony;
- common changes in frequency/intensity (common fate);
- pitch and timbre similarity;
- stereo level/phase/coherence cues;
- transient ownership;
- decay and directness;
- temporal predictability;
- continuity over multiple time scales.

A possible internal entity:

```text
AuditoryRegion {
    confidence
    continuity
    onset_coherence
    harmonic_coherence
    common_fate
    directness
    diffuseness

    azimuth
    elevation
    distance
    extent

    age
    persistence
    movement_confidence
    room_coupling
}
```

This is deliberately a **region/object hypothesis**, not a promise to reconstruct physical stems.

The frontend should prefer soft masks and metadata/control signals applied to the original waveform wherever possible. Reconstructed stems are permitted only if experiments show that the perceptual benefit outweighs separation artifacts.

---

## 8. Learned spatial intelligence is a serious research lane

Modern learned stereo-to-immersive systems are close to the actual product problem and must be evaluated scientifically rather than dismissed because they are generative.

### ImmersiveFlow

The 2026 paper formulates direct stereo-to-7.1.4 generation with flow matching in a learned latent space. It is almost exactly the task statement “turn stereo into an immersive master.”

As of the initial Reference Immersive setup pass, the paper names `violet-audio/ImmersiveFlow`, but that GitHub repository is not currently resolvable through the GitHub API. Treat the paper as a research candidate until code, weights, license, and inference requirements can be verified.

Potential use if reproducible:

- full 7.1.4 output as an offline oracle;
- compare its inferred spatial organization against deterministic analysis;
- derive training targets or metadata rather than necessarily replacing the source waveform;
- evaluate transient, phase, timbre, and generalization failure modes.

### Ambisonizer

`yongyizang/ambisonizer` is currently public and releases model weights/scripts for neural generation of Ambisonic B-format from mono/stereo inputs. Its current stereo-conditioned output is first-order W/X/Y, so it is not the final spatial resolution target, but its **representation choice** is especially relevant: learned spatial intelligence can output a continuous sound field rather than a fixed 7.1.4 speaker bed.

A future hybrid may therefore look like:

```text
stereo
  ├── deterministic perceptual analysis ─────────────┐
  └── learned spatial/semantic analysis ─────────────┤
                                                     ↓
                                      confidence-aware scene metadata
                                                     ↓
                                           ORIGINAL waveform energy
                                                     ↓
                                   direct objects + HOA fields + room
                                                     ↓
                                              Omniphony binaural
```

No architecture choice is protected from evidence. If a learned full-waveform method sounds materially better without unacceptable damage, it remains a contender.

---

## 9. HRTF and headphone personalization

Personalization is a future multiplier, not a prerequisite for a good default.

The generic KEMAR path must remain convincing enough that Reference Immersive works immediately on normal headphones.

Future lanes include:

- user-provided SOFA HRTFs;
- perceptual HRTF selection from a database;
- anthropometric / ear-image prediction;
- camera/video based HRTF estimation;
- headphone transfer-function compensation;
- channel-matching correction where useful.

Personalized HRTFs must be validated perceptually. Individualization does not automatically guarantee higher overall preference or timbral quality.

Keep **scene rendering** and **headphone translation** conceptually separate:

```text
canonical binaural world
          ↓
headphone-specific translation / correction
          ↓
physical headphones + DAC / amplifier
```

Better headphones should reveal more of the generated spatial information, not require a different artistic scene algorithm.

---

## 10. Evaluation protocol

### 10.1 Loudness matching is mandatory

A candidate is not allowed to win because it is louder. Compare integrated loudness and peak/headroom before subjective judgment.

### 10.2 Acclimation + removal test

For important changes:

1. level-match baseline and candidate;
2. listen to the candidate long enough for its scene to normalize;
3. bypass or switch to baseline;
4. record what perceptually disappeared;
5. switch back and check whether clarity/timbre defects become newly obvious.

Desired result: volume/depth/height/externalization collapse on removal while source quality remains essentially intact.

### 10.3 Single-song dimensional-leap test

For later milestones, use a familiar track and a listener who has not been briefed on the implementation. Ask whether the difference is best described as:

- louder / wider / wetter / more processed, or
- a more credible external three-dimensional acoustic volume.

Only the second advances the north star.

### 10.4 Objective guardrails

Automated measurements cannot prove immersion, but they can catch regressions.

Track where practical:

- peak and integrated loudness;
- spectral deviation / tonal balance;
- transient crest-factor change;
- interaural cross-correlation / coherence;
- direct-path energy;
- low-frequency mono/coherence stability;
- impulse-response arrival times;
- reflection direction/delay correctness;
- deterministic output hashes for fixed fixtures;
- NaN/Inf and clipping;
- CPU time at 48 kHz and representative block sizes;
- allocations/locks on the audio thread.

### 10.5 Perceptual score axes

Listening notes should separate:

- front externalization;
- rear discrimination;
- side precision;
- elevation;
- radial distance;
- source extent;
- source separation;
- source stability;
- room scale;
- ambient continuity;
- transient clarity;
- vocal/direct clarity;
- timbral fidelity;
- fatigue;
- bypass-collapse strength.

A single “immersive 8/10” score throws away too much information.

---

## 11. Reference fixtures

The test corpus should eventually contain both synthetic and musical fixtures.

Synthetic / diagnostic fixtures:

- impulse at front, side, rear, above, below;
- slow azimuth sweep;
- elevation sweep;
- distance sweep;
- centered mono impulse;
- correlated vs decorrelated noise;
- bass-only material;
- transient + diffuse decay;
- known 7.1.4 speaker impulse sequence;
- multiple simultaneous point sources.

Musical fixtures:

- centered vocal + wide accompaniment;
- hard-panned material;
- dense orchestral mix;
- sparse acoustic recording;
- electronic mix with broad synthetic fields;
- old narrow/mono-derived recording;
- bass-heavy modern master;
- bright transient-heavy percussion;
- material with strong natural room ambience.

Do not tune only on a small set of favorite tracks. Reference should generalize.

---

## 12. Research influences and how to use them

These are **influences and controls**, not a dependency shopping list.

### Google Open Binaural Renderer (`google/obr`)

Important ideas:

- channels and objects can enter a common Ambisonic representation;
- object/channel content is encoded to third-order HOA;
- HRTF/BRIR filters can be decomposed into spherical harmonics;
- Direct, Ambient, and Reverberant binaural profiles are treated separately;
- an offline WAV CLI makes controlled A/B rendering easy.

### OpenAL Soft (`kcat/openal-soft`)

Important control architecture:

- full unique per-source HRIR rendering for maximum directional clarity;
- optional Ambisonic HRTF modes from first through fourth order for different CPU/clarity tradeoffs;
- conventional AmbiX ACN/SN3D support.

Reference Immersive is likely to benefit from using these as separate signal classes rather than one global mode.

### libspatialaudio (`videolabs/libspatialaudio`)

Important ideas:

- unified objects / speaker feeds / HOA / binaural architecture;
- ACN/SN3D up to third order;
- SOFA support;
- psychoacoustic high-frequency Ambisonic optimization;
- efficient spherical-harmonic HRTF binauralization.

Its LGPL/commercial licensing must be considered before copying implementation code. Concepts and independent reimplementation remain useful.

### Spatial Audio Framework

Already present in Omniphony's ecosystem. Continue using it as a mathematical/reference toolbox for HRIR, SH, VBAP, near-field, decorrelation, spread, and binaural experiments, while respecting module-specific licensing.

### BinauralSDM (`facebookresearch/BinauralSDM`)

Important room-design law:

- directional direct/early energy;
- direction-independent late reverberation after a configurable mixing time;
- BRIR/SOFA export makes it useful as an offline reference.

### libmysofa (`hoene/libmysofa`)

Useful control for SOFA loading, normalization, nearest-neighbor search, linear interpolation, coordinate conventions, and caching. Its BSD-3-Clause license also makes it a useful implementation reference if needed.

### Hearing Anything Anywhere / DiffRIR

Useful as an offline learned-room oracle. It models room impulse response fields and can synthesize binaural room behavior from SADIE HRIR data. Initial use should be research comparison, not a realtime runtime dependency.

### Dolby / DTS / Sony 360 / Waves Nx / SteelSeries Sonar / HeSuVi

Commercial systems are perceptual/UX benchmarks rather than code dependencies. Extract design laws: object/bed separation, radial depth, room-mediated externalization, headphone translation, easy routing, robust defaults, and one spatializer at a time.

### QuadraphonicQuad

Use experienced surround listeners as an adversarial taste benchmark. A bad upmix with copied fronts, extra reverb, and indiscriminate rear/height energy must not pass merely because it fills more channels.

---

## 13. Licensing boundary

Reference Immersive is intended to remain capable of becoming a clean open-source project.

Before adding code, datasets, HRTFs, neural weights, or algorithms from another project:

1. verify the exact license of the relevant component, not merely the repository homepage;
2. separate “scientific idea used as influence” from copied implementation;
3. do not bundle commercial virtualizer impulse responses;
4. preserve attribution for permissively licensed data/code;
5. keep optional research tooling from contaminating the realtime core when licenses differ;
6. document model-weight and training-data licensing independently from code licensing.

The current fork is GPL-3.0-or-later. That does not automatically make every external asset redistributable.

---

## 14. Development phases

### Phase 0 — establish the laboratory

- reproduce current stock/demo binaural behavior;
- freeze baseline and dry configs;
- establish deterministic offline render fixtures;
- establish level-matched A/B procedure;
- benchmark current CPU/latency;
- verify Windows ASIO path and direct physical-output path;
- record the current Noire X listening reference without treating one headphone as universal truth.

### Phase 1 — raise the native binaural ceiling

- HRTF grid/interpolation experiments;
- directional early reflections;
- direct-object vs HOA-field split;
- psychoacoustic HOA optimization experiments;
- near-field/radial depth;
- refine rather than replace the current FDN;
- preserve native multichannel/object behavior.

### Phase 2 — deterministic stereo scene frontend

- primary/ambient analysis;
- interchannel coherence and panning evidence;
- onset/transient analysis;
- harmonic/common-fate grouping;
- persistent regions;
- confidence-aware spatial specificity;
- soft masks applied to original waveform energy;
- bass and transient protection.

### Phase 3 — learned spatial intelligence

- reproduce credible stereo-to-immersive research systems;
- compare learned HOA vs discrete 7.1.4 targets;
- use models as waveform generators, scene teachers, or metadata predictors depending on measured fidelity;
- hybridize only where it beats deterministic baselines.

### Phase 4 — automatic Reference tuning

- remove unnecessary public controls;
- derive robust adaptive defaults;
- make normal operation essentially ON/OFF + output device;
- optional HRTF/headphone auto selection;
- maintain an Advanced/Diagnostics surface for research only.

### Phase 5 — system-wide Windows path

Prototype:

```text
Windows applications
      ↓
virtual endpoint / cable
      ↓
Omniphony service
      ↓
Reference Immersive binaural
      ↓
physical DAC / headphones
```

A native virtual endpoint may come later. Do not start driver work until the renderer has demonstrated enough perceptual value to justify it.

### Phase 6 — structured/native music expansion

Only after the general renderer and music scene model are mature:

- VGM / chip voices;
- SPC source voices;
- sequencer-aware playback;
- vgmstream layers;
- MIDI / tracker structures;
- other source representations that preserve pre-stereo musical organization.

These paths may eventually exceed what ordinary stereo inference can achieve because they retain information that a stereo mix discarded. They are intentionally not the current dependency chain.

---

## 15. Immediate next actions

The next implementation work should be deliberately small and testable:

1. reproduce `baseline-room.yaml` and `dry-binaural.yaml` offline from the same multichannel fixture;
2. establish a repeatable stereo WAV wrapping/conversion step for file-backend output;
3. capture baseline CPU time and level metrics;
4. add a renderer experiment switch for **directional early reflections** while preserving the current broadband-ILD implementation as control;
5. build impulse tests that prove each image source has the expected delay and direction;
6. separately prototype a full-sphere / higher-resolution HRTF grid and benchmark it against the current 10° grid;
7. do not begin the full stereo inference frontend until at least one renderer experiment demonstrates a clear level-matched perceptual win.

---

## 16. Kill conditions

Reference Immersive should be difficult to impress.

Reject a change when:

- it adds space by making the direct signal less clear;
- its advantage disappears after loudness matching;
- it sounds impressive for ten seconds but tiring over an album;
- objects shimmer, teleport, or change apparent size without evidence;
- bass loses a stable foundation;
- height comes mainly from brightness/EQ rather than convincing spatial cues;
- rear energy is merely copied front material with delay/reverb;
- a simpler baseline sounds more natural;
- CPU or latency cost rises dramatically without a proportional perceptual gain;
- implementation complexity exists mainly because the algorithm is clever.

**Correction outranks cleverness.** If a simpler chain sounds better, keep the simpler chain.

---

## 17. Working definition of success

Reference Immersive succeeds when a listener stops evaluating “the spatial effect” and instead accepts a coherent external acoustic scene.

The deepest goal is not to hear more speakers.

It is to stop hearing the headphones.
