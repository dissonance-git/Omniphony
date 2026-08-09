# Headphone Rendering Research

> Research and engineering direction for headphone playback in `dissonancehelix/Omniphony`.
>
> **Goal:** make the standard Omniphony headphone renderer capable of presenting ordinary audio as a stable, externalized, spatially coherent acoustic scene while preserving musical identity, clarity, timbre, transients, dynamics, and source intent.

This is not a branded listening mode and is not intended to remain a separate product path. It is the research surface for improving **base Omniphony**. A technique that survives controlled tests should graduate into the normal binaural renderer or the normal upstream scene-analysis path. Experimental names, presets, and parallel renderers should disappear once the underlying behavior is proven.

The long-term product target is simple from the listener's perspective:

```text
application / player / game
          ↓
       Omniphony
          ↓
standard headphone rendering
          ↓
       headphones
```

The user should not need to understand HRTFs, Ambisonics, early-reflection geometry, auditory scene analysis, or headphone compensation. Complexity belongs inside the engine.

---

## 1. Core thesis

Conventional stereo headphone playback is a severe representational bottleneck. A recording may contain evidence about foreground and background, source grouping, source width, direct and diffuse energy, reverberant structure, motion, mix hierarchy, temporal identity, and apparent distance, but ordinary playback presents that organization primarily as two ear-coupled signals.

The research question is whether Omniphony can re-expand the surviving evidence into a coherent three-dimensional percept without damaging the source.

A useful conceptual model is a **latent spatial scene**:

```text
performances / tracks / rooms / effects / mix hierarchy
                         ↓
                  stereo projection
                         ↓
                       L / R

playback reconstruction:

                       L / R
                         ↓
             auditory organization
                         ↓
          confidence-aware scene model
                         ↓
              binaural acoustics
                         ↓
                    headphones
```

The latent scene is not claimed to be a uniquely recoverable historical object. Stereo does not tell us the original three-dimensional coordinates of every musical element. The task is therefore constrained reconstruction:

1. preserve structure that is explicitly available;
2. infer structure where the signal supports it;
3. represent uncertainty instead of inventing false precision;
4. use conservative spatial authoring where multiple interpretations remain possible.

The eventual experience should feel less like sound emitted by virtual loudspeakers and more like a musical event occupying acoustic space.

---

## 2. Successful research graduates into core

The research path exists to change Omniphony itself.

```text
current Omniphony
      ↓
controlled renderer / perception experiments
      ↓
objective + listening validation
      ↓
surviving method
      ↓
base Omniphony behavior
```

Do not preserve an "experimental premium mode" merely because a technique began as an experiment. If directional early reflections are demonstrably better, they should become the normal early-reflection implementation. If a scene representation is demonstrably better, it should become the normal scene representation.

The same applies in the other direction: a candidate that does not survive matched, repeatable tests should be removed or retained only as a diagnostic control.

---

## 3. Non-negotiable constraints

### 3.1 Clarity-preserving dimensionality

Increase spatial dimensionality while keeping blur near zero.

```text
maximize:
  externalization
  azimuth / rear discrimination
  radial depth
  elevation
  apparent source extent
  listener envelopment
  room scale
  source separation
  spatial persistence
  ambient continuity

subject to bounded change in:
  timbre
  transient shape
  direct-source coherence
  bass stability
  integrated loudness
  spectral balance
  phase / group delay
  source identity
  fatigue
```

No amount of spatial scale compensates for softened attacks, watery cymbals, wandering vocals, unstable bass, or obvious comb coloration.

### 3.2 Cue agreement

A believable source is not just a coordinate. Direction, distance, apparent width, direct/reverberant ratio, early reflections, spectral cues, and room response should describe the same acoustic event.

Contradictory cues produce an effect. Consistent cues produce a scene.

### 3.3 Spatial specificity follows confidence

```text
high confidence   → precise persistent direct object
medium confidence → bounded spatial region / larger extent
low confidence    → diffuse or ambient field
no evidence       → do not invent unnecessary precision
```

### 3.4 Spatial persistence

Object identity and position should not be recomputed as a new universe every analysis frame. Maintain a scene hypothesis and change it only when new evidence is strong enough.

### 3.5 One binaural transform

Controlled tests terminate at Omniphony's binaural stereo output. Do not feed that result through Windows Spatial Audio, HeSuVi, another HRTF virtualizer, or a game-side headphone HRTF during evaluation.

Double virtualization is an invalid comparison condition.

---

## 4. Perceptual dimensions must remain independent

Spatial reproduction is not one scalar called "spaciousness". At minimum, distinguish:

- localization / angular precision;
- externalization;
- auditory distance;
- elevation;
- apparent source width (ASW);
- listener envelopment (LEV);
- source extent;
- room presence / scale;
- direct-source clarity;
- diffuse-field continuity.

