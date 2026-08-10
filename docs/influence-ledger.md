# External influence ledger

This file is the durable memory of external projects, documentation, listening systems, papers, and implementation patterns mined while building **Omniphony for Headphones**.

It is intentionally **not** a second roadmap. `README.md` owns product direction. This ledger exists so useful mechanisms survive chat compaction even when they are not immediately promoted into code.

## Promotion rule

```text
source / influence
→ concrete mechanism or lesson
→ relevance to an observed weakness
→ bounded experiment
→ objective validation + listening
→ retain / narrow / reject
```

Parking is not rejection.

The upstream Omniphony renderer remains the spatial foundation. External work should normally improve how we feed, preserve, validate, or selectively extend that core rather than vote to replace it.

---

# 1. Upstream Omniphony

Sources:

- https://github.com/mgth/Omniphony
- https://omniphony.mgth.fr/
- upstream `BINAURAL.md`
- upstream bundled `assets/demo/demo.yaml`

Durable conclusions:

- The renderer is the inheritance. Studio is supervision/control, not the audio engine.
- Binaural output is an independent path that bypasses the speaker/VBAP output chain.
- Upstream owns the load-bearing HRTF/HRIR, ITD, geometry, object/bed, reflection, room, head-pose and binaural machinery.
- Bridges are the intended decode/format seam.
- Known spatial content is already a strong fit for the upstream engine.
- Upstream's actual bundled headphone demo is materially different from the fork's earlier approximate reference.

Upstream bundled demo settings include approximately:

```text
SAF/KEMAR HRTF
unit_scale_m = 3.0
early reflections enabled, level 0.4
short late reverb enabled, level 0.2
RT60 = 0.3 s
```

Important correction:

- those settings are a **known-spatial demo control**, not automatically an ideal preset for finished stereo music;
- ordinary stereo music exposed a different problem: preserving the authored master while adding spatial support.

Current product law:

> When possible, feed upstream Omniphony better source/presentation material rather than replacing its renderer.

---

# 2. Current HeSuVi / Equalizer APO incumbent

Reference chain:

```text
foobar2000
→ SoX
→ Skip Silence (alternative)
→ Vocal Exciter
→ Reverb
→ stereo→5.1/side upmix
→ Advanced Limiter
→ Hi-Fi Cable
→ Equalizer APO + HeSuVi
→ DTS Virtual:X for speakers HRIR
→ ASIO Bridge
→ FiiO
→ Dan Clark Noire X
```

Useful lessons:

- large coherent acoustic volume matters;
- rear structure can be compelling when stable;
- bass weight/timing and center authority matter;
- complicated internals are acceptable, complicated user ritual is not;
- end-to-end success means eventually making this incumbent unnecessary;
- its virtual-speaker architecture is an influence/reference, not a blueprint.

Migration law:

```text
keep installed
→ disable competing stage
→ test Omniphony replacement
→ remove only after earned
```

---

# 3. Stereo Convolution DSP / 2x2 convolution matrix

Relevant lineage:

- foobar Stereo Convolution DSP (`foo_dsp_stereoconv`)
- Hydrogenaudio component documentation
- general multichannel FIR/convolution literature

Core mathematical lesson:

A stereo convolver can be represented as a 2x2 transfer matrix:

```text
yL = L * HLL + R * HRL
yR = L * HLR + R * HRR
```

This is more relevant to Omniphony than a single opaque wet convolution because it exposes separate responsibilities:

```text
HLL / HRR
→ diagonal / identity-bearing transfer

HLR / HRL
→ cross-ear / interaural support
```

Useful Omniphony interpretation:

```text
H(z) = I + S(z)
```

where the identity term can explicitly preserve the finished stereo master while the added matrix supplies only spatial support that earns itself.

This does **not** mean the final implementation must literally be one fixed 2x2 FIR matrix.

What survives as a design law:

> The original stereo solution can remain structurally present instead of being deleted and recreated as two virtual speakers.

Immediate relevance:

- current generic full-wet L/R HRTF rendering was heard as tinny, bass-light and less spatial than dry stereo;
- a preserved-direct + spatial-support architecture is therefore the next bounded experiment.

