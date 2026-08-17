# Source / scene / renderer contract

This document defines the boundary between **source truth**, **presentation state** and **binaural rendering**.

It is subordinate to the root [`README.md`](../README.md).

The governing rule is simple:

> **Rendering may transform a scene. It may not rewrite uncertainty into fake authorship.**

---

## 1. Keep the three layers separate

```text
SOURCE TRUTH / EVIDENCE
What did the source or host actually provide?
        ↓
PRESENTATION STATE
What additional spatial support is defensible?
        ↓
RENDERING
How should that state reach two ears?
```

A failure in one layer must not be hidden by another.

Examples:

- a stereo master can justify bounded spatial presentation without containing literal rear metadata;
- an authored 7.1 bed should remain 7.1 truth instead of being flattened and reconstructed;
- an already-binaural render should not receive a second HRTF stage simply because it is two channels.

---

## 2. Canonical static scene

The product static-scene vocabulary is **8.1.4.4 with 17 semantic lanes**:

```text
L R C LFE Ls Rs Lb Rb Cb Tfl Tfr Tbl Tbr Bfl Bfr Bbl Bbr
```

Every anchor must be able to carry provenance:

```text
AUTHORED
DERIVED
EMPTY
```

The scene is deliberately richer than many inputs. Rich coordinates allow different source classes to converge on one renderer without pretending that absent information was authored.

Dynamic sources with supplied continuous positions belong in a parallel object layer and should not be prematurely snapped to static anchors.

---

## 3. Current stereo mapping

For the current stereo-derived music path, only these canonical lanes receive evidence-backed support:

```text
L R Ls Rs Lb Rb Tfl Tfr Tbl Tbr
```

These remain EMPTY:

```text
C LFE Cb Bfl Bfr Bbl Bbr
```

That is a product invariant, not a temporary naming accident.

Centered vocals, kick authority and other central musical information remain owned primarily by the protected stereo master and coherent foundation path. The support scene exists to enlarge presentation without fabricating a discrete center or LFE channel.

---

## 4. Current render shell

The canonical scene is not the final spatial lattice.

Current expands/renders through the repository's **22-direction System-H-derived shell** before binaural reduction:

```text
canonical 17-lane scene
        ↓
Current support mapping
        ↓
22-direction full-sphere render shell
        ↓
cascaded binaural renderer
        ↓
stereo headphones
```

Keep the distinction explicit:

```text
17 lanes = semantic scene vocabulary
22 directions = internal rendering geometry
```

A 7.1.4 layout or fixture is a useful reference/regression surface, not the Current product base.

---

## 5. Stereo evidence is not recovered metadata

A normal stereo master can expose evidence such as:

- L/R balance and asymmetry;
- inter-channel phase and coherence;
- complex mid/side relationships;
- direct versus diffuse behavior;
- persistence through time;
- onset and transient behavior;
- spectral region;
- trajectory stability.

It generally does **not** expose literal authored rear, overhead or below-listener coordinates for individual sources.

Therefore:

```text
stable stereo evidence
≠ recovered source metadata
```

Rear or elevated support may still be a useful presentation decision. Describe it as presentation.

---

## 6. Rich source truth outranks inference

When the host/source provides stronger geometry, preserve it.

```text
stereo
→ protected master + bounded DERIVED support

5.1 / 7.1
→ matching AUTHORED anchors

height beds
→ matching AUTHORED upper/lower anchors when exposed

object audio
→ continuous AUTHORED positions

Ambisonics / HOA
→ field representation preserved until an appropriate render boundary
```

Wrong:

```text
rich source
→ flatten to stereo
→ infer geometry that was already known
```

A richer source needs less inference, not more.

---

## 7. Scene entity vocabulary

Useful higher-level presentation entities remain distinct from channel provenance.

### `FrontalAnchor`
Material whose relocation would destabilize musical focus, center of gravity or groove floor.

### `DirectObject`
Persistent source-like material for which spatially specific presentation is justified.

### `BroadSource`
Coherent source-like material with meaningful apparent extent or insufficient evidence for a point representation.

### `DiffuseField`
Musical or ambient energy better represented as a distribution than as one point.

### `RoomField`
Environmental energy such as reflections and late reverberation.

Keep:

```text
DirectObject
≠ BroadSource
≠ DiffuseField
≠ RoomField
```

and:

```text
rear direct support
≠ rear reflection
≠ diffuse rear field
```

The renderer should not use reverb as a universal substitute for width, distance or rear placement.

---

## 8. Bass and groove law

