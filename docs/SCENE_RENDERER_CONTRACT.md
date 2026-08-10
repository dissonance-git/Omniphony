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

`object_separation` is now symmetric around the neutral midpoint rather than silently biasing every uncertain bin toward object-like/direct.

`renderer::scene_inference` is the first policy-light synthesis layer.

It currently emits:

- `FrontalAnchor`;
- `LateralObjectCandidate`;
- `BroadSource`;
- `DiffuseField`;
- spatial specificity;
- bass-protection strength;
- coherent-foundation support;
- field support;
- object support;
- `reassignment_safety`.

Low-frequency protection is deliberately separate from object identity. A diffuse low-frequency field can be protected from aggressive movement without being mislabeled as a coherent bass object.

`reassignment_safety` is **not** rear evidence.

Future libaural cues may change the classification or confidence without destroying the underlying measurements.

---

## 4. Bass and groove law

Do not buy spatial size by dissolving the timing and weight of the groove floor.

The current scene-evidence stage uses a smooth low-frequency protection prior:

```text
≤ 80 Hz
→ strongly protected

80–220 Hz
→ continuously decreasing protection

≥ 220 Hz
→ this specific bass prior contributes no protection
```

Frequency alone does not establish `FrontalAnchor` identity. The current `foundation_support` additionally requires persistent direct/source-like evidence and is suppressed by field-like evidence.

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

The current `HrirSet` already uses directional interpolation. Preserve smooth filter evolution across future HRTF providers.

### Custom HRTFs are first-class

SOFA/personalized datasets should remain swappable without rewriting scene logic.

The asynchronous HRTF rebuild path now tags completed grids with the request that produced them and rejects stale late completions. Rapid future calibration/A-B switching therefore cannot install an obsolete HRTF merely because its worker finished later.

### Bulk arrival delay and spectral phase are different

Measured HRIR preprocessing must remove bulk/direct-arrival offsets when analytic ITD is applied separately, while retaining direction-dependent HRTF spectral phase.

A left/right cross-correlation peak at zero is **not** the correct time-alignment contract for two spectrally different ear filters.

The active validation gate now checks direct-arrival alignment rather than forcing the measured HRTF toward identical ear phase histories.

### Source rendering and environment rendering are different

Distance, air absorption, directivity, reflections, occlusion/transmission and room energy must not become object identity.

### Fields need spherical representations

Diffuse/ambient energy should not be forced into fake point sources or a fixed 7.1 transport bed.

Ambisonics or another spherical field representation is a strong candidate internal representation for field-like scene entities.

This is a candidate architecture decision to test, not an immediate dependency on Steam Audio.

---

## 6. Current room / externalization path

The current binaural room path is intentionally split:

```text
DIRECT OBJECT
→ analytic per-ear ITD
→ interpolated HRIR convolution

EARLY ROOM
→ six first-order image sources
→ relative geometric propagation delay
→ per-reflection directional ITD
→ broadband ILD

LATE ROOM FIELD
→ shared send bus
→ 8-line FDN
→ frequency-dependent interaural-coherence shaping
→ stereo tail
```

This is much closer to a useful externalization architecture than one generic reverb effect.

### Early reflections

The reflection bank now carries independent left/right arrival times. Each image source uses its own head-relative azimuth/elevation and the same analytic ear-delay model as the direct path.

The cheap early layer still does **not** run a complete HRTF convolution for every reflection. That is intentional until listening tests show the extra CPU buys enough externalization/front-back/elevation value.

### Late field

The FDN uses:

- mutually orthogonal zero-sum output tap patterns;
- slow mutually detuned delay modulation;
- high-frequency damping;
- low-frequency shared/coherent return;
- higher-frequency decorrelated return;
- distance-related send rather than destructive direct-level attenuation.

Its internal timing is now sample-based rather than host-block-based:

```text
0 ms predelay
→ actually zero

modulation update horizon
→ persists across process_block calls
→ same room trajectory for 40-sample, 1024-sample, or whole-buffer processing
```

Block size is not allowed to change the simulated room.

---

## 7. Lessons assimilated from Dolby PMD / immersive metadata

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

## 8. Lessons assimilated from MPEG-I acoustic-scene tooling

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

## 9. Fidelity gates inspired by Dolby SATS

Subjective immersion alone cannot pass a build.