Status:

- **promoted as an architectural experiment**, not yet a frozen production algorithm.

---

# 4. MathAudio Headphone EQ / crossfeed lineage

Sources:

- https://mathaudio.com/headphone-eq.htm
- https://mathaudio.com/download.htm

Useful ideas:

- headphone correction and spatial presentation should remain separate responsibilities;
- headphone stereo lacks the natural opposite-ear leakage present with loudspeakers;
- bounded crossfeed/interaural coupling can therefore be perceptually useful;
- left and right correction can be considered independently;
- a consumer UI can expose sophisticated DSP while remaining simple to operate.

Important boundary:

- do **not** blindly add strong crossfeed to Omniphony;
- crossfeed can narrow stereo, and the current dry stereo reference was already heard as having more useful width/rear definition than the failed wet path;
- any cross-ear term must be frequency-, timing-, and listening-test-aware.

Potential use:

- think of controlled cross-ear support as an off-diagonal transfer term;
- keep headphone compensation separate from the world/presentation renderer;
- test crossfeed only as one mechanism inside a larger protected-master architecture.

Status:

- **design influence only**.

---

# 5. MathAudio Room EQ

Sources:

- https://mathaudio.com/room-eq.htm
- https://mathaudio.com/why-room-eq.htm

Useful lessons:

- do not blindly invert every measured imperfection;
- deep notches and ill-conditioned inverse corrections should be bounded;
- pre-ringing and transient behavior matter perceptually;
- correction strength should be constrained by audible benefit rather than mathematical completeness.

Useful general law:

> **Measurable invertibility is not permission to perform the inversion.**

Important rejection:

- MathAudio's broad anti-FIR framing is **not adopted**;
- modern binaural/auralization literature and open renderers successfully use FIR and partitioned convolution;
- the real concern is bad phase design, long inappropriate kernels, switching artifacts, latency and poorly constrained correction.

Status:

- **bounded correction philosophy retained; anti-FIR generalization rejected**.

---

# 6. Partitioned convolution literature

## Frank Wefers, 2015

**Partitioned convolution algorithms for real-time auralization**

Relevant conclusions:

- straightforward long time-domain FIR processing becomes expensive for realtime auralization;
- partitioned fast convolution is a standard solution;
- uniform, non-uniform and unpartitioned approaches have different latency/compute tradeoffs;
- multichannel and time-varying filtering are first-class requirements in spatial audio.

Product implication:

```text
short early partitions
→ low latency

larger late partitions
→ cheap long response
```

Non-uniform partitioning is a strong future candidate if Omniphony develops longer direct/ambient/room filters.

Do not introduce it until current short upstream HRIR processing actually needs replacement or augmentation.

---

# 7. HiFi-LoFi/FFTConvolver

Source:

- https://github.com/HiFi-LoFi/FFTConvolver

Useful implementation reference:

- compact C++ realtime FFT convolution;
- uniform partitioned convolution;
- non-uniform / two-stage partitioning branch;
- useful source for studying low-latency front partitions plus cheaper long tails.

Status:

- **parked implementation reference**.

No transplant until a measured product requirement exists.

---

# 8. 3D Tune-In Toolkit

Sources:

- paper: **3D Tune-In Toolkit: An open-source library for real-time binaural spatialisation**
- PLoS ONE, 2019
- DOI / article: https://doi.org/10.1371/journal.pone.0211899
- project lineage: https://github.com/3DTune-In/3dti_AudioToolkit

Highly relevant architectural lesson:

```text
ANECHOIC / DIRECT PATH
source
→ HRIR
→ ITD
→ near-field corrections

separate from

REVERBERANT / ENVIRONMENT PATH
field representation
→ room / BRIR processing
```

The paper also emphasizes:

- ITD handled separately from HRIR interpolation;
- careful avoidance of audible gain/filter-transition artifacts;
- room/reverb computed as a distinct stage;
- realtime convolution as infrastructure, not an audible effect by itself.

This independently supports the current Omniphony direction:

