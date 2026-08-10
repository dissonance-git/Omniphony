# Omniphony for Headphones

> **Private master project plan and canonical re-entry surface.**
>
> This repository is being built for one listener first. This README is an engineering memory, roadmap, product contract, and context-recovery document rather than public marketing.
>
> If chat context disappears, a connector fails, research sprawls, or a later refactor starts optimizing the wrong thing, recover the project from this file and current Git history before inventing a new direction.

Omniphony for Headphones is a fork of [`mgth/Omniphony`](https://github.com/mgth/Omniphony) being turned into a **Windows-first binaural spatial-audio system for headphones, with ordinary music as the primary use case**.

The reason the fork exists is simple and must not be abstracted away:

> **Upstream Omniphony already sounds unusually good. Preserve that spatial character, make it practical for normal Windows listening, and improve it only when an improvement earns itself.**

The upstream hosted headphone demo already produced a convincing acoustic volume: rotating material felt like a real 360° orbit around the listener rather than a flattened lateral pan. That percept is the foundation.

The current practical goal is to replace a complicated HeSuVi/VB-Audio/ASIO listening pipeline with Omniphony **without first destroying the working pipeline and without burying the already-good Omniphony effect under research architecture**.

The long-term aspiration is intentionally extreme:

> after acclimation, ordinary headphone playback should feel lower-dimensional by comparison.

The engineering constraint is stricter:

> **dimension may not be purchased by damaging the music.**

---

# 0. Project hierarchy

```text
UPSTREAM OMNIPHONY
already-good 360° binaural percept
        ↓
PROTECTED PERCEPTUAL FLOOR
must stay reproducible
        ↓
NATIVE WINDOWS PRODUCT
make the renderer easy to use every day
        ↓
CONTROLLED IMPROVEMENTS
only where tests/listening expose a weakness
        ↓
OPTIONAL RICHER INTELLIGENCE
libaural / research later, when it earns itself
```

Do **not** reverse this into:

```text
research architecture
→ general artificial hearing
→ speculative scene system
→ someday rebuild Omniphony inside it
```

The renderer is the thing being productized. Research is a toolbox around it.

---

# 1. The two references Omniphony must preserve or beat

## 1.1 Upstream Omniphony = renderer perceptual ancestor

The hosted upstream headphone demo is the primary listening oracle for the renderer's starting character.

The smallest local approximation of its published configuration is:

```text
omniphony-renderer/assets/binaural-baselines/upstream-demo-reference.yaml
```

Published ingredients approximated by that control:

```text
stock-style Omniphony defaults
+ SAF/KEMAR HRTF
+ early reflections enabled
+ late reverb explicitly disabled
```

The hosted site does not pin the exact render commit/command, so this local config is not claimed to be byte-identical to the hosted file. Its role is perceptual ancestry.

A richer fork config that sounds worse at matched loudness does not become the default merely because it contains more DSP.

## 1.2 Current Windows listening chain = end-to-end incumbent

The second reference is the real daily system that Omniphony must eventually make unnecessary.

Omniphony does not graduate because it beats dry stereo or a weak generic virtualizer. It should eventually be preferable to the actual tuned chain on the actual hardware.

Keep these questions separate:

```text
Did the renderer improve?

and

Would the finished native product replace the current system?
```

---

# 2. Current incumbent snapshot

Snapshot restored from the actual Windows setup on **2026-08-10**. This is a benchmark and provenance record, not a specification to clone stage by stage.

## 2.1 foobar2000 DSP order

```text
Resampler (SoX)
→ Skip Silence (alternative)
→ Vocal Exciter
→ Reverb
→ Upmix to 5.1/side
→ Advanced Limiter
```

Active upmix channels:

```text
FL FR C LFE SL SR
```

Older snapshots may show `FreeSurround Decoder`. That is historical lineage, not the current configuration. FreeSurround remains a useful negative comparison because earlier listening found that it could flatten/collapse the desired 3D bubble.

## 2.2 Virtual multichannel transport

Current transport uses **VB-Audio ASIO Bridge / Hi-Fi Cable**.

```text
input transport:   8 channels
sample rate:       48,000 Hz
resolution:        24-bit
ASIO device:       FiiO ASIO Driver
ASIO output:       2 channels
buffer:            512 samples
ASIO sample rate:  48,000 Hz
```

The foobar upmix itself uses six active channels inside that multichannel transport.

ASIO is part of the incumbent because it is an effective specialist bridge through the Hi-Fi Cable/HeSuVi stack. That does **not** make ASIO the required product-default route for Omniphony for Headphones.

## 2.3 HeSuVi / Equalizer APO virtualization

Current HRIR selection:

```text
DTS Virtual:X for speakers
Original-unmodified file, DTS, Inc.
version shown: 2025.3.16.0
```

Observed matrix state:

```text
Upmix Content
  Stereo: enabled
  5.1:    enabled

Content Format
  Automatic
```

Observed speaker-position adjustments:

```text
front: -5
side:  +5
rear: -15
```

Observed level adjustments:

```text
Master  90
Center 100
Front  100
Side   100
Rear   100
LFE    200
```

These numbers are part of the incumbent listening reference. They are **not** automatically Omniphony target parameters.

## 2.4 Hardware reference

```text
FiiO K7
→ Dan Clark Noire X
```

A resolving chain is useful for exposing false spaciousness, phase smear, transient softening, brittle HRTF coloration, bass timing damage, unstable localization, and reverb haze.

Desired scaling law:

```text
better headphones / DAC chain
→ clearer access to the same coherent auditory world
→ more microdetail / dynamics / localization

NOT

better headphones
→ more obvious DSP artifacts
```

---

# 3. The incumbent is evidence, not a specification

Do not cargo-cult the current chain.

Omniphony does not need permanent literal equivalents of:

```text
Vocal Exciter
Reverb
5.1 upmix
LFE = 200
DTS virtualization
Hi-Fi Cable
ASIO Bridge
```

Ask instead:

> **What useful audible function was each stage buying, and can Omniphony provide that function more directly, coherently, and with less plumbing?**

What the incumbent teaches us perceptually:

- a large convincing acoustic volume is desirable;
- meaningful behind-head presentation is desirable when stable;
- bass weight and timing matter strongly;
- center authority cannot be casually weakened;
- music must retain punch and identity;
- ordinary stereo music is the primary source;
- set-and-forget daily use matters;
- complicated internals are acceptable, complicated listener rituals are not.

---

# 4. Target product

```text
WINDOWS AUDIO SOURCE
system / player / foobar / game
        ↓
NATIVE WINDOWS ROUTE
ordinary PCM first
        ↓
OMNIPHONY REALTIME CORE
protected upstream foundation
+ only proven fork improvements
        ↓
BINAURAL STEREO
        ↓
WINDOWS OUTPUT DEVICE
        ↓
headphones
```

The first successful product does **not** require complete artificial hearing, perfect source separation, a giant learned model, mobile ports, a research dashboard, per-song authoring, dozens of exposed controls, or dismantling the incumbent before Omniphony is ready.

The eventual normal UX should be boring:

```text
install
→ choose / detect output + headphones
→ enable
→ play normally
```

Development builds may expose more diagnostics and A/B controls. The product should not become a cockpit.

---

# 5. Migration law: coexist first, replace later

There is **no cold-turkey migration**.

The current foobar + HeSuVi + VB-Audio + ASIO system remains available while Omniphony develops beside it.

```text
CURRENT CHAIN
known-good daily listening
        │
        ├──────── remains available
        │
        ▼
OMNIPHONY DEVELOPMENT
        ├→ protected renderer reference
        ├→ deterministic tests
        ├→ native Windows host tests
        ├→ incumbent-chain A/B
        └→ matched-loudness listening
        │
        ▼
Omniphony clearly earns a function
        │
        └→ old component becomes redundant
```

The old chain is retired because Omniphony **wins**, not because the roadmap declares it obsolete.

---

# 6. Perceptual acceptance law

The goal is not simply "wider."

The desired result is a coherent acoustic volume with independent front/back, side precision, elevation, radial depth, source extent, and ambience.

At matched loudness, bypass should ideally collapse:

```text
perceived acoustic volume
externalization
front/back structure
height
radial depth
source extent
ambient continuity
```

Bypass must **not** restore:

```text
clarity
transient punch
bass timing or weight
timbral identity
vocal solidity
rhythmic precision
microdetail
dynamics
musical hierarchy
comfort
```

Failure examples:

```text
huge bubble + smeared music
rear spectacle + weak center
room scale + softened transients
more width + less image stability
more ambience + worse bass timing
```

The desired bypass reaction is:

> “the world collapsed.”

not:

> “the music came back.”

---

# 7. Scene vocabulary without scene-model tyranny

Useful renderer concepts:

```text
FrontalAnchor
DirectObject
BroadSource
DiffuseField
RoomField
```

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

These distinctions prevent every kind of spaciousness from collapsing into reverb or decorrelation. They are **not** a prerequisite hierarchy for the first useful Windows build.

Ordinary stereo does not reveal literal authored rear-object metadata. Rear placement may be a valid presentation decision without being described as recovered source truth.

---

# 8. Research boundary and durable external memory

The broad research passes were useful and are not being thrown away.

Durable findings are parked in:

- `docs/influence-ledger.md` for external projects, mechanisms, comparisons, and adopted-vs-parked status;
- `docs/windows-integration-research.md` for Microsoft Core Audio, Spatial Sound, endpoint/APO, and driver-integration findings.

Influences include Steam Audio, Resonance Audio, Meta XR Audio, Cavern, CamillaDSP / `wasapi-rs`, Dolby public work, Microsoft Core Audio, Ambisonic tooling, HRTF/BRIR research, psychoacoustics, MIR/source separation, the older `spatial-dsp` experiment, and mature realtime audio engines.

Their role is:

```text
actual weakness appears
        ↓
formulate exact missing capability
        ↓
search research / implementations
        ↓
identify candidate mechanism
        ↓
small isolated experiment
        ↓
measure + listen
        ↓
keep only if earned
```

External projects are mechanism sources and benchmarks, not architecture votes. Academic novelty does not outrank an existing good percept.

Useful findings should be written into the repo even when they are parked, so later work does not need to reconstruct GitHub dives from chat history.

---

# 9. libaural relationship

`libaural` is a separate artificial-hearing research/framework project. It may later provide better evidence for adaptive presentation.

It is **not** a prerequisite for Omniphony to be useful.

```text
Omniphony
existing renderer
+ local bounded evidence
+ native Windows product work
        ↓
works independently
        ↓
optional richer evidence later
        ↓
libaural-informed improvements when proven
```

Rules:

- libaural does not replace the perceptual baseline by fiat;
- Omniphony must remain runnable without the full research stack;
- libaural should enter through a bounded projection/state interface;
- model/hearing work stays off the realtime callback;
- observations are evidence for presentation, not spatial commands;
- a cheaper local heuristic may remain if it is more stable or sounds as good or better;
- research graduates only when the product benefits.

---

# 10. `spatial-dsp` relationship

The older Real3D/`spatial-dsp` work is lineage and a mechanism mine, not a topology to preserve.

Useful concepts already harvested include:

- hard-pan-safe direct/source evidence;
- phase-correct complex M/S evidence;
- persistence separate from first-frame agreement;
- center preservation;
- direct/diffuse distinction;
- bass anchoring;
- rear structure without claiming recovered rear truth;
- source spread and room/boundary cues;
- Windows CI lessons.

Old topology:

```text
stereo
→ synthetic multichannel bed
→ external virtualizer
```

Target topology:

```text
stereo / rich source
→ bounded evidence / scene state
→ Omniphony binaural renderer
→ headphones
```

The old migration diary has been removed from the working tree because its useful laws now live in code/tests, this README, and the scene/rendering contracts. Git history remains the archive.

---

# 11. Windows is the product now

Correct current rule:

> **Build the Windows product first.**

Keep OS/device code above the renderer where practical, avoid unnecessary Windows semantics inside scene/HRTF math, and keep a small host/engine boundary. Do not spend current project time implementing macOS/Linux/Android/iOS products.

Portability is preserved by sane ownership, not by an active cross-platform roadmap.

---

# 12. Current native Windows progress

## 12.1 `windows_host` — EXISTS

Workspace crate:

```text
omniphony-renderer/windows_host/
```

Current purpose: thin Windows product/transport prototype, not a second renderer.

It currently provides/proves:

- CPAL default Windows host output-device discovery;
- normal Windows build without enabling CPAL's optional ASIO feature;
- manual self-excluding WASAPI process-loopback activation probe;
- explicit `--smoke-output` native-output test through the bit-exact realtime PCM seam;
- explicit `--reference-demo` path that renders the bundled upstream 7.1.4 reference through the protected Omniphony binaural engine and plays the resulting stereo over native WASAPI;
- `--render-reference-only` packaged engine validation for CI;
- packaging in the Windows artifact workflow.

Internal P0 path:

```text
bundled 7.1.4 reference WAV
→ reference_bridge
→ protected Omniphony binaural renderer
→ stereo PCM
→ realtime_ffi identity seam
→ native WASAPI output
```

P0 is an internal milestone name, not public branding.

Important limitation:

> **loopback is a copy, not an intercept.**

The dry Windows mix still reaches its normal endpoint, so loopback is diagnostic/experimental only, not the final HeSuVi replacement.

## 12.2 `realtime_ffi` — EXISTS

Purpose: tiny PCM boundary between native host code and the Omniphony renderer.

Current ABI:

```text
interleaved f32 PCM
sample rate + channels at creation
process callback by frame count
in-place or out-of-place
explicit reset
C ABI / header
```

The first implementation is deliberately **bit-exact identity**. That gives transport a deterministic oracle before persistent realtime Omniphony DSP is connected behind the seam.

## 12.3 P0 CI checkpoint — COMPILED

The P0 Actions run completed successfully on 2026-08-10, including Windows compilation/package work and packaged protected-reference validation.

The only reported compiler warning was an `unused_mut` in a test-local closure in `orender_engine/src/object_gen.rs`. It is cosmetic and does not invalidate the build. Remove it with the next code-touching commit rather than expanding this documentation/structure commit into an unrelated large source rewrite.

## 12.4 What does NOT exist yet

Do not infer these from P0:

- no completed transparent system-wide Omniphony route yet;
- no chosen production APO yet;
- no finished virtual render endpoint yet;
- loopback capture is not the final route;
- the protected renderer is not yet the persistent callback-time processor behind `realtime_ffi` for arbitrary daily PCM;
- no arbitrary-stereo music product path has been judged yet;
- no claim yet that native Omniphony has replaced the incumbent.

---

# 13. Windows single-path routing problem

The final product needs ordinary Windows playback to reach the listener **once**, through Omniphony.

Detailed transport ownership and decision gates live in `docs/windows-audio-route.md`. Windows API/endpoint research lives in `docs/windows-integration-research.md`.

Candidate families remain:

## A. Endpoint/system-effect APO

```text
application
→ Windows shared audio engine
→ Omniphony APO
→ physical endpoint
```

Attractive for true in-place, set-and-forget playback. Costs include realtime in-process constraints, endpoint association, installation/signing, and crash containment.

## B. Virtual render endpoint

```text
application
→ Omniphony virtual endpoint
→ Omniphony host process
→ realtime_ffi
→ renderer
→ physical endpoint
```

Attractive for process isolation and explicit single-path routing. Costs include WDK/signing, another endpoint, buffering/clock-domain complexity, and device lifecycle ownership.

## C. ASIO specialist route

ASIO remains useful for specialist hardware/workflows and as a compatibility/reference route. It should remain supported where useful, but it is not sufficient as the ordinary system-wide solution by itself.

```text
normal Windows route
→ native system integration

optional specialist route
→ ASIO
```

Microsoft Spatial Sound app APIs are useful future **input semantics** for static beds/dynamic objects, but reviewed public documentation does not establish a simple generic registration API for making an arbitrary renderer appear beside Sonic/Atmos/DTS in the Spatial Sound dropdown.

The UX target stays stable even if the implementation mechanism changes.

---

# 14. Current renderer foundation

Do not rewrite useful inherited machinery for aesthetics.

Retained substrate includes:

- stateful binaural DSP;
- analytic ITD;
- interpolated HRTF/HRIR rendering;
- embedded SAF/KEMAR, parametric, and SOFA-capable providers;
- moving-filter crossfades;
- object position/size state;
- known-scene VBAP/layout machinery useful for controlled truth;
- early image-source reflections;
- late FDN room-field machinery;
- deterministic DSP fixtures;
- headless engine/FFI boundaries.

Fork work already includes or has absorbed:

- complex M/S stereo evidence;
- persistence-aware evidence;
- conservative object/field separation;
- bass/foundation protection separate from object identity;
- deterministic asynchronous HRTF switching;
- stale HRIR-build rejection;
- measured-HRIR direct-arrival validation;
- per-ear directional early-reflection timing;
- sample-time-oriented FDN modulation;
- true zero predelay;
- reusable fidelity metrics;
- optional upstream spectral-phantom extraction;
- optional distance-diffuse mirror-axis behavior;
- upstream runtime-isolation mechanisms.

---

# 15. Protected binaural controls

Directory:

```text
omniphony-renderer/assets/binaural-baselines/
```

### `upstream-demo-reference.yaml`

**Perceptual ancestor.** Minimal stock-style approximation of the published upstream demo.

### `baseline-room.yaml`

**Fork room-assisted comparison.** It does not outrank the upstream control merely because it contains more room processing.

### `dry-binaural.yaml`

Fork HRTF/scale/air policy with reflections and late reverb disabled, useful for isolating fork room contribution.

Experimental algorithms get separate explicit configs/flags. Never overwrite a protected control to make a candidate look better.

---

# 16. Upstream relationship

`mgth/Omniphony` remains the technical ancestor, perceptual foundation, and continuing source of mechanisms/fixes. It does not define this fork's product roadmap.

Use:

```text
inspect exact upstream change
→ identify mechanism
→ check whether already present
→ ask whether it serves this Windows headphone product
→ import the smallest useful missing part
→ validate locally
```

Do not merge broad upstream product work merely to keep history visually similar. Do not delete useful inherited renderer behavior merely because the final UX is narrower.

The August 2026 active-branch sweep already found the important spectral-phantom, diffuse-mirror-axis, and runtime-isolation implementation work present in the fork. Do not repeatedly remine those branches unless upstream changes.

When this fork proves a general fix, isolate the portable/general portion before considering an upstream contribution.

---

# 17. Mandatory validation lanes

Keep failures attributable.

## Lane A · Compiler / deterministic DSP

```text
compile
→ unit tests
→ deterministic fixtures
→ null/fidelity checks
→ callback invariance where required
```

## Lane B · Known scene → binaural

```text
known geometry
→ HRTF / ITD
→ extent / room
→ binaural output
```

## Lane C · Stereo evidence / scene hypothesis

```text
controlled stereo
→ evidence
→ persistence / confidence
→ scene state
```

## Lane D · Native Windows transport

```text
same PCM / engine
→ host route A
→ host route B
→ compare timing / glitches / latency / semantics
```

## Lane E · Renderer perceptual comparison

```text
controlled source/scene
→ upstream Omniphony reference
↔ fork candidate
```

## Lane F · End-to-end product comparison

```text
CURRENT
foobar + VB-Audio + HeSuVi + FiiO ASIO

VERSUS

TARGET
native Omniphony + FiiO K7 / Noire X
```

---

# 18. Listening scorecard

Score dimensions separately:

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
vocal/direct solidity
timbral fidelity
bass stability / groove
microdetail
dynamics
fatigue
bypass-collapse strength
```

Loudness-match comparisons. Do not normalize candidates independently in a way that turns level into “quality.”

---

# 19. Realtime law

> **Host callback size is an implementation detail, not a coordinate system for the auditory world.**

Gain, movement, HRTF transitions, room changes, and other continuous state should be defined in sample/time coordinates when they are supposed to be continuous.

Do **not** generalize existing green callback-related tests into “all motion is solved.” Position/HRTF movement remains a separate candidate defect where block-start publication can still matter.

See `docs/realtime-control-contract.md` and `docs/scene-renderer-contract.md`.

---

# 20. Current implementation frontier

## W0 · Protect/reproduce the renderer sound — ESTABLISHED

- upstream-demo-style local perceptual control exists;
- fork room control is explicitly not the perceptual ancestor;
- deterministic known-scene/file routes remain;
- Windows backend path test failure is fixed;
- baseline renderer/core Actions checkpoint was owner-verified green.

## P0 · First audible native reference build — COMPILED, LISTENING PENDING

Built and packaged:

```text
windows_host.exe
reference_bridge.dll
orender.dll
omniphony_realtime.dll
protected reference config
7.1.4 layout
spatial-demo.wav
```

Acceptance on the real Windows machine:

```text
1. run windows_host.exe --smoke-output
2. confirm clean native endpoint playback
3. run windows_host.exe --reference-demo
4. confirm the protected reference is audible/stable/recognizably Omniphony
5. compare against the hosted upstream perceptual ancestor
```

## P0.1 · Simplest ordinary-stereo listening path

After the controlled reference works:

```text
ordinary stereo music
→ conservative presentation entry
→ protected Omniphony renderer
→ native Windows output
```

No new external DSP mechanism is required merely to prove this path.

## P1 · Easy everyday Windows listening

- persistent realtime renderer behind the host seam;
- device/output handling that survives real use;
- fast incumbent ↔ Omniphony A/B;
- transport hardening from evidence;
- direct event-driven `wasapi-rs` remains a parked candidate if CPAL becomes limiting.

## P2 · System-wide single-path enable/disable

Prototype the smallest viable supported endpoint/APO/virtual-endpoint route and choose from measured reliability, latency, installability, and real use.

## P3 · Native surround / rich spatial input

Preserve 5.1/7.1/height beds and true spatial-object semantics where available instead of double-virtualizing already-collapsed binaural content.

## R1 · Fix actual renderer weaknesses exposed by listening

Candidates, not mandatory queue:

- sample-time position/HRTF motion;
- source extent / `BroadSource` behavior;
- directional early-reflection consistency;
- front/back/elevation robustness;
- direct/room separation;
- low-frequency integrity.

## S1 · Small persistent stereo scene

Wire current stereo evidence into bounded persistent state while preserving center/foundation and remaining conservative under uncertainty.

## C1 · Calibration/personalization

Later: per-device profiles, optional headphone correction, HRTF selection/import, headroom management, and deeper driver-ear/BRIR personalization after the core experience is strong.

---

# 21. Product anti-goals

- Do not replace good sound with research.
- Do not build a settings forest.
- Do not detour into cross-platform shells before Windows works.
- Do not hallucinate object truth from stereo evidence.
- Do not equate reverb with 3D.
- Do not make AI availability a playback dependency.
- Do not let infrastructure consume the product.
- Do not rewrite history merely because a newer plan is cleaner.

Use:

```text
actual weakness
→ smallest tested mechanism
→ measure
→ listen
→ keep only if earned
```

---

# 22. Repository contraction law

The working tree should become easier to understand, but deletion is not a sport.

Keep code when it serves at least one of:

```text
1. native Windows daily playback
2. protected renderer behavior
3. deterministic renderer / known-scene truth
4. HRTF / calibration truth
5. migration / A-B observability
6. a current isolated experiment
```

Already removed/retired inherited surfaces include Studio-centric product code, obsolete suite packaging/release surfaces, JACK/mpv product routes, generic demonstration/script backends, and historical implementation diaries.

Do not delete reference layouts, deterministic fixtures, protected configs, useful ASIO support, or renderer internals merely because the final listener UI will never expose them.

Detailed current crate/surface ownership lives in `docs/contraction-ledger.md`.

---

# 23. Documentation precedence

This README owns:

- product identity;
- current listener/incumbent context;
- baseline hierarchy;
- migration law;
- roadmap priority;
- research/libaural boundary;
- current phase status.

Supporting docs own narrower technical contracts.

Current docs:

- `docs/windows-audio-route.md` — single-path Windows route, transport ladder, APO vs virtual endpoint;
- `docs/windows-integration-research.md` — parked Core Audio/Spatial Sound, endpoint/APO, and driver findings;
- `docs/influence-ledger.md` — durable external GitHub/research memory and adopted-vs-parked status;
- `docs/headphone-rendering-research.md` — practical renderer experiments and listening plan;
- `docs/scene-renderer-contract.md` — evidence/scene/rendering distinctions and current renderer gaps;
- `docs/realtime-control-contract.md` — sample-time/realtime correctness;
- `docs/music-presentation-contract.md` — optional future adaptive-presentation laws;
- `docs/headphone-calibration.md` — later listener/headphone calibration architecture;
- `docs/contraction-ledger.md` — current crate/surface ownership and safe contraction order;
- `omniphony-renderer/assets/binaural-baselines/README.md` — protected sound controls and A/B procedure;
- `CONTRIBUTING.md` — private development rules for us/Codex/tooling.

Removed as redundant working-tree documents:

- old standalone portability policy: its live law is now in this README and realtime/host boundaries;
- old standalone fork policy: its live laws are now in this README, `CONTRIBUTING.md`, and the contraction ledger;
- old `spatial-dsp` migration diary: its useful mechanisms are implemented or retained in this README/contracts and Git history.

If a supporting document conflicts with this README's product priority, the README wins until explicitly revised.

Do **not** create another master-plan document beside this one.

---

# 24. Working development loop

When uncertain what to do next:

```text
make native Windows Omniphony easier to hear
        ↓
compare against protected upstream-style reference
        ↓
compare against real HeSuVi incumbent
        ↓
identify the next actual weakness
        ↓
research only that weakness
        ↓
implement smallest candidate
        ↓
measure + listen
        ↓
keep only what earns itself
```

This is intentionally resistant to research drift.

---

# 25. Re-entry checkpoint

If conversational context is lost, do not reconstruct the project from memory.

Start with:

1. this README;
2. recent commits on `main`;
3. `docs/windows-audio-route.md` for the live native-host frontier;
4. `docs/influence-ledger.md` for external research/influences;
5. `docs/windows-integration-research.md` for system-route/Spatial Sound/APO questions;
6. `docs/scene-renderer-contract.md` and `docs/realtime-control-contract.md` for renderer/realtime contracts;
7. the protected binaural baselines;
8. the real incumbent snapshot in this file.

Durable hierarchy:

```text
1. preserve the already-good upstream Omniphony percept
2. keep the current HeSuVi chain intact while developing beside it
3. finish the native Windows route around the existing renderer
4. compare against the real incumbent on K7 + Noire X
5. improve only where listening/testing reveals an actual weakness
6. use research and libaural later as optional sources of better mechanisms
7. never trade the music for the effect
```

That is Omniphony for Headphones.
