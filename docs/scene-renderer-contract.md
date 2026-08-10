# Source / scene → binaural renderer contract

This document defines the boundary between source truth, optional presentation hypotheses, and binaural rendering.

It is subordinate to the root `README.md`.

The portable product does **not** require a complete artificial scene model before it can exist. Upstream Omniphony already provides a useful binaural renderer; stereo inference is one presentation problem among several input classes.

Keep three questions separate:

```text
1. What source truth/evidence actually exists?
2. What additional presentation state is defensible?
3. How should that state be rendered to two ears?
```

The third question must never rewrite the first two into fake certainty.

---

## 1. Renderer comes after source truth

Correct high-level chain:

```text
logical source streams
        ↓
authoritative layout/object metadata where present
        ↓
optional conservative presentation state where truth is missing
        ↓
Omniphony renderer
        ↓
binaural stereo
```

For a stereo master, presentation may infer useful spatial support.

For a real 7.1/height/object source, the renderer should preserve the authored information instead of reconstructing it from stereo.

---

## 2. Layout is stream-local

The renderer must be able to receive more than one logical input stream with different layouts.

Example:

```text
Stream A
  stereo music

Stream B
  7.1 game

Stream C
  mono voice/chat
```

These may coexist.

Do not define a global scene input mode such as:

```text
current_layout = stereo | 7.1
```

Starting a surround source must not reinterpret unrelated stereo material.

The platform host may temporarily collapse sources into one mixed bed. That is a host limitation, not the desired core model.

---

## 3. Stereo does not contain rear ground truth

A normal stereo master can provide evidence about:

- L/R balance;
- inter-channel phase/coherence;
- complex M/S structure;
- direct versus diffuse behavior;
- persistence through time;
- spectral region;
- onset/offset relations;
- masking/audibility where measured;
- optional later object/role evidence.

It generally does **not** reveal a literal authored rear azimuth for an inferred source.

Therefore:

```text
stable stereo evidence
≠
recovered rear metadata
```

Rear placement can still be a useful immersive presentation decision. Describe it as presentation, not forensic recovery.

---

## 4. Rich inputs contain stronger truth

When the source provides:

```text
5.1
7.1
height beds
objects
Ambisonics / HOA
```

those authored relations should survive into the scene/renderer contract.

Correct:

```text
rich source
→ layout/object adapter
→ renderer
```

Wrong:

```text
rich source
→ flatten to stereo
→ stereo inference
→ try to recover what was discarded
```

A richer source generally needs **less inference**, not more.

---

## 5. Scene entity vocabulary

Keep renderer vocabulary small and typed.

### `FrontalAnchor`
Material whose relocation would destabilize center of gravity, groove floor or musical focus.

### `DirectObject`
Persistent source-like material for which spatially specific presentation is justified.

### `BroadSource`
Coherent source-like material with meaningful apparent extent or insufficient evidence for a point representation.

### `DiffuseField`
Musical/ambient energy better represented as a distribution than as one point.

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

These distinctions prevent easy-but-wrong substitutions such as adding more reverb whenever something should feel broad or behind the listener.

---

## 6. Current stereo evidence

Current local work already contains useful low-level evidence:

- pan/lateral relation;
- phase coherence;
- magnitude asymmetry;
- directness/diffuseness;
- complex mid/side relation;
- persistence/maturity;
- trajectory agreement/stability;
- conservative object/field support;
- low-frequency/foundation protection.

Hard law:

> **Measurements remain measurements.**

A feature does not become an instrument identity, authored object or placement command merely because it is stable.

Future libaural state may enrich classification/confidence. It is not required for baseline operation.

---

## 7. Bass and groove law

Do not buy spatial scale by dissolving timing and weight of the groove floor.

Frequency alone must not imply object identity.

A diffuse low-frequency field may deserve protection without being mislabeled as a compact bass object.

A melodic bass line may need contour/agency preserved rather than generic sub reinforcement.

These are preservation laws first.

---

## 8. Rendering laws retained from research

### Stateful rendering per source
HRTF convolution, delay history and movement smoothing need persistent state.

### Interpolated HRTFs
Movement should not jump between discrete HRIR directions.

### Swappable HRTF providers
Measured generic, parametric and SOFA/custom sources can change without rewriting scene semantics.

### Bulk delay and HRTF spectral phase are distinct
When analytic ITD is separate, measured-HRIR preprocessing must not erase useful ear-specific spectral phase merely to force waveform alignment.

### Source and environment rendering are distinct
Distance, air absorption, reflections and room energy do not become source identity.

### Fields may need distributed representation
Diffuse musical material should not automatically become random point sources or a fixed fake speaker bed.