> direct identity and environmental support should not be one inseparable wet effect.

Status:

- **high-value architectural influence**.

---

# 9. Google Open Binaural Renderer (OBR)

Source:

- https://github.com/google/obr

Useful structural lesson:

OBR distinguishes filter roles such as:

```text
Direct
Ambient
Reverberant
```

with substantially different response lengths in its published configuration/examples.

Durable lesson:

```text
DIRECT
short / precise / identity-bearing

AMBIENT + REVERBERANT
may use longer temporal support
```

Do not copy tap counts literally. Preserve the asymmetry of responsibilities.

Status:

- **high-value design influence**.

---

# 10. Preferred headphone response can depend on content type

Paper:

**On the Differences in Preferred Headphone Response for Spatial and Stereo Content**

Authors: Isaac Engel, D. Alon, Kevin Scheumann, Jeff Crukley, Ravish Mehra
Journal of the Audio Engineering Society, 2022.

Useful result:

- listeners in the reported tests preferred different headphone responses for conventional stereo content versus spatial/binaural content;
- this reinforces that a finished stereo master and authored spatial source are not identical reproduction problems.

Omniphony implication:

```text
finished stereo
→ preserve its established presentation expectations

rich spatial source
→ solve accurate binaural reproduction of known geometry
```

This does not require separate products. It requires source-aware presentation boundaries.

Status:

- **supports source-truth / stereo-authority law**.

---

# 11. FIR phase / pre-ringing literature

Relevant papers reviewed include:

- **Optimization of Phase Correction for Finite Impulse Response Filters**, Johann Gaus, JAES, 2026;
- **Evaluation of headphone phase equalization on sound reproduction**, Li et al., Applied Acoustics, 2019;
- **Perceptual Study and Auditory Analysis on Digital Crossover Filters**, Korhola & Karjalainen, JAES, 2008.

Durable lessons:

- FIR phase correction can create pre-ringing/time-domain coloration when designed without perceptual constraints;
- phase/group-delay behavior can influence clarity and transient perception;
- crossover/filter evaluation must include temporal artifacts, not only magnitude response.

Future validation dimensions:

```text
magnitude error
phase / group-delay error
pre-response
ringing
transient smear
stereo-width error
interchannel mismatch
```

Status:

- **validation influence**, not permission to add long correction filters now.

---

# 12. Valve Steam Audio

Source:

- https://github.com/ValveSoftware/steam-audio

Useful mechanisms / tests:

- stateful per-source binaural processing;
- HRTF interpolation quality/performance tradeoff;
- SOFA HRTFs;
- separation of direct sound, reflections and late environment;
- Ambisonics for diffuse/full-sphere fields;
- SIMD-aware realtime engineering;
- mature Unity/Unreal/FMOD/Wwise host boundaries.

Status:

- benchmark / influence only.

Do not graft Steam Audio over Omniphony.

---

# 13. Resonance Audio

Source:

- https://github.com/resonance-audio/resonance-audio

Useful lessons:

- per-source binaural state;
- smooth HRTF transitions;
- Ambisonic field representation;
- room simulation separable from direct binaural rendering;
- thin host adapters around a stable engine.

Status:

- archived reference, not dependency target.

---

# 14. Meta XR Audio SDK samples

Sources:

- https://github.com/oculus-samples/Unity-MetaXRAudioSDK
- https://github.com/oculus-samples/Unreal-MetaXRAudioSDK

Primary value is a perceptual test taxonomy:

- room acoustics;
- source directivity;
- HRTF/spatialization amount;
- host integration behavior.

Status:

- parked.

---

# 15. Cavern

Source:

- https://github.com/VoidXH/Cavern

Relevant convergence:

- headphone direction + distance rendering;
- channel/object formats;
- surround→3D upconversion;
- calibration/measurement;
- low-latency operation;
- listener/source abstractions.

Use as a benchmark for how one product unifies many source types without forcing the listener to manage them manually.

Do not replace Omniphony with Cavern.

---

# 16. Dolby public repositories

Sources:

