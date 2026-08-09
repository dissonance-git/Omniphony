# Spatial DSP → Omniphony migration

The earlier `dissonance-git/spatial-dsp` foobar2000 component is a useful experimental ancestor of the current Windows music goal, but its architecture should not become the final Omniphony architecture.

Its old practical chain was roughly:

```text
decoded audio
→ stereo analysis
→ pseudo-object 7.1 bed
→ downstream HeSuVi / virtual-surround HRTF
→ headphones
```

Omniphony can collapse that chain into a more principled path:

```text
stereo
→ auditory / scene evidence
→ objects + broad sources + fields
→ direct binaural rendering
→ headphones
```

The goal of migration is therefore **behavioral inheritance without architectural cargo culting**.

The wider scene/rendering boundary now lives in [`SCENE_RENDERER_CONTRACT.md`](SCENE_RENDERER_CONTRACT.md).

---

## What the old experiment got right

### 1. Hard-panned dry material is still source-like

A naive stereo coherence test can fail badly when one channel is nearly silent. The phase of the quiet channel becomes numerically unstable, so a clean hard-left or hard-right source may appear incoherent.

The useful correction is to combine two kinds of evidence:

```text
shared material
→ phase alignment matters

strongly asymmetric material
→ channel localization itself is evidence of a coherent source
```

The old directness estimate effectively used:

```text
pan_intensity = |L - R| / (L + R)
phase_align   = max(0, cos(phase_L - phase_R))

directness = pan_intensity
             + phase_align * (1 - pan_intensity)
```

This has already been cleanly reimplemented in Rust as:

```text
renderer::stereo_inference
```

The Rust version treats the result as **evidence**, not as an automatic routing command.

### 2. M/S evidence must retain phase

The first Rust port exposed `mid_magnitude` and `side_magnitude` using scalar channel magnitudes. That was too weak: equal-amplitude antiphase material could incorrectly appear all-mid because phase had already been discarded.

The implementation now reconstructs each complex FFT bin and computes:

```text
M = (L + R) / 2
S = (L - R) / 2

mid_magnitude  = |M|
side_magnitude = |S|
```

This is now regression-tested with both in-phase and antiphase stereo material.

### 3. Time matters, and first-frame agreement is not persistence

The old component learned that frame-local pan estimates cause transient material to jump around spatially.

It added per-bin memory and a stability test so a lateral component needed to remain consistent before receiving stronger object-like treatment.

Omniphony keeps the principle but improves the implementation:

- memory is expressed with a physical time constant rather than a fixed block-dependent EMA coefficient;
- trajectory **agreement** and temporal **persistence** are separate values;
- a newly observed component cannot receive perfect stability merely because its first sample agrees with itself;
- one-frame excursions are explicitly rejected in tests;
- stable-object evidence remains a score rather than a routing command.

Later scene logic can combine it with onset, timbre, masking, libaural object state and other evidence.

### 4. Direct and diffuse energy should not be treated the same

The old renderer distinguished source-like/direct energy from diffuse/ambient energy.

That maps naturally into the newer scene vocabulary:

```text
FRONTAL ANCHOR
DIRECT OBJECT
BROAD SOURCE
DIFFUSE / AMBIENT FIELD
ROOM FIELD
```

Omniphony should carry this distinction all the way into binaural rendering.

### 5. Rear content needs structure

The old system did more than add a hall tail. It created directional side/rear energy, used different delay/phase paths, and maintained lateral bias.

The new renderer should go farther:

```text
rear object
≠ rear reflection
≠ diffuse rear field
```

But an additional boundary is now explicit:

```text
stable lateral stereo evidence
≠ recovered rear-source metadata
```

A persistent source-like component may become **eligible** for a rear or rear-lateral presentation when musical-role and fidelity constraints support it. That is a renderer decision, not a claim that the stereo master encoded a physical rear position.

Diffuse energy should be rendered separately as field/room energy.

### 6. Bass needs a stable floor

The old experiment deliberately kept low-frequency content more frontal and less diffuse. That is a useful product law even if the exact crossover/weight curves are not sacred.

The principle is:

> Do not buy spatial size by dissolving the timing and weight of the groove floor.

The first `renderer::scene_inference` pass now contains a smooth low-frequency anchor prior rather than a hard crossover discontinuity.

Future implementations should evaluate bass preservation using matched loudness and transient/phase measurements, not merely preference.

### 7. The front image should remain authoritative

The old path preserved the original FL/FR bins and added spatial information around them rather than subtracting large amounts of the original signal.

The new equivalent is broader:

