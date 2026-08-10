# External influence ledger

This file is the durable memory of external projects, documentation, listening systems, papers, plugins, and implementation patterns mined while building **Omniphony for Headphones**.

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

This exposes separate responsibilities:

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

where the identity term can explicitly preserve the finished stereo master while the added matrix supplies only spatial support that earns its existence.

This does **not** mean the final implementation must literally be one fixed 2x2 FIR matrix.

Durable law:

> The original stereo solution can remain structurally present instead of being deleted and recreated as two virtual speakers.

Immediate relevance:

- generic full-wet L/R HRTF rendering was heard as tinny, bass-light and less spatial than dry stereo;
- preserved-direct + spatial-support is therefore the current bounded experiment.

Status:

- **promoted as an architectural experiment**, not a frozen production algorithm.

---

# 4. MathAudio Headphone EQ / crossfeed lineage

Sources:

- https://mathaudio.com/headphone-eq.htm
- https://mathaudio.com/download.htm

Useful ideas:

- headphone correction and spatial presentation should remain separate responsibilities;
- headphone stereo lacks natural opposite-ear leakage present with loudspeakers;
- bounded crossfeed/interaural coupling can therefore be perceptually useful;
- left and right correction can be considered independently;
- sophisticated DSP can still have a simple consumer UI.

Important boundary:

- do **not** blindly add strong crossfeed;
- crossfeed can narrow stereo;
- the current dry reference was heard as having more useful width/rear definition than the failed full-wet path;
- any cross-ear term must be frequency-, timing-, and listening-test-aware.

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

General law:

> **Measurable invertibility is not permission to perform the inversion.**

Important rejection:

- MathAudio's broad anti-FIR framing is **not adopted**;
- modern binaural/auralization systems successfully use FIR and partitioned convolution;
- the real risks are bad phase design, inappropriate kernels, switching artifacts, latency and unconstrained correction.

---

# 6. Partitioned convolution literature

## Frank Wefers, 2015

**Partitioned convolution algorithms for real-time auralization**

Relevant conclusions:

- straightforward long time-domain FIR processing becomes expensive for realtime auralization;
- partitioned fast convolution is a standard solution;
- uniform, non-uniform and unpartitioned approaches trade latency and compute differently;
- multichannel and time-varying filtering matter in spatial audio.

Product implication:

```text
short early partitions
→ low latency

larger late partitions
→ efficient long response
```

Non-uniform partitioning is a future candidate if longer direct/ambient/room filters become useful.

Do not introduce it merely because it exists.

---

# 7. HiFi-LoFi/FFTConvolver

Source:

- https://github.com/HiFi-LoFi/FFTConvolver

Useful implementation reference:

- compact realtime FFT convolution;
- uniform partitioning;
- non-uniform / two-stage partitioning;
- useful source for low-latency front partitions plus cheaper long tails.

Status:

- **parked implementation reference**.

---

# 8. 3D Tune-In Toolkit

Sources:

- **3D Tune-In Toolkit: An open-source library for real-time binaural spatialisation**
- PLoS ONE, 2019
- https://doi.org/10.1371/journal.pone.0211899
- https://github.com/3DTune-In/3dti_AudioToolkit

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

Other useful lessons:

- ITD can be handled separately from HRIR interpolation;
- gain/filter transitions need explicit artifact control;
- room/reverb should remain a distinct stage;
- realtime convolution is infrastructure, not an audible effect by itself.

Status:

- **high-value architectural influence**.

---

# 9. Google Open Binaural Renderer (OBR)

Source:

- https://github.com/google/obr

Useful structural lesson:

OBR distinguishes roles such as:

```text
Direct
Ambient
Reverberant
```

with very different response lengths.

Durable lesson:

```text
DIRECT
short / precise / identity-bearing

AMBIENT + REVERBERANT
may use longer temporal support
```

Do not copy tap counts literally. Preserve the asymmetry of responsibilities.

---

# 10. Preferred headphone response can depend on content type

Paper:

**On the Differences in Preferred Headphone Response for Spatial and Stereo Content**

Authors: Isaac Engel, D. Alon, Kevin Scheumann, Jeff Crukley, Ravish Mehra
Journal of the Audio Engineering Society, 2022.

Useful result:

- the reported listening tests preferred different headphone responses for conventional stereo and spatial/binaural content;
- a finished stereo master and an authored spatial source are therefore not identical reproduction problems.

Omniphony implication:

```text
finished stereo
→ preserve established presentation expectations

rich spatial source
→ solve accurate binaural reproduction of known geometry
```

---

# 11. FIR phase / pre-ringing literature

