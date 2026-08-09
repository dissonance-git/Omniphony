# Stereo scene → binaural renderer contract

Omniphony's hardest problem is not HRTF convolution by itself. It is the boundary between a finished stereo master and a convincing headphone scene.

The renderer must therefore keep three questions separate:

```text
1. What acoustic evidence exists in the stereo signal?
2. What scene organization is a defensible hypothesis?
3. How should that scene be rendered to two ears?
```

The third question must never rewrite the first two into fake certainty.

---

## 1. Stereo does not contain rear ground truth

A normal two-channel music master can provide evidence about:

- L/R balance;
- inter-channel phase/coherence;
- true complex M/S structure;
- direct versus diffuse behavior;
- persistence through time;
- spectral region;
- onset/offset relations;
- masking/audibility;
- later libaural object identity and musical role.

It generally does **not** tell us that an inferred source was physically recorded at a specific rear azimuth.

Therefore:

```text
stable lateral object evidence
≠
rear-source evidence
```

Rear placement in Omniphony is an intentional reconstruction/presentation decision constrained by evidence and musical hierarchy.

The renderer may create a compelling rear object. It must not mislabel that choice as recovered recording metadata.

---

## 2. Scene entity types

The practical scene vocabulary should remain small and typed.

### `FrontalAnchor`

Examples:

- centered lead/vocal authority;
- bass/groove foundation;
- low-frequency timing floor;
- mixture components whose relocation would destabilize musical hierarchy.

### `DirectObject`

A persistent source-like entity with enough evidence for a spatially specific presentation.

A direct object may eventually be placed front, side, rear, above/below where HRTF support is credible, or moved through time.

### `BroadSource`

A coherent source with meaningful spatial extent or insufficient evidence for a point position.

Do not collapse broad sources into either point objects or diffuse reverb.

### `DiffuseField`

Ambient/decorrelated/field-like energy whose identity is better represented as directional distribution than a single point.

### `RoomField`

Reflections / late energy belonging to the presentation environment rather than to the identity of a source.

The essential distinction is:

```text
DIRECT OBJECT
≠ BROAD SOURCE
≠ DIFFUSE FIELD
≠ ROOM FIELD
```

---

## 3. Current stereo evidence stage

`renderer::stereo_inference` owns inspectable low-level measurements.

It currently exposes:

- pan;
- phase coherence;
- magnitude asymmetry;
- directness / diffuseness;
- **true complex** mid and side magnitudes;
- total magnitude;
- time-constant-based trajectory state;
- persistence maturity;
- trajectory agreement;
- conservative stability.

`renderer::scene_inference` is the first policy-light synthesis layer.

It currently emits:

- `FrontalAnchor`;
- `LateralObjectCandidate`;
- `BroadSource`;
- `DiffuseField`;
- spatial specificity;
- bass-anchor strength;
- field support;
- object support;
- `reassignment_safety`.

`reassignment_safety` is **not** rear evidence.

Future libaural cues may change the classification or confidence without destroying the underlying measurements.

---

## 4. Bass and groove law

Do not buy spatial size by dissolving the timing and weight of the groove floor.

The current scene-evidence stage uses a smooth bass-anchor prior:

```text
≤ 80 Hz
→ strongly anchored

80–220 Hz
→ continuously decreasing protection

≥ 220 Hz
→ this specific bass prior contributes no protection
```

This curve is a product prior, not a universal law of hearing.

It must remain testable and replaceable.

---

## 5. Lessons assimilated from Steam Audio

Steam Audio is especially useful because it keeps spatial rendering concepts separate.

Omniphony should preserve the following design laws:

### Stateful rendering per source

HRTF convolution, delay/history and movement smoothing need per-source state.

The inherited Omniphony binaural path already follows this direction with per-input-channel DSP state.

### Interpolated HRTFs

Moving spatial objects should not jump between discrete HRTF directions.

The current `HrirSet` already uses bilinear azimuth/elevation interpolation. Preserve that property across future HRTF providers.

### Custom HRTFs are first-class

SOFA/personalized datasets should remain swappable without rewriting scene logic.

### Source rendering and environment rendering are different

Distance, air absorption, directivity, reflections, occlusion/transmission and room energy must not become object identity.

### Fields need spherical representations

Diffuse/ambient energy should not be forced into fake point sources or a fixed 7.1 transport bed.

Ambisonics or another spherical field representation is a strong candidate internal representation for field-like scene entities.

This is a candidate architecture decision to test, not an immediate dependency on Steam Audio.

---

## 6. Lessons assimilated from Dolby PMD / immersive metadata

Dolby's open PMD tooling reinforces a useful separation:

```text
PCM signal
≠ audio element
≠ bed/object organization
≠ presentation
```

Omniphony's equivalent is:

```text
stereo waveform
≠ inferred auditory entity
≠ scene hypothesis
≠ headphone presentation
```

This is load-bearing.

A renderer may alter the presentation without claiming the source waveform itself contained that presentation metadata.

---

## 7. Lessons assimilated from MPEG-I acoustic-scene tooling

Room rendering should eventually make acoustic properties explicit rather than hiding them in a generic `wet` knob.

Useful candidate dimensions include frequency-dependent:

- reflectivity;
- scattering;
- transmission;
- coupling;
- geometric/diffraction context.

These describe propagation/environment behavior.

They do not decide whether a guitar, voice or percussion event is one auditory object.

---

## 8. Fidelity gates inspired by Dolby SATS

Subjective immersion alone cannot pass a build.

Omniphony should develop automatable WAV-based gates for at least:

```text
bypass identity
frequency response
level / loudness parity
dynamic-range preservation
spectrum preservation
noise / residual behavior
THD+N or nonlinear distortion where applicable
transient timing / overshoot
group delay / phase behavior
bass timing and weight
channel/ear peak safety
```

These are **fidelity axes**, not complete quality scores.

A system can measure cleanly and still spatialize badly.

Human listening remains necessary for:

- externalization;
- front/back discrimination;
- elevation plausibility;
- object stability;
- envelopment;
- image depth;
- mix hierarchy;
- fatigue;
- preference.

---

## 9. Two independent validation lanes

A stereo-inference failure must not be confused with a binaural-renderer failure.

### Lane A — known scene → headphones

Use known object/bed scenes, including 7.1.4-style calibration material, to test only:

```text
positions
→ HRTF / ITD
→ reflections / fields
→ headphones
```

This answers:

> If Omniphony is told the correct scene, can it render it convincingly?

### Lane B — stereo master → inferred scene

Hold the binaural renderer fixed and test:

```text
stereo
→ measurements
→ grouping / persistence / musical role
→ scene hypothesis
```

This answers:

> Did Omniphony infer a useful scene without destroying the mix?

Only after both lanes work should the end-to-end system be judged as one product.

---

## 10. Confidence → spatial behavior

The renderer should become more specific only as evidence improves.

```text
low confidence
→ preserve mixture / frontal authority

medium confidence
→ broad or gently displaced source

high confidence
→ spatially specific object eligible for stronger placement
```

No confidence level authorizes arbitrary spectacle.

Musical role and fidelity guards can veto a geometrically dramatic placement.

---

## 11. North-star failure test

Matched-loudness bypass remains the product-level test:

> After acclimation, bypass should feel dimensionally collapsed, while bypass must **not** restore clarity, punch, timbre, transient precision, bass definition, dynamics, or musical hierarchy that Omniphony damaged.

The desired illusion is larger than stereo.

The source recording must remain more important than the illusion.