### 4.1 Apparent source width is not listener envelopment

Room-acoustics and precedence-effect research distinguishes at least two important spatial-impression components.

**Apparent source width (ASW)** is the perceived width of the source image that remains fused with the direct sound.

**Listener envelopment (LEV)** is the sense of surrounding room or sound-field energy that is perceptually distinct from the direct source image.

This distinction is load-bearing for headphone rendering.

```text
direct source
   + fused early directional energy
          ↓
     source width / body

later + distributed room energy
          ↓
      envelopment / room
```

Do not enlarge every source in order to make the room larger. Do not collapse the room onto the source in order to make the source wider.

### 4.2 Precedence-effect boundary

Early reflections can strengthen source width, room evidence, and externalization while the first-arriving source retains localization dominance. The exact fusion/echo boundary depends on signal content, level, direction, repetition, adaptation, and delay, so it must not be encoded as one universal millisecond threshold.

The renderer should treat reflection timing and level as psychoacoustic variables, not merely geometric outputs.

### 4.3 Preserve temporal envelopes

Research on concert-hall perception indicates that wideband early reflections that preserve the source's temporal envelope can contribute to clear, open sound, whereas reflections that damage that envelope can weaken clarity and produce muddiness.

For Omniphony this creates an explicit acceptance criterion for early-reflection work:

- increase useful spatial evidence;
- preserve onset and amplitude-envelope integrity;
- avoid audible doubling;
- avoid frequency-selective comb coloration that changes source identity.

### 4.4 Interaural correlation is a diagnostic, not a target knob

Interaural cross-correlation / coherence is strongly related to spatial impression, but different percepts depend on it differently. ASW can be related to band-limited interaural correlation, while LEV also depends on the spatial distribution of late energy, including front/back/vertical distribution.

Therefore the engine should measure interaural coherence by band, but should not maximize decorrelation blindly.

---

## 5. Preserve truth; infer only absence

Long-term, all source types should enter a canonical scene representation with provenance describing what is known versus inferred.

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

Available source structure increases upward. Required inference increases downward.

For the current stage, **ordinary stereo music and native surround/object content are the primary paths**. Native structured VGM/SPC/sequencer integration is a later expansion. Those systems are useful now as architectural examples of music before flattening, not as implementation scope for the core headphone renderer.

---

## 6. Canonical scene direction

The exact Rust API remains intentionally unfrozen, but the signal model should distinguish at least three roles:

```text
                     SCENE
                       │
          ┌────────────┼────────────┐
          │            │            │
   DIRECT SOURCES   AMBIENT FIELD   ROOM FIELD
          │            │            │
    precise HRTF      HOA / SH     early + late
    stable position   diffuse      source-linked
    radial depth      broad        externalization
    sharp attacks     enveloping   environment
          │            │            │
          └────────────┼────────────┘
                       ↓
                 binaural output
```

The three roles are perceptually different and do not need to share one rendering algorithm.

### Direct sources

Localization-critical material should use the highest-precision practical binaural path. Discrete per-source HRIR remains the reference for high-confidence sources.

### Ambient field

Diffuse, broad, or low-confidence material should not be fabricated into point sources. Third-order HOA (16 ACN/SN3D channels) is the leading first field representation to test, not a permanently fixed order.

### Room field

Room response is localization and externalization evidence, not decorative reverb. The early field remains source-linked and directionally informative. After a perceptual/acoustic mixing region, energy becomes increasingly diffuse and supports room presence and envelopment.

---

## 7. Why Omniphony is the base

Current Omniphony already provides useful foundations:

- independent binaural output that bypasses the VBAP speaker path;
- per-channel/object 3D positions;
- separate ITD and HRIR processing;
- embedded measured KEMAR, synthetic/parametric, and SOFA HRTF paths;
- first-order shoebox image-source geometry;
- shared stereo FDN late reverberation;
- distance scaling and air absorption;
- real-time Rust rendering;
- Windows ASIO output;
- deterministic non-realtime file output;
- decoder bridge ABI;
- OSC state/control and 3D Studio supervision;
- a self-contained multichannel test asset.

The immediate job is to raise the native binaural-renderer ceiling before implementing a complex stereo scene frontend.

---

## 8. Reproducible controls

Stable binaural controls live in:

```text
omniphony-renderer/assets/binaural-baselines/
```

The baseline configs must continue to exercise only already-established renderer behavior. Experimental algorithms receive explicit flags/configs and must not silently modify the control.

For every meaningful candidate retain:

- exact source asset;
- exact config;
- sample rate / block size;
- renderer commit;
- HRTF source;
- output gain;
- objective measurements;
- listening notes.