Relevant work:

- **Optimization of Phase Correction for Finite Impulse Response Filters**, Johann Gaus, JAES, 2026;
- **Evaluation of headphone phase equalization on sound reproduction**, Li et al., Applied Acoustics, 2019;
- **Perceptual Study and Auditory Analysis on Digital Crossover Filters**, Korhola & Karjalainen, JAES, 2008.

Durable lessons:

- FIR phase correction can create pre-ringing/time-domain coloration when unconstrained;
- phase/group-delay behavior can affect clarity and transient perception;
- crossovers and filters require temporal validation, not only magnitude-response plots.

Future metrics:

```text
magnitude error
phase / group delay
pre-response
ringing
transient smear
stereo-width error
interchannel mismatch
```

---

# 12. Valve Steam Audio

Source:

- https://github.com/ValveSoftware/steam-audio

Useful mechanisms / tests:

- stateful per-source binaural processing;
- HRTF interpolation quality/performance tradeoffs;
- SOFA HRTFs;
- direct/reflection/late-environment separation;
- Ambisonics for diffuse/full-sphere fields;
- SIMD-aware realtime engineering;
- mature host integrations.

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

Status: archived reference, not dependency target.

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

Use as a benchmark. Do not replace Omniphony with Cavern.

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

Useful lessons:

- channel/layout negotiation;
- object/channel distinction;
- preserve rich source truth until render.

Boundary: proprietary Dolby render libraries are not open implementation sources.

---

# 17. CamillaDSP

Source:

- https://github.com/HEnquist/camilladsp

Important host architecture:

```text
capture
→ bounded handoff
→ processing
→ bounded handoff
→ playback

+ supervisor/control
```

Useful mechanisms:

- `wasapi-rs`;
- event/poll modes;
- explicit format negotiation;
- reconnect/format handling;
- capture/playback clock management;
- optional resampling;
- optional ASIO outside ordinary Windows route;
- realtime thread priority.

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

Boundary: host plumbing only.

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
- realtime priority as host concern;
- small infrastructure crates rather than platform contamination in DSP.

---

# 20. ASIO2WASAPI

Source:

- https://github.com/levmin/ASIO2WASAPI

Useful as interoperability evidence that ASIO compatibility can remain a boundary concern rather than define the core.

---

# 21. Trifield LR→LRC / Michael Gerzon lineage

Sources:

- https://www.foobar2000.org/components/view/foo_dsp_trifield
- Hydrogenaudio documentation linked from the component page

The foobar component implements Michael Gerzon's Trifield decoder.

Its important product idea is **not simply creating a center channel**. It uses Ambisonic-derived stereo decoding to create an L/C/R stage intended to improve image stability and center definition, including when listening away from the ideal stereo axis.

Relevant Omniphony lesson:

> **Center authority is an independent spatial invariant.**

A stereo enhancer must not obtain width/rear envelopment by hollowing or smearing the authored phantom center.

Potential future evidence stage:

```text
correlated / center-like energy
→ protect as FrontalAnchor evidence

side / differential energy
→ candidate field evidence
```

Do not automatically route extracted center energy to a literal center object in headphones. The valuable part is the stabilization principle.

Status:

- **high-value stereo-analysis influence**.

---

# 22. LCC - Localization Cue Correction

Sources:

- https://www.foobar2000.org/components/view/foo_dsp_lcc
- https://github.com/MeteorStudioASU/lcc

LCC is a lightweight crosstalk-cancellation approach for **stereo loudspeaker playback**. Its stated goal is to prevent the opposite loudspeaker from corrupting the ear-specific ITD/ILD information carried by stereo signals.

Direct use in Omniphony would be wrong because headphones do not have loudspeaker acoustic crosstalk.

But the deeper law transfers:

> **Do not let the reproduction transform destroy useful interaural cues already present in the source.**

Potential use:

- validate ITD/ILD preservation before and after stereo presentation;
- treat source interaural structure as evidence with confidence, not noise to overwrite;
- keep crossfeed/cross-ear support bounded so it does not erase source localization cues.

Status:

- **cue-preservation influence, not a headphone algorithm transplant**.

---

# 23. FreeSurround

Sources:

- current foobar component: https://www.foobar2000.org/components/view/foo_dsp_fsurround
- source lineage preserved in Real3D fork: `freesurround_decoder.cpp`

The source reveals a useful analysis model.

For each FFT bin it computes approximately:

```text
left amplitude
right amplitude
left phase
right phase
        ↓
amplitude difference
phase difference
        ↓
2-D soundfield position x/y
        ↓
wrap / shift / depth / focus
        ↓
channel-allocation map
```

