# External influence ledger

This file is the durable memory of external projects, documentation, listening systems, and implementation patterns mined while building **Omniphony for Headphones**.

It is intentionally **not** a second roadmap. `README.md` remains the product authority. This ledger exists so useful findings do not disappear when chat context is compacted or research moves on.

## Promotion rule

External work enters the product only through this sequence:

```text
source / influence
→ concrete mechanism or lesson
→ relevance to an observed product need
→ bounded experiment
→ objective validation + listening
→ adopt only if it preserves or beats the protected Omniphony baseline
```

A useful idea can remain parked here indefinitely. Parking is not rejection.

The protected upstream Omniphony binaural character remains the product floor.

---

## Upstream Omniphony documentation

Sources reviewed:

- https://omniphony.mgth.fr/
- https://omniphony.mgth.fr/docs/getting-started/
- https://omniphony.mgth.fr/docs/using-studio/
- https://omniphony.mgth.fr/docs/speaker-layout/
- https://omniphony.mgth.fr/docs/binaural/
- https://omniphony.mgth.fr/docs/playback-mpv-omniphony/
- https://omniphony.mgth.fr/docs/osc-protocol/
- https://omniphony.mgth.fr/docs/custom-backends/

Durable conclusions:

- The renderer is the inheritance. Studio is supervision/control, not the audio engine.
- Binaural is a separate output stage and bypasses the speaker/VBAP output chain.
- The stock headphone path is approximately:

```text
position
→ head-pose rotation
→ azimuth/elevation/distance
→ air absorption
→ distance gain
→ per-ear timing
→ HRIR convolution
→ early reflections
→ optional shared late tail
→ stereo headphones
```

- The bundled reference demo is the cleanest first listening oracle: rotating 7.1.4 reference material through the real binaural stage.
- Room cues are perceptually important for externalization, but richer room processing is not automatically better.
- Plain channel beds and object audio are already distinct concepts upstream.
- Channel-bed spatialization, phantom extraction, and height synthesis are useful upstream mechanisms to test later rather than reinvent blindly.
- Bridges are the intended format boundary. The renderer should not absorb every decoder.
- Custom render backends are bounded algorithm seams, not a reason to rewrite the engine.
- ASIO is an optional Windows path in upstream builds, not a requirement for the renderer itself.

Immediate use:

- P0 should exercise the bundled reference bridge + protected binaural reference over native Windows playback.

Parked:

- expose only a tiny subset of upstream expert controls in a future consumer UI;
- head tracking;
- SOFA browsing/personalized HRTFs;
- live OSC diagnostics;
- richer bed synthesis once ordinary stereo and system routing are stable.

---

## Current HeSuVi / Equalizer APO incumbent

This is the practical system Omniphony for Headphones must eventually replace, not an architecture to clone.

Known current chain:

```text
foobar2000
→ SoX resampling
→ Skip Silence (alternative)
→ Vocal Exciter
→ Reverb
→ stereo→5.1/side upmix
→ Advanced Limiter
→ VB-Audio / Hi-Fi Cable multichannel route
→ Equalizer APO + HeSuVi
→ DTS Virtual:X for speakers HRIR
→ VB-Audio ASIO Bridge
→ FiiO ASIO Driver
→ FiiO K7
→ Dan Clark Noire X
```

Known transport/reference facts:

- 8-channel virtual transport;
- 48 kHz;
- 24-bit;
- 512-sample ASIO buffer;
- final hardware output is stereo;
- ASIO is being used because it is an effective way to bridge the existing Hi-Fi Cable/HeSuVi stack, not because the future product should require ASIO.

Known perceptual lessons:

- large coherent acoustic volume is desirable;
- rear presentation is valuable only when stable;
- bass weight/timing and center authority are important;
- complicated internal machinery is acceptable, complicated user ritual is not;
- FreeSurround-style flattening/collapse is a negative reference;
- end-to-end A/B must be against this real incumbent, not only dry stereo.

Future HeSuVi-directory mining should append concrete configuration/HRIR/channel-layout findings here when the files are available individually.

---

## Valve Steam Audio

Source:

- https://github.com/ValveSoftware/steam-audio

Useful mechanisms / validation targets:

- stateful per-source binaural processing;
- HRTF interpolation quality/performance tradeoff;
- custom SOFA HRTFs;
- explicit separation of direct sound, reflections, transmission/occlusion, and late environmental response;
- SIMD-aware runtime optimization;
- fixed PCM/audio-buffer contracts;
- Ambisonics as a compact representation for diffuse/full-sphere sound fields;
- mature engine integration patterns for Unity/Unreal/FMOD/Wwise.

