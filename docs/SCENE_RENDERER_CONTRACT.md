# Stereo scene → binaural renderer contract

This document defines the boundary between stereo evidence, optional scene hypotheses, and binaural rendering.

It is subordinate to the root `README.md`.

The native Windows product does **not** require a complete artificial scene model before it can exist. Upstream Omniphony already provides a useful binaural renderer; scene inference is a later mechanism for making ordinary stereo presentation more intelligent once the native listening lane is easy to test.

The three questions remain separate:

```text
1. What acoustic evidence exists in the stereo signal?
2. What scene organization is a defensible presentation hypothesis?
3. How should the resulting state be rendered to two ears?
```

The third question must never rewrite the first two into fake certainty.

---

## 1. Protected renderer comes first

The renderer has a perceptual ancestor independent of future stereo inference:

```text
omniphony-renderer/assets/binaural-baselines/upstream-demo-reference.yaml
```

A scene-inference experiment that makes the product sound worse than the protected renderer at matched loudness does not earn the default.

The useful architecture is therefore:

```text
protected Omniphony renderer
        ↑
small validated scene/presentation state
        ↑
current local stereo evidence
        ↑
optional richer evidence later
```

not:

```text
no complete scene model
→ no useful Omniphony
```

---

## 2. Stereo does not contain rear ground truth

A normal stereo master can provide evidence about:

- L/R balance;
- inter-channel phase/coherence;
- complex M/S structure;
- direct versus diffuse behavior;
- persistence through time;
- spectral region;
- onset/offset relations;
- masking/audibility where measured;
- later optional object/role evidence.

It generally does **not** reveal a literal authored rear azimuth for an inferred source.

Therefore:

```text
stable stereo evidence
≠
recovered rear metadata
```

Rear placement can still be a useful immersive presentation decision.

It must be described as presentation, not forensic recovery.

---

## 3. Scene entity vocabulary

Keep the renderer vocabulary small and typed.

### `FrontalAnchor`

Material whose relocation would destabilize the recording's center of gravity, groove floor or musical focus.

### `DirectObject`

Persistent source-like material for which a spatially specific presentation is justified.

### `BroadSource`

Coherent source-like material with meaningful apparent extent or insufficient evidence for a point representation.

### `DiffuseField`

Musical/ambient energy better represented as a directional distribution than as one point.

### `RoomField`

Presentation-environment energy such as reflections and late reverberation.

Keep:

```text
DirectObject
≠ BroadSource
≠ DiffuseField
≠ RoomField
```

and:

```text
rear direct object
≠ rear reflection
≠ diffuse rear field
```

These distinctions prevent easy-but-wrong substitutions such as using more reverb whenever a source should feel broad or behind the listener.

---

## 4. Current stereo evidence

Current local renderer work already contains useful low-level evidence rather than waiting for general artificial hearing.

Relevant measures include:

- pan/lateral relation;
- phase coherence;
- magnitude asymmetry;
- directness/diffuseness;
- true complex mid/side relation;
- persistence/maturity;
- trajectory agreement/stability;
- conservative object/field support;
- low-frequency/foundation protection.

The important law is:

> **Measurements remain measurements.**

A low-level feature does not become an instrument identity, authored object, or placement command merely because it is stable.

Future `libaural` state may enrich classification/confidence. It does not own this evidence layer and is not required for baseline operation.

---

## 5. Bass and groove law

Do not buy spatial scale by dissolving the timing and weight of the groove floor.

Current low-frequency safeguards are product priors, not universal psychoacoustic laws.

Use them conservatively to protect foundation behavior while better musical evidence develops.

Frequency alone must not imply object identity.

A diffuse low-frequency field may deserve protection without being mislabeled as a compact bass object.

---

## 6. Rendering laws retained from research

Several ideas from Steam Audio, Dolby tooling, Ambisonic systems and other research remain useful because they clarify renderer jobs.

### Stateful rendering per source

HRTF convolution, delay history and movement smoothing need persistent state.

### Interpolated HRTFs

Movement should not jump between discrete HRIR directions.

### HRTF providers remain swappable

Measured generic, parametric and SOFA/custom sources can be changed without rewriting scene semantics.

### Bulk arrival delay and HRTF spectral phase are distinct

When analytic ITD is applied separately, measured HRIR preprocessing must not erase useful ear-specific spectral phase merely to force left/right filters into the same waveform alignment.

### Source rendering and environment rendering are distinct

Distance, air absorption, reflections, occlusion/transmission and room energy do not become source identity.

### Fields may need distributed/spherical representation

Diffuse or ambient musical material should not automatically become random point sources or a fixed fake speaker bed.

These are candidate mechanism constraints, not a requirement to import another renderer wholesale.

---

## 7. Current room / externalization architecture

The useful conceptual split is:

```text
DIRECT
→ direction / ITD / HRTF

EARLY ROOM
→ image-source geometry
→ directional per-ear timing / filtering

LATE ROOM
→ distributed late field / FDN behavior
```

This is better than one generic `spaciousness` or `wet` control because the perceptual jobs are different.

The fork has already improved several pieces of this machinery, including directional early-reflection timing and sample-time-oriented FDN behavior.

The existence of those improvements does not promote `baseline-room.yaml` above the upstream demo reference.

Late room remains optional presentation behavior.