The shared DSP-fixture layer now exposes known-answer measurements for:

- strict peak residual/null level;
- RMS residual;
- peak dBFS;
- RMS dBFS;
- crest factor;
- DC offset;
- matched RMS-level delta;
- FFT/frequency-response analysis;
- lag/ITD analysis with ambiguity detection.

The Windows renderer-core CI lane now runs the measurement fixture crate's own tests before renderer tests, so a broken ruler cannot silently certify the DSP being measured.

Further automatable WAV-based gates should cover:

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

## 10. Two independent validation lanes

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

## 11. Confidence → spatial behavior

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

## 12. Current implementation defects / missing bridges

These are concrete implementation gaps, not speculative wishlist items.

### A. Stereo inference is not yet in the realtime audio path

`stereo_inference` and `scene_inference` are real tested renderer modules, but ordinary two-channel playback is not yet feeding FFT/band evidence through them into live object/field state.

Do not claim end-to-end stereo → objects yet.

### B. Binaural object gain slew is still block-quantized

`ChannelState::slew_gain` produces a sample-accurate `(start, per_sample_step)` pair for the speaker path. The binaural branch currently advances the same state but passes only the block-end gain into `BinauralRenderer`, which applies that value to the whole block.

Consequences:

```text
40-sample live blocks
→ fine staircase

large offline/device blocks
→ much coarser gain transition
```

This should be fixed by carrying the gain ramp into the binaural per-sample loop, while keeping `ChannelState` as the single gain-state authority so speaker ↔ headphone switching cannot drift.

Do not create a second independent gain envelope inside `BinauralRenderer`.

### C. Binaural rendering currently collapses object extent to a point

The inherited speaker/VBAP path preserves object `size` and can convert spatial extent into spread. The binaural branch currently forwards position and gain but not the object's ramped size/extent.

Therefore a future `BroadSource` cannot yet be rendered as a true broad source in headphone mode.

The fix should reuse the existing object-size state rather than invent a parallel width parameter.

### D. `DiffuseField` has no first-class spherical direct renderer yet

The current FDN is a **room field**, not a substitute for diffuse musical content.

A texture classified as `DiffuseField` still needs an internal spherical/extended representation, likely Ambisonic or an experimentally equivalent field basis, before binaural decoding.

Do not fake this by assigning random point objects around the listener.

### E. Realtime performance remains report-only

The inherited worst-case block-time test intentionally reports p99.9 but does not assert because raw wall-clock timing changes dramatically under CI contention. Upstream documented same-scene timings ranging from ~4 % of the block budget when idle to >100 % under synthetic host load.

Do not invent a fixed threshold.

A real gate needs a same-run calibration/control workload or another load-normalized metric before performance can become pass/fail.

### F. CI result is not yet observable through the current connector

The workflow is configured to run formatter checks, DSP-fixture tests, renderer-core tests and the Windows ASIO/listening build, but the available GitHub connector currently exposes no push-run status/check record for these commits.

Until an actual run result is observed, the current checkpoint is:

```text
code committed
+ diffs audited
+ tests added
≠ verified green CI
```

This is also why the large physical upstream-deletion/contraction pass remains intentionally paused.

---

## 13. Contraction gate

Do not begin broad inherited-code deletion merely because the new architecture is clearer.

Required order:

```text
current renderer-core checkpoint
→ green compiler/tests
→ green Windows listening artifact
→ freeze baseline
→ map retained dependency graph
→ remove one upstream product layer
→ compile/test
→ repeat
```

Keep:

- the mature binaural/HRTF substrate;
- known-scene speaker/VBAP machinery needed for calibration or shared renderer internals;
- room/reflection code that survives listening/measurement tests;
- deterministic DSP fixtures;
- Windows audio integration needed by the final product.

Remove only after proof of irrelevance:

- broad Studio/UI product surfaces;
- mpv-oriented distribution surfaces not needed by the final Windows listener;
- unrelated cross-platform packaging;
- compatibility machinery whose retained dependency graph is empty.

---

## 14. North-star failure test

Matched-loudness bypass remains the product-level test:

> After acclimation, bypass should feel dimensionally collapsed, while bypass must **not** restore clarity, punch, timbre, transient precision, bass definition, dynamics, or musical hierarchy that Omniphony damaged.

The desired illusion is larger than stereo.

The source recording must remain more important than the illusion.