> Infer structure from the master, but preserve the master waveform and mix relationships unless a transformation has a specific, tested reason to alter them.

### 8. Decorrelation is a cue, not a scene model

Different short delay/phase paths can help externalization and envelopment.

But:

```text
decorrelated signal
≠ auditory object
≠ room
≠ rear source
```

Omniphony should use decorrelation only where it supports a scene entity that is already justified by evidence.

---

## What should *not* be copied directly

### Stereo → fixed pseudo-7.1 as the permanent internal truth

The old system needed a multichannel bed because HeSuVi was downstream.

Omniphony has a direct binaural renderer, so the internal representation can be richer than an eight-channel transport bed.

### Frequency curves as universal psychoacoustic laws

The old front/side/rear band weights and high-frequency elevation/rear boosts were useful listening experiments. They are not automatically general truths about human hearing.

Treat them as candidate priors or comparison controls.

### Fixed delay values as universal room geometry

The old 7/11/17/23 ms decorrelation paths were intentionally differentiated virtual paths. They were not measured room/image-source geometry.

The modern renderer should prefer explicit directional early-reflection geometry and HRTF processing where practical.

### A single directness number as full object identity

Directness is only one cue.

Object formation eventually needs combinations of:

- harmonicity;
- onset synchrony;
- temporal coherence/common fate;
- timbre/identity evidence;
- pitch continuity;
- masking/audibility;
- spatial continuity;
- memory and prediction;
- competing hypotheses.

Those broader mechanisms belong in libaural and can be consumed by Omniphony as they mature.

---

## Migration map

```text
SPATIAL DSP IDEA                    OMNIPHONY DESTINATION

phase/alignment directness      →  renderer::stereo_inference
hard-pan source detection       →  renderer::stereo_inference
scalar "M/S" proxy             →  true complex M/S evidence
fixed block EMA                 →  time-constant evidence tracker
first-frame "stability"        →  agreement × persistence maturity
stable lateral objects          →  scene candidate evidence
front lock                      →  FrontalAnchor / hierarchy policy
bass anchor                     →  smooth bass-preservation prior
rear orbit                      →  later presentation policy, not recovered metadata
short decorrelation delays      →  directional reflection / field cues
micro-room shell                →  early-reflection / room model
pseudo 7.1 output               →  remove from final internal architecture
HeSuVi dependency               →  direct Omniphony binaural output
```

---

## Tests to preserve from the old experiment

The migration should include regression fixtures for the failure modes that motivated the original code:

1. **Hard-panned dry tone/synth** must remain source-like even with meaningless opposite-channel phase.
2. **Centered in-phase material** should produce strong coherent/direct evidence and be mid-dominant.
3. **Balanced antiphase material** should produce strong diffuse/field evidence and be side-dominant.
4. **First observation of a lateral event** must not be treated as a mature persistent object.
5. **One-frame pan excursion** should break trajectory agreement and not be promoted as a persistent object.
6. **Stable lateral material** should accumulate stronger object evidence with time.
7. **Low bass** should not become spatially smeared merely because higher-frequency content is widened.
8. **Wet mix = 0 / bypass control** must preserve the source path exactly where the relevant stage claims true bypass.
9. **Rear-field energy** must be distinguishable from a direct rear object in the scene representation.
10. **Stereo scene inference** must never report `rear` as recovered acoustic truth from L/R evidence alone.

---

## Current status

- [x] Identify the useful algorithmic ideas.
- [x] Port hard-pan-safe directness estimation to Rust.
- [x] Replace scalar M/S proxies with phase-correct complex M/S magnitudes.
- [x] Replace fixed-block temporal memory with time-constant tracking.
- [x] Split trajectory agreement from persistence maturity.
- [x] Add unit tests for centered, hard-panned, antiphase, first-frame, pan-excursion and sustained-persistence cases.
- [x] Add an initial conservative Omniphony scene-evidence stage.
- [x] Add a smooth bass/frontal-anchor safeguard to the scene-evidence stage.
- [x] Make the `stable lateral ≠ rear truth` boundary explicit.
- [ ] Add deterministic audio fixtures derived from the old failure cases.
- [ ] Compare candidate object evidence against libaural grouping mechanisms.
- [ ] Add broad/diffuse spherical-field rendering rather than point-source fakery.
- [ ] Use reviewed object/field state to control binaural placement directly.
- [ ] Add objective fidelity gates for frequency response, dynamics, spectrum, distortion, transient/phase behavior and bypass identity.
- [ ] Remove any remaining need for stereo → pseudo-7.1 → external HRTF during normal Omniphony playback.