Immediate status:

- **parked**. Do not graft Steam Audio into P0.

Potential future experiments:

- compare Omniphony moving-source interpolation against Steam Audio-style bilinear HRTF interpolation behaviour;
- benchmark SIMD/convolution hot paths if profiling proves they matter;
- test an Ambisonic intermediate only for diffuse-field problems that the current renderer cannot solve cleanly;
- use Steam Audio environmental semantics as a test taxonomy, not as ownership of our room model.

---

## Resonance Audio

Source:

- https://github.com/resonance-audio/resonance-audio

Status:

- archived project; engineering literature/reference, not a dependency target.

Useful findings:

- stateful binaural processing per source;
- custom SOFA HRTFs;
- smooth interpolation is essential for moving wide-band sources;
- Ambisonics can represent both point-source mixtures and diffuse sound fields;
- geometrical-acoustics tooling and scene-derived reverberation are separable from the core binaural renderer;
- DAW/game integrations reinforce the value of a stable engine boundary plus thin host adapters.

Parked:

- Ambisonic field representation;
- personalized HRTF experiments;
- geometrical-acoustic room estimation.

---

## Meta XR Audio SDK samples

Sources:

- https://github.com/oculus-samples/Unity-MetaXRAudioSDK
- https://github.com/oculus-samples/Unreal-MetaXRAudioSDK

Useful contribution is primarily a **perceptual test taxonomy**:

- room acoustics;
- source directivity;
- HRTF intensity / spatialization amount;
- engine-specific integration demonstrations.

Immediate status:

- parked; no Meta SDK dependency or transplant.

Future use:

- build isolated listening fixtures for directivity, room contribution, and spatialization-strength behaviour when those dimensions become tunable in Omniphony for Headphones.

---

## Cavern

Source:

- https://github.com/VoidXH/Cavern

Why it matters:

Cavern independently converges on several mature-product goals close to ours:

- direction + distance headphone virtualization;
- immersive/object formats;
- realtime regular-surround→3D upconversion;
- room correction/calibration;
- low-latency operation at very small block sizes;
- measurement tooling;
- explicit listener/source abstractions.

Immediate status:

- benchmark/influence only. Do not replace Omniphony's binaural renderer with Cavern.

Parked future comparisons:

- surround→3D upconversion behaviour;
- distance virtualization;
- tiny-buffer stability;
- measurement/calibration workflows;
- how one product unifies object, channel, and headphone paths without forcing users to reason about them.

License note:

- inspect Cavern's current license carefully before copying any implementation. Prefer concepts/tests unless a specific code reuse decision is explicitly reviewed.

---

## Dolby Laboratories public repositories

Organization:

- https://github.com/orgs/DolbyLaboratories/repositories

Most relevant reviewed repository:

- https://github.com/DolbyLaboratories/gst-home-audio

Useful architectural lesson:

Dolby's public GStreamer work separates stages cleanly:

```text
parse/decode
→ object rendering / flexible rendering
→ perceptual post-processing
→ output
```

It also exposes channel-layout negotiation and object/channel distinctions.

Important boundary:

- core Dolby rendering/processing libraries used by these plugins are proprietary; there is no open Atmos-for-Headphones implementation here to transplant.

Immediate status:

- architecture/protocol reference only.

Parked:

- channel-mask/layout negotiation ideas;
- decoder→object-renderer→post-process separation;
- testing against common immersive layouts;
- consumer UX observations from Dolby products, without cloning proprietary processing.

---

## CamillaDSP

Source:

- https://github.com/HEnquist/camilladsp

This is one of the most relevant Windows/realtime engineering references found.

Current architecture lesson:

```text
capture thread
→ bounded/message-queue handoff
→ processing thread
→ bounded/message-queue handoff
→ playback thread

+ supervisor/control thread
```

Useful Windows mechanisms:

- direct `wasapi-rs` backend;
- shared and exclusive WASAPI;
- event-driven and polling modes;
- explicit sample-format negotiation;
- device-period-aware buffering;
- loopback capture;
- device disconnect / format-change handling;
- capture/playback clock-drift management;
- optional resampling/rate adjustment;
- optional ASIO feature kept separate from the ordinary Windows build;
- realtime thread-priority promotion;
- prebuilt Windows binaries whose default backend is WASAPI.

Immediate product lesson:

- P0 may use CPAL to prove sound quickly, but **direct `wasapi-rs` is the preferred P1 hardening candidate** once we need deterministic event-driven behaviour, device/session notifications, recovery, and explicit format control.