---

## 8. Known-scene truth remains valuable

Known geometry isolates renderer quality from inference quality.

### Renderer lane

```text
known scene / layout
→ protected Omniphony renderer
→ headphones
```

asks:

> If Omniphony is told the scene, does it render it convincingly?

### Inference lane

```text
controlled stereo
→ evidence
→ scene hypothesis
```

asks:

> Did Omniphony infer useful organization without inventing unsupported specificity?

### Product lane

```text
ordinary music
→ complete native Omniphony route
↔ current HeSuVi incumbent
```

asks:

> Would the listener actually prefer the new product?

Do not collapse these lanes into one score.

---

## 9. Confidence controls specificity

A useful policy shape remains:

```text
low confidence
→ preserve mix / protected relationships

medium confidence
→ broad, modest, reversible spatial change

high confidence
→ greater permission for specific stable placement
```

Confidence does not authorize spectacle.

Musical/fidelity guards can veto a geometrically dramatic placement.

---

## 10. Current confirmed renderer state

At the August 2026 checkpoint, useful retained/fork behavior includes:

- stateful binaural HRTF/ITD rendering;
- measured/parametric/SOFA-capable HRTF providers;
- directional interpolation/crossfades;
- stale asynchronous HRIR rebuild rejection;
- measured-HRIR direct-arrival validation;
- per-ear directional early-reflection timing;
- early reflection and late FDN room machinery;
- sample-time-oriented FDN modulation;
- stereo evidence/persistence modules;
- bass/foundation protection;
- deterministic DSP/fidelity fixtures;
- optional upstream spectral-phantom and distance-diffuse mechanisms already present in the fork.

The Windows renderer/core Actions path after the August host-native test repair was visually verified green by the repository owner on 2026-08-10.

Do not keep old language saying that checkpoint is still waiting for CI proof.

---

## 11. Current gaps that still matter

These are real gaps, but the root README decides when they outrank Windows host work.

### A. Ordinary stereo is not yet a complete persistent realtime scene

The evidence modules exist, but normal playback is not yet producing the mature persistent object/field world described by the long-term model.

Do not claim end-to-end stereo object recovery.

### B. Position/HRTF movement still needs one authoritative sample-time trajectory

The gain/callback-invariance work and green CI do **not** prove that all movement is sample-time invariant.

The known separate concern is position/HRTF publication that can still be shaped by block-start updates.

When this is fixed, prefer:

```text
one scene position trajectory
→ all renderer consumers
```

rather than a second hidden motion authority inside the HRTF stage.

### C. Source extent is not yet a fully proven binaural primitive

Object size/extent exists in inherited state, but a mature headphone `BroadSource` behavior still needs controlled listening/measurement before it becomes a product assumption.

Do not fake extent by indiscriminate decorrelation.

### D. `DiffuseField` is not a first-class musical field renderer yet

The FDN is a room field.

It is not automatically the correct representation for diffuse musical content.

If a distributed musical field is needed, test a real spherical/extended basis separately.

### E. Realtime performance needs meaningful controls

Raw wall-clock CI timing is too sensitive to host contention for a naive fixed threshold.

Use same-run controls or another normalized metric before turning performance reports into hard pass/fail gates.

### F. Native Windows transport is now in progress

Current host work includes:

```text
windows_host
→ normal Windows output-device probe
→ self-excluding process-loopback diagnostic probe

realtime_ffi
→ bit-exact interleaved-f32 PCM boundary
→ C ABI/header
→ CI/package coverage
```

Loopback remains diagnostic because it copies rather than intercepts the system mix.

The next transport problem is single-path integration, likely by evaluating endpoint APO and virtual-endpoint strategies.

See `docs/WINDOWS_AUDIO_ROUTE.md`.

---

## 12. Fidelity gates

Spatial quality alone cannot pass a build.

Useful objective axes include:

- strict residual/null where identity is expected;
- peak and RMS level;
- crest factor;
- DC;
- frequency response;
- lag/ITD;
- transient timing;
- bass timing/coherence;
- callback-size invariance where the behavior should be invariant;
- profile/renderer state-switch continuity;
- clipping/headroom.

Human listening remains necessary for:

- externalization;
- front/back discrimination;
- elevation;
- image stability;
- source body/extent;
- listener envelopment;
- radial depth;
- room naturalness;
- timbral/direct-source solidity;
- bass/groove integrity;
- fatigue;
- preference.

The desired bypass result is dimensional collapse, not restored fidelity.

---

## 13. Research trigger rule

Do not make the scene contract a reason for endless broad research.

Use:

```text
controlled test / real listening exposes weakness
→ identify missing renderer or evidence capability
→ inspect research / existing implementations
→ isolate candidate
→ measure + listen
→ keep only if earned
```

Steam Audio, Dolby, Ambisonics, libaural and learned models remain excellent sources when a concrete problem calls for them.

They do not outrank the protected sound merely by being sophisticated.

---

## 14. North-star failure test

At matched loudness, Omniphony should eventually make bypass feel spatially collapsed.

But bypass must not restore:

- clarity;
- punch;
- timbre;
- transient precision;
- bass definition;
- dynamics;
- center authority;
- musical hierarchy.

The scene is there to strengthen the headphone world.

The recording remains more important than the scene model.