Offline and real-time paths should execute the same DSP whenever practical. VISR's research architecture is a useful precedent: the same components can be exercised in deterministic offline simulations and real-time audio, avoiding separate reference and production implementations.

---

## 9. Immediate renderer research frontier

### R1. Directional early reflections

Current Omniphony computes correct first-order image-source positions and relative delays, then renders each reflection as a broadband ILD-panned copy. The spatial geometry contains more directional information than the ear signals currently receive.

Test at least:

1. each important image rendered through an interpolated HRIR;
2. early images encoded into an HOA field and binauralized once;
3. hybrid rendering, precise HRTF for strongest/earliest images and field rendering for weaker/dense images.

Measure and listen for:

- externalization;
- source width without loss of source identity;
- room geometry;
- front/back stability;
- transient-envelope error;
- comb coloration;
- interaural coherence by band;
- CPU cost.

### R2. HRTF geometry and interpolation

Current HRIR interpolation uses a regular azimuth/elevation grid with separate analytic ITD, which is a useful design strength because time-aligned filters interpolate without moving bulk interaural delay through the FIR coefficients.

Compare:

- current grid;
- denser grid;
- full lower-hemisphere coverage;
- spherical-neighbor interpolation;
- SOFA native-grid interpolation;
- diffuse-field equalization / direction-independent normalization alternatives.

No grid-resolution control belongs in the normal UI.

### R3. Hybrid direct-source / field binauralization

Mature renderers expose a useful trade-off: unique per-source HRIR filtering maximizes point-source clarity, while Ambisonic binaural paths can efficiently represent larger source counts and diffuse fields.

Do not force one choice globally.

```text
high-confidence direct source → direct HRTF
broad / diffuse material      → HOA field
early room events             → directional or hybrid
late room energy              → diffuse field
```

### R4. HOA perceptual optimization

Use conventional ACN/SN3D semantics at internal boundaries. Compare basic decoding against psychoacoustically motivated alternatives such as max-rE, diffuse-field equalization, and MagLS-style methods where licensing and implementation permit.

Direct-HRTF rendering remains the localization/timbre control.

### R5. Radial depth and near field

Distance must remain independent from simple gain reduction. Evaluate direct/reverberant ratio, source-linked early reflections, air absorption, near-field HRTF behavior, source extent, bass anchoring, and proximity filtering together.

### R6. Late field

Retain the current FDN as a baseline. Candidate improvements include:

- frequency-dependent decay;
- room-size-dependent mixing behavior;
- more controlled interaural coherence;
- explicit transition from directional early events to diffuse late energy;
- spatially distributed late energy rather than stereo decorrelation alone;
- measured BRIR and BinauralSDM comparisons;
- expensive learned room models as offline oracles.

IEM's FDN implementation is a useful reference rather than a drop-in dependency: it supports large multichannel networks, Walsh-Hadamard feedback mixing, per-line delays, and frequency-dependent feedback via shelving filters. These are useful ideas for testing richer late fields.

### R7. Source width and envelopment controls

Add objective and perceptual experiments that vary these independently.

Candidate measurements:

- band-limited interaural coherence;
- lateral/front/rear/vertical energy distribution;
- early-to-late energy ratio;
- direct-envelope similarity after reflected-energy addition;
- source centroid variance;
- apparent-source-width ratings;
- listener-envelopment ratings.

A candidate that increases LEV by smearing direct sources fails.

---

## 10. Stereo auditory scene frontend comes later

Do not return to hard `FFT bin → speaker` routing.

The frontend should form persistent auditory-region hypotheses using evidence such as:

- harmonicity;
- onset synchrony;
- common changes in frequency/intensity;
- pitch/timbre similarity;
- stereo level, phase, delay and coherence;
- transient ownership;
- decay/directness;
- temporal predictability;
- continuity over multiple time scales.

A possible internal state:

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

This represents a perceptual hypothesis, not a claim of recovering physical stems.

Two!Ears is a useful conceptual reference because it explicitly combines classical signal-driven auditory processing with higher-level/top-down knowledge in an active-listening model. The lesson is architectural: future scene inference may require both bottom-up evidence and contextual priors, while provenance/confidence must distinguish them.

---

## 11. Learned spatial intelligence

Learned approaches remain serious candidates, especially for scene interpretation.

Possible roles:

```text
offline oracle
      ↓
metadata teacher
      ↓
optional runtime scene estimator
```

Prefer preserving the original musical waveform where possible. A learned model may generate masks, source/field confidence, direction, extent, depth, or ambient structure while deterministic Omniphony DSP performs final binaural rendering.

Full generated replacement waveforms remain experimental until they survive transient, vocal, cymbal, bass, phase, timbre, hallucination, latency, generalization, and licensing tests.