Do not copy CamillaDSP's whole architecture. Adopt only what our measured transport needs.

Parked:

- supervisor/recovery state machine;
- adaptive rate control if capture and playback clocks diverge in a future virtual-device topology;
- direct WASAPI event loop;
- realtime thread priority;
- reload/reconfigure patterns;
- mature device enumeration and error reporting.

---

## wasapi-rs

Source:

- https://github.com/HEnquist/wasapi-rs

Already used in `windows_host` for process-loopback probing.

Relevant supported capabilities:

- playback and capture;
- shared and exclusive modes;
- event-driven and polled buffering;
- loopback capture;
- volume/session/device-disconnect notifications;
- application-specific capture.

The examples provide small canonical implementations of:

- shared-mode event-driven playback;
- exclusive playback;
- loopback capture;
- device enumeration;
- per-application capture.

Promotion status:

- process-loopback probe: **already adopted for diagnostics**;
- direct render/playback loop: **parked for P1 hardening** after P0 is audible;
- application capture: **parked** as a possible development/testing route, not a final system-wide interception strategy.

---

## HEnquist audio ecosystem

Profile/repositories reviewed:

- https://github.com/HEnquist?tab=repositories
- `camilladsp`
- `wasapi-rs`
- `audio_thread_priority`
- `audioadapter-rs`
- `cpal-listdevices`
- related audio buffer/sample utilities

Useful themes:

- isolate device plumbing from DSP;
- make buffer topology explicit;
- wrap different memory layouts behind narrow interfaces;
- treat realtime thread priority as an explicit host concern;
- keep platform APIs thin enough that DSP remains portable;
- use dedicated small crates for infrastructure rather than contaminating DSP code.

Immediate status:

- conceptual reinforcement only except existing `wasapi-rs` use.

Parked:

- `audio_thread_priority` or equivalent if profiling/glitch testing shows scheduler pressure;
- buffer-adapter ideas if renderer/host copies become measurable;
- more direct device-enumeration patterns for eventual UI.

---

## ASIO2WASAPI

Source:

- https://github.com/levmin/ASIO2WASAPI

Role for this project:

- interoperability/reference for translating between ASIO-oriented software expectations and normal Windows WASAPI devices;
- evidence that ASIO compatibility can remain a boundary concern rather than dictate the core renderer architecture.

Immediate status:

- parked. P0 stays ordinary WASAPI-first.

Future use:

- revisit when publishing and deciding how much specialist ASIO compatibility the consumer package should expose;
- compare its device/buffer/format adaptation assumptions against our eventual optional ASIO host.

---

## Product-level findings that survived the final pass

These are now considered durable design constraints unless later evidence overturns them.

### 1. Renderer and host remain separate

```text
input/decoder/bridge
→ Omniphony scene + binaural renderer
→ narrow PCM boundary
→ Windows host
```

### 2. Ordinary Windows first, specialist routes second

```text
WASAPI shared/default path
→ expected normal user route

ASIO
→ optional specialist/reference route
```

### 3. Environment is not one effect

Keep separable:

```text
direct localization
source extent/directivity
early reflections
late diffuse field
distance/air cues
```

A giant room effect must never become the substitute for spatial structure.

### 4. Movement quality matters

Moving sources are a stress test for:

- HRTF interpolation;
- filter transitions;
- callback-size invariance;
- head tracking;
- source-position smoothing.

### 5. Buffer size must not change the intended auditory world

Host callback size is transport. Perceptual timing semantics belong on the sample timeline.

### 6. Realtime reliability belongs in the host boundary

Eventually test:

- device removal/reconnect;
- sleep/resume;
- format changes;
- shared-mode contention;
- default-device switching;
- underrun/overrun recovery;
- scheduler pressure;
- capture/playback clock drift if separate devices are ever involved.

### 7. The user-facing product should collapse complexity

The eventual target remains:

```text
install
→ choose/detect headphones/output
→ enable
→ forget it is there
```

Advanced controls may exist later, but the default should already be excellent for as many listeners and headphones as practical.

---

## Research freeze for first audible prototype

For the first runnable Windows artifact, do **not** add new external DSP mechanisms.

P0 scope:

```text
bundled upstream reference scene
→ reference bridge
→ protected Omniphony binaural configuration
→ stereo PCM
→ existing realtime identity seam
→ native Windows WASAPI output
```

Then listen.

Only after this path builds and is heard do we promote additional transport or DSP work.