- https://github.com/orgs/DolbyLaboratories/repositories
- https://github.com/DolbyLaboratories/gst-home-audio

Useful architecture:

```text
parse / decode
→ object / flexible rendering
→ perceptual post-processing
→ output
```

Also useful:

- layout negotiation;
- object/channel distinction;
- rich source truth preserved until render.

Boundary:

- proprietary Dolby render libraries are not open implementation sources.

Status:

- protocol/architecture influence only.

---

# 17. CamillaDSP

Source:

- https://github.com/HEnquist/camilladsp

Important host architecture lessons:

```text
capture
→ bounded handoff
→ processing
→ bounded handoff
→ playback

+ supervisor/control
```

Useful Windows/realtime mechanisms:

- `wasapi-rs`;
- event-driven/polling modes;
- explicit format negotiation;
- reconnect/format handling;
- capture/playback clock management;
- optional resampling;
- optional ASIO kept outside the ordinary Windows route;
- realtime thread priority.

Status:

- high-value host engineering influence.

---

# 18. wasapi-rs

Source:

- https://github.com/HEnquist/wasapi-rs

Already adopted for Windows process-loopback capture.

Useful capabilities:

- render/capture;
- shared/exclusive;
- event/poll modes;
- application loopback;
- session/device notifications.

Boundary:

- host plumbing only;
- portable Omniphony DSP must remain unaware of Windows APIs.

---

# 19. HEnquist audio ecosystem

Sources include:

- `camilladsp`;
- `wasapi-rs`;
- `audio_thread_priority`;
- `audioadapter-rs`.

Useful themes:

- isolate device plumbing from DSP;
- explicit buffer topology;
- realtime priority as a host concern;
- small infrastructure crates instead of platform contamination in DSP.

---

# 20. ASIO2WASAPI

Source:

- https://github.com/levmin/ASIO2WASAPI

Useful as an interoperability reference proving that ASIO compatibility can remain a boundary concern rather than define the core.

Status:

- parked until specialist-route work.

---

# 21. Product-level findings that survive this research pass

## A. Upstream renderer stays the heart

```text
better source/presentation
→ upstream Omniphony
→ binaural output
```

Prefer this to replacing the renderer.

## B. Finished stereo must remain structurally present

Generic full-wet virtual-speaker treatment failed the first clean music listening tests.

The next architecture is:

```text
protected stereo identity
+
small Omniphony-derived support field
```

## C. Direct and environment are separate

```text
direct identity
≠ ambient field
≠ early reflection
≠ late room
```

## D. Convolution is a tool, not a sound signature

Partition it correctly, constrain phase, transition kernels safely, and test temporal artifacts.

## E. Crossfeed is a mechanism, not a product mode

It may help restore natural interaural coupling but can also narrow stereo. Test it in bounded frequency/timing contexts.

## F. Bass/foundation gets a veto

Spatial processing may not remove the low-frequency pressure/groove foundation and then repair it with EQ.

## G. Richer source truth reduces inference

Native 7.1/object/height sources should go more directly into Omniphony than stereo does.

## H. Multiple source layouts coexist

Stereo music and a surround game are separate logical streams, not one global Windows channel mode.

## I. Platform host and core stay separate

Windows solves Windows routing. Omniphony solves sound.

## J. The UI collapses complexity

```text
install
→ ON
→ play
```

---

# 22. Immediate experiment promoted by this pass

The next bounded listening prototype should test:

```text
ORIGINAL STEREO MASTER
        │
        ├──────────────→ protected direct output
        │
        └→ conservative field extraction
             ↓
           upstream Omniphony binaural machinery
             ↓
           low-level spatial support
        │
        └──────────────→ combine
                         ↓
                      headphones
```

Constraints:

- bass/foundation protected;
- no default late reverb;
- no requirement that the whole master pass through an HRTF;
- support field must be removable without revealing that the music itself was damaged;
- first implementation may remain host-side so it can be rejected cheaply;
- if it wins listening tests, move the mechanism into the portable presentation layer.

This experiment supersedes the assumption that ordinary stereo should simply be treated as a two-channel spatial bed.