Do not buy spatial scale by dissolving timing and weight of the groove floor.

Frequency alone must not imply object identity.

A diffuse low-frequency region may deserve protection without becoming a fake compact bass object. A melodic bass line may require contour and agency preservation rather than generic sub reinforcement.

These are preservation obligations first.

---

## 9. Direct, early and late fields are different jobs

```text
DIRECT
→ direction / HRTF / ITD / distance

EARLY FIELD
→ image timing
→ wall filtering
→ directional binaural reflection cues

LATE FIELD
→ bounded closure / envelopment / decay
```

Current uses a bounded directional early field rather than treating all externalization as generic late reverb.

The early path groups first-order contributions into six directional reflection buses before measured-HRTF rendering. This keeps directional evidence while avoiding a combinatorial bank of full HRTF convolvers.

---

## 10. Research obligations

Peer-reviewed binaural literature supports treating the following as separate validation axes:

### Externalization
Frontal and rear sources are common failure cases. Room-related binaural cues and head motion can improve out-of-head perception.

### Interaural behavior
Externalization can correlate strongly with binaural cue structure, including interaural coherence and the interaction between direct and reflected cues.

### Timbre
Spatial processing can damage coloration even when localization appears plausible. HRTF/HRIR handling should therefore be validated spectrally as well as geometrically.

### Motion
A world-stable scene during head motion can improve externalization. Head tracking is therefore a meaningful future lever, not merely a UI feature.

Representative anchors:

- Zaunschirm, Schörkhuber & Höldrich (2018), DOI `10.1121/1.5040489`
- Catic, Santurette & Dau (2015), DOI `10.1121/1.4928132`
- Leclère, Lavandier & Perrin (2019), DOI `10.1121/1.5128325`
- Brimijoin, Boyd & Akeroyd (2013), DOI `10.1371/journal.pone.0083068`
- Hendrickx et al. (2017), DOI `10.1121/1.4978612`

These findings motivate tests. They do not automatically validate a particular Current tuning.

---

## 11. Current implementation status

Confirmed Current architecture includes:

- 17-lane canonical 8.1.4.4 scene order;
- explicit stereo EMPTY-lane preservation;
- 22-direction Current render shell;
- cascaded binaural output;
- measured/parametric/SOFA-capable HRTF infrastructure;
- ITD and distance handling;
- directional early-field machinery;
- callback/motion regression work;
- protected master and coherent foundation paths;
- deterministic Current geometry tests.

The Windows endpoint APO currently accepts **stereo float32** for Current. Native authored 5.1/7.1 APO ingress remains a separate host frontier.

---

## 12. Validation lanes

Do not collapse every problem into one end-to-end score.

### Known-scene lane

```text
known source geometry
→ canonical scene
→ renderer
→ headphones
```

Tests renderer correctness without stereo inference uncertainty.

### Stereo-presentation lane

```text
controlled stereo
→ evidence
→ bounded DERIVED scene support
→ renderer
```

Tests inference/presentation permission.

### Rich-input lane

```text
known multichannel / object source
→ preserve AUTHORED geometry
→ renderer
```

Tests source authority and host ingress.

### Product lane

```text
ordinary Windows playback
→ endpoint host
→ Current
→ headphones
```

Tests integration, reliability and listening quality.

---

## 13. Objective and listening gates

Useful engineering axes include:

- exact scene order;
- provenance preservation;
- EMPTY-lane silence;
- shell direction count;
- finite stereo output;
- peak/RMS and headroom;
- residual/null where identity is expected;
- frequency response and coloration;
- lag/ITD and ILD behavior;
- interaural coherence where relevant;
- transient timing;
- bass timing/coherence;
- block-size invariance;
- motion continuity;
- state-switch continuity.

Human listening remains necessary for:

- externalization;
- front/back discrimination;
- elevation and below-listener perception;
- source body/extent;
- envelopment;
- radial depth;
- room naturalness;
- direct-source solidity;
- center authority;
- bass/groove integrity;
- fatigue;
- preference.

---

## 14. Research trigger rule

```text
controlled test or listening exposes a weakness
→ identify the missing obligation
→ inspect research and implementations
→ isolate one candidate mechanism
→ measure + listen
→ keep only if earned
```

Research sophistication does not outrank the protected sound.

---

## 15. North-star failure test

At matched loudness, bypass should eventually feel spatially collapsed.

Bypass must not restore clarity, punch, timbre, transient precision, bass definition, dynamics, center authority or musical hierarchy.

The scene exists to strengthen the headphone world. The recording remains more important than the scene model.