---

## 12. HRTF and headphone translation

Personalization is a ceiling-raiser, not a prerequisite.

Keep two layers conceptually separate:

```text
canonical binaural scene
          ↓
headphone-specific translation / correction
          ↓
physical headphone
```

Research lanes include:

- user SOFA HRTFs;
- perceptual HRTF matching;
- anthropometric / ear-image estimates;
- individualized headphone equalization;
- left/right mismatch correction;
- generic model-specific headphone profiles.

A generic default must still be excellent.

For research tooling, `pyfar`, `sofar`, and `spharpy` are useful independent analysis references: general acoustic-signal/coordinate processing, AES69 SOFA validation and manipulation, and spherical-harmonic mathematics respectively. They belong in the research environment, not necessarily the real-time Rust dependency graph.

---

## 13. Evaluation

### Loudness matching

A candidate may not win because it is louder. Record integrated level, peak/headroom, and gain changes.

### Acclimation + removal

For important candidates:

1. level-match;
2. listen long enough for the candidate scene to normalize;
3. switch to the control;
4. note what spatial dimensions disappeared;
5. return to candidate and inspect any clarity/timbre penalty.

Desired removal result: acoustic volume, depth, height, externalization, or envelopment collapses while the dry source does not suddenly become obviously cleaner.

### Score dimensions separately

Do not use one generic "immersive" score. Record at least:

```text
front externalization
rear discrimination
side precision
elevation
radial distance
apparent source width
listener envelopment
source extent
source separation
source stability
room presence / scale
ambient continuity
transient clarity
vocal/direct clarity
timbral fidelity
bass stability
fatigue
bypass-collapse strength
```

### Objective diagnostics

Where practical compute:

- short-time ILD;
- ITD / group-delay behavior;
- interaural coherence by frequency band;
- direct/reverberant ratio;
- early reflection timing and energy;
- source-centroid / position variance;
- transient-envelope error;
- magnitude-response deviation;
- left/right energy bias;
- RMS/integrated level;
- true peak/headroom;
- deterministic output hashes for fixtures;
- real-time CPU and callback timing.

Objective metrics are guardrails, not substitutes for listening.

---

## 14. External research references and boundaries

Use external projects to extract methods and controls, not to assemble a dependency collage.

Especially useful references currently include:

- Omniphony upstream architecture;
- Spatial Audio Framework / SPARTA;
- OpenAL Soft;
- Google Open Binaural Renderer;
- libspatialaudio;
- IEM Plug-in Suite;
- VISR / Binaural Synthesis Toolkit;
- BinauralSDM;
- libmysofa;
- pyfar / sofar / spharpy;
- Two!Ears;
- measured BRIR/HRTF databases;
- modern learned stereo-to-spatial and room-acoustic models.

Check license compatibility before importing code or data. A useful algorithmic idea does not imply that its implementation belongs in the production binary.

---

## 15. Anti-patterns

Reject or heavily penalize candidates that depend primarily on:

- loudness increase;
- bass boost;
- generic reverb as a spatial substitute;
- polarity tricks / anti-phase synthetic rears;
- hard per-bin speaker assignment;
- rapid scene churn;
- indiscriminate decorrelation;
- duplicating content into rear/height channels with EQ/reverb only;
- per-track manual tuning;
- proprietary HRIR redistribution;
- double binauralization;
- resurrecting the abandoned Spatial DSP renderer.

Do not delete upstream head-tracking support, but static listening remains the primary target for this research.

---

## 16. Development order

```text
0. reproducible baseline and offline comparison
1. native binaural renderer ceiling
   - directional early reflections
   - HRTF interpolation / coverage
   - radial depth / near field
   - early→late room transition
   - direct vs field rendering
2. conservative stereo direct/ambient decomposition
3. persistent auditory-region scene model
4. learned scene intelligence experiments
5. automatic standard playback behavior
6. surround/game precision and latency validation
7. Windows-wide capture/output integration
8. headphone translation / personalization
9. optional structured-source expansions such as VGM
```

The research can loop backward whenever evidence exposes a weaker lower layer.

---

## 17. Current first falsifiable experiment

**Directional early reflections.**

Keep the current image-source positions, relative delays, room dimensions, HRTF source, late field, source content, and overall gain as controlled as possible. Change only how the early reflected directions reach the ears.

Compare the current broadband-ILD reflection renderer against direction-dependent binaural alternatives.

Advance only if the candidate produces a repeatable increase in externalization, room geometry, source body, or distance **without** measurable/listenable damage to localization, transient envelope, timbre, bass stability, or headroom.

That is the first acoustic step because it improves the renderer itself before any stereo scene inference can obscure the result.