These are mechanism constraints, not a requirement to import another renderer wholesale.

---

## 9. Room / externalization architecture

Useful split:

```text
DIRECT
→ direction / ITD / HRTF

EARLY ROOM
→ image-source geometry
→ directional per-ear timing/filtering

LATE ROOM
→ distributed late field / FDN behavior
```

This is better than one generic `spaciousness` control because the perceptual jobs are different.

The first live arbitrary-audio prototype was reported as tinny/hallway-like, but that result was route-contaminated by a likely duplicate physical path and unclean bypass. Do not retune room DSP around that observation until the route is clean.

---

## 10. Known-scene truth remains valuable

Known geometry isolates renderer quality from inference quality.

### Renderer lane

```text
known scene / layout
→ protected Omniphony renderer
→ headphones
```

### Stereo-presentation lane

```text
controlled stereo
→ evidence
→ bounded presentation hypothesis
→ renderer
```

### Rich-input lane

```text
known 5.1 / 7.1 / height / object source
→ preserve source truth
→ renderer
```

### Mixed-stream lane

```text
stereo source
+
surround source
→ independent logical layouts
→ one binaural world/output
```

### Product lane

```text
ordinary real use
→ platform host
→ Omniphony
↔ incumbent
```

Do not collapse these lanes into one score.

---

## 11. Confidence controls specificity

Useful policy shape:

```text
low confidence
→ preserve mix / protected relationships

medium confidence
→ broad, modest, reversible change

high confidence
→ greater permission for specific stable placement
```

Confidence does not authorize spectacle.
Musical/fidelity guards can veto a dramatic placement.

---

## 12. Current confirmed renderer state

Retained/fork behavior includes:

- stateful binaural HRTF/ITD rendering;
- measured/parametric/SOFA-capable HRTF providers;
- directional interpolation/crossfades;
- stale asynchronous HRIR rebuild rejection;
- measured-HRIR direct-arrival validation;
- per-ear directional early-reflection timing;
- early reflection and late FDN machinery;
- sample-time-oriented FDN modulation;
- stereo evidence/persistence modules;
- bass/foundation protection;
- deterministic DSP/fidelity fixtures;
- optional upstream spectral-phantom and distance-diffuse mechanisms.

The native Windows app now also proves arbitrary real audio can reach this engine and the physical headphones.

That proves transport viability, not yet clean product-quality sound.

---

## 13. Current gaps that matter

### A. Clean single-path product listening
Before tuning, prove no old incumbent forwarding reaches the physical output beside Omniphony.

### B. Clean bypass
Current prototype can leave already-queued wet-selected data after OFF. Selection must move closer to physical output or otherwise invalidate stale wet history.

### C. Ordinary stereo presentation
The evidence modules exist, but normal stereo is not yet the mature full-sphere world described by the north star.

### D. Multi-stream core contract
The desired core is stream-local and supports simultaneous layouts. Current Windows loopback prototype sees a platform mix rather than ideal independent streams.

### E. Position/HRTF movement
Prefer one authoritative sample-time trajectory rather than multiple hidden motion authorities.

### F. Source extent
A mature headphone `BroadSource` primitive still needs controlled listening/measurement.

### G. `DiffuseField`
Room FDN is not automatically a musical diffuse-field renderer.

---

## 14. Fidelity gates

Useful objective axes include:

- strict residual/null where identity is expected;
- peak/RMS;
- crest factor;
- DC;
- frequency response;
- lag/ITD;
- transient timing;
- bass timing/coherence;
- callback-size invariance where intended;
- state-switch continuity;
- clipping/headroom;
- bypass queue cleanliness.

Human listening remains necessary for:

- externalization;
- front/back discrimination;
- elevation/below;
- image stability;
- source body/extent;
- envelopment;
- radial depth;
- room naturalness;
- direct-source solidity;
- bass/groove integrity;
- fatigue;
- preference.

A duplicate physical route invalidates subtle listening evidence.

---

## 15. Research trigger rule

Use:

```text
clean controlled test / real listening exposes weakness
→ identify missing capability
→ inspect research / implementations
→ isolate candidate
→ measure + listen
→ keep only if earned
```

External engines, Ambisonics, libaural and learned models remain excellent sources when a concrete problem calls for them.

They do not outrank the protected sound merely by being sophisticated.

---

## 16. North-star failure test

At matched loudness, Omniphony should eventually make bypass feel spatially collapsed.

Bypass must not restore clarity, punch, timbre, transient precision, bass definition, dynamics, center authority or musical hierarchy.

And before applying that test:

```text
one physical route
+
clean OFF
```

must already be established.

The scene is there to strengthen the headphone world.
The recording remains more important than the scene model.