It also exposes separate front/rear separation controls and optional low-frequency redirection with explicit cutoff regions.

This is valuable because it demonstrates that **amplitude + phase relations can be converted into candidate spatial evidence without semantic source separation**.

But the user's historical listening result is also important: FreeSurround could collapse/flatten the desired 3-D bubble.

Therefore:

```text
FreeSurround-style analysis
→ potentially useful evidence

FreeSurround-style reconstructed speaker bed
→ not automatically the desired music output
```

The future experiment worth mining is the **analysis transform**, not wholesale reuse of its final multichannel mix.

Status:

- **high-value evidence extractor / negative output reference**.

---

# 24. NRSC5-Fan Real3D-Surround-Upmixer

Source:

- https://github.com/NRSC5-Fan/Real3D-Surround-Upmixer-

The repository explicitly describes itself as a foobar plugin supporting multiple surround layouts **based on FreeSurround**.

Useful contribution:

- preserves the FreeSurround amplitude/phase → soundfield decode lineage;
- exposes many larger surround allocations;
- demonstrates how the same inferred x/y evidence can be remapped into different channel topologies;
- keeps front/rear separation, depth, focus, circular wrap and bass redirection explicit.

Important boundary:

- more output speakers do not by themselves create better headphone spatiality;
- mapping inferred evidence into 13/16 channels and then HRTF-rendering all of it could simply produce a more elaborate version of the failure already heard.

Potential transfer:

```text
stereo analysis
→ position/confidence evidence
→ Omniphony presentation vocabulary
```

rather than:

```text
stereo
→ giant fake speaker bed
→ treat as authored truth
```

Status:

- **mine the evidence model and layout ideas; do not adopt the bed as truth**.

---

# 25. NUGEN Halo Upmix

Source:

- https://nugenaudio.com/haloupmix/

Commercial reference, not code source.

Highly relevant product claims/design goals:

- stereo→LCR / 5.1 / 7.1 / 3-D / Ambisonic expansion;
- locational-cue analysis of the original stereo;
- no artificial reverb, chorus or delay required for its core upmix;
- coherent spatial extension intended to preserve original character;
- an `Exact` mode specifically concerned with downmix/source integrity;
- separate center management and low-frequency control.

These are almost exactly the dimensions our current prototype needs to test.

Durable Omniphony law:

> **Spatial expansion should be evaluated for reversibility/source preservation, not only for how impressive the expanded field sounds.**

Potential regression test:

```text
stereo master
→ presentation/upmix state
→ defined collapse/downmix
→ compare with original stereo
```

A presentation that creates a wonderful sphere but cannot approximately recover the source relationships may be too destructive for default music playback.

Status:

- **very high-value commercial benchmark**.

---

# 26. Penteo

Source:

- https://www.perfectsurround.com/

Commercial reference, not code source.

Penteo emphasizes:

- phaseless upmix/downmix behavior;
- preservation of source depth/clarity;
- strong downmix compatibility;
- many surround, Atmos, Ambisonic and binaural formats;
- separation/decorrelation controls without requiring reverb as the basic spatializer.

The exact proprietary algorithm is not available, but its constraints are useful.

Durable Omniphony tests:

```text
phase coherence
source recoverability
center stability
low-frequency stability
no mandatory artificial room
```

Important lesson:

> **A powerful immersive upmix can treat downmix compatibility as a first-class invariant rather than an afterthought.**

Status:

- **very high-value benchmark / invariance source**.

---

# 27. Airwindows Wider

Sources:

- https://www.airwindows.com/wider-vst/
- https://github.com/airwindows/airwindows

Wider uses M/S-domain processing to alter the apparent foreground/background relation of center and side material and adds only a very small timing/interpolation effect to the less-forward component.

Useful lesson:

- convincing stereo-space changes do not require reconstructing discrete objects or a surround bed;
- very small M/S-dependent changes can alter perceived depth while retaining a coherent source;
- subtlety can outperform aggressive width processing.

This reinforces the current field-support experiment and suggests future **mid/side depth weighting** before heavier inference.

Status:

- **small-mechanism design influence**.

---

# 28. Goodhertz CanOpener Studio

Sources:

- https://goodhertz.com/canopener-studio/
- https://manuals.goodhertz.com/3.13/canopener-studio/

CanOpener is a stereo→stereo headphone crossfeed/monitoring system rather than a full spatial upmixer.

Useful ideas:

- crossfeed amount and apparent speaker angle are separate controls;
- more realistic modes use delay + spectral modeling;
- a simpler no-delay mode preserves a constant spatial frequency response;
- the product explicitly takes a "less is more" approach rather than requiring a room IR.

Omniphony lesson:

```text
cross-ear support
= one bounded component of presentation
≠ the full spatial world
```

It may be useful later for the protected-direct branch or for understanding natural loudspeaker-like interaural coupling.

Status:

- **crossfeed reference only**.

---

# 29. IEM Plug-in Suite

Sources:

- https://github.com/tu-studio/IEMPluginSuite
- https://plugins.iem.at/

The suite is open-source and supports high-order Ambisonic encoding, manipulation, room processing and binaural decoding.

Useful contribution to Omniphony:

- Ambisonics as a portable representation for a **derived field**;
- matrix tools and explicit encoder/decoder stages;
- binaural decoding separated from field construction;
- standalone/plugin host portability patterns.

Important boundary:

- IEM StereoEncoder is not an automatic music-understanding upmixer;
- Ambisonics should be considered when it is the right representation for a diffuse/support field, not inserted merely because it is spatially elegant.

Status:

- **field-representation and tooling influence**.

---

# 30. Product-level findings that survive the expanded stereo pass

## A. Upstream renderer stays the heart

```text
better source/presentation
→ upstream Omniphony
→ binaural output
```

Prefer this to replacing the renderer.

## B. Finished stereo must remain structurally present

The generic full-wet virtual-speaker treatment failed clean listening.

Current direction:

```text
protected stereo identity
+
small Omniphony-derived support field
```

## C. Analysis is not rendering

FreeSurround/Real3D make this distinction especially clear.

```text
amplitude / phase / M-S / correlation evidence
→ useful spatial clues

inferred speaker bed
→ only one possible rendering
```

Do not confuse evidence extraction with authored source truth.

## D. Center authority gets its own invariant

TriField reinforces that a convincing stereo presentation must keep the center image stable and distinct while other material expands around it.

## E. Source interaural cues get protection

LCC reinforces ITD/ILD preservation as a test target even though its loudspeaker crosstalk algorithm is not directly used on headphones.

## F. Reversibility/downmix quality becomes a regression metric

Halo and Penteo strongly reinforce:

```text
expand
→ collapse/downmix
→ original relationships should survive
```

Perfect bit-exact recovery is not required for every creative presentation, but destructive default processing should be detectable.

## G. Environment is optional support, not spatiality itself

Professional upmix references can create coherent expansion without mandatory reverb/chorus/delay.

Therefore Omniphony must not depend on audible room coloration to feel 3-D.

## H. Convolution is a tool, not a sound signature

Partition it correctly, constrain phase, transition kernels safely, and test temporal artifacts.

## I. Crossfeed is a mechanism, not a mode

It can help restore natural cross-ear relationships but can also narrow the image.

## J. Bass/foundation gets a veto

Spatial processing may not erase low-frequency pressure/groove and repair it later with EQ.

## K. Richer source truth reduces inference

Native surround/object/height sources go more directly into Omniphony than stereo does.

## L. Multiple source layouts coexist

Stereo music and a surround game are independent logical streams, not one global channel mode.

## M. Platform host and core stay separate

Windows solves Windows routing. Omniphony solves sound.

## N. The UI collapses complexity

```text
install
→ ON
→ play
```

---

# 31. Immediate experiment promoted by this pass

The current next-build experiment is deliberately smaller than the final ambition:

```text
ORIGINAL STEREO MASTER
        │
        ├──────────────→ latency-aligned protected direct output
        │
        └→ (L-R)/2 side evidence
             ↓
           high-pass ~220 Hz
             ↓
           rear/side support positions
             ↓
           upstream Omniphony binaural machinery
             ↓
           low-level spatial support
        │
        └──────────────→ combine
                         ↓
                      headphones
```

Initial support gain: about `14%`.

Constraints:

- original stereo remains authoritative;
- direct and support are latency-aligned before addition;
- bass/foundation remains out of the support branch;
- support loses headroom before the master is clipped/scaled;
- no default early reflections;
- no default late reverb;
- no air absorption;
- no semantic runtime reasoning;
- first implementation is intentionally host-side and disposable;
- if it wins listening tests, promote the mechanism into the portable presentation layer.

This experiment tests the architecture, not final spatial strength.

If it succeeds, later candidates include:

```text
center-anchor protection inspired by Trifield
frequency-dependent side/field confidence
FreeSurround-style amplitude/phase evidence without adopting its bed
bounded cross-ear support
reversibility/downmix tests
Ambisonic field representation only if it earns itself
```

This supersedes the assumption that ordinary stereo should simply be treated as a two-channel spatial bed.
