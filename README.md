# Omniphony for Headphones

> **Private master project plan and canonical re-entry surface.**
>
> This repository is being built for one listener first. This README is therefore written as an engineering memory, roadmap, product contract and context-recovery document rather than public marketing.
>
> If chat context disappears, a connector fails, research sprawls, or a later refactor starts optimizing the wrong thing, recover the project from this file and the current Git history before inventing a new direction.

Omniphony for Headphones is a fork of [`mgth/Omniphony`](https://github.com/mgth/Omniphony) being turned into a **native Windows headphone spatial-audio system for ordinary music**.

The reason the fork exists is simple and must not be abstracted away:

> **Upstream Omniphony already sounds unusually good. Preserve that spatial character, make it practical for normal Windows listening, and improve it only when an improvement earns itself.**

The project is not starting from a blank DSP canvas.

The upstream hosted headphone demo already produced a convincing acoustic volume: the rotating test sound felt like a real 360° orbit around the listener rather than a flattened side-to-side image. That percept is the foundation.

The current practical goal is to replace a complicated HeSuVi/VB-Audio/ASIO listening pipeline with Omniphony **without first destroying the working pipeline and without burying the already-good Omniphony effect under research architecture**.

The long-term aspiration is intentionally extreme:

> after acclimation, ordinary headphone playback should feel lower-dimensional by comparison.

The engineering constraint is stricter:

> **dimension may not be purchased by damaging the music.**

---

# 0. Read this first: project hierarchy

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

The renderer is the thing being productized.

Research is a toolbox around it.

---

# 1. The two references Omniphony must beat or preserve

There are two different baselines. They answer different questions.

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

The hosted site does not pin the exact render commit/command, so this local config is not claimed to be byte-identical to the hosted file.

Its role is perceptual ancestry.

A richer fork config that sounds worse at matched loudness does not become the default merely because it contains more DSP.

## 1.2 Current Windows listening chain = end-to-end incumbent

The second reference is the real daily system that Omniphony must eventually make unnecessary.

Omniphony does not graduate because it beats dry stereo or a weak generic virtualizer.

It should eventually be preferable to the actual tuned chain on the actual hardware.

These two references create two separate questions:

```text
Did the renderer improve?

and

Would the finished native product replace the current system?
```

Never confuse them.

---

# 2. Current incumbent snapshot

Snapshot restored from the actual Windows setup on **2026-08-10**.

This is a benchmark and provenance record, not a specification that Omniphony must clone stage by stage.

## 2.1 foobar2000 DSP order

Current order:

```text
Resampler (SoX)
→ Skip Silence (alternative)
→ Vocal Exciter
→ Reverb
→ Upmix to 5.1/side
→ Advanced Limiter
```

The active upmix outputs:

```text
FL FR C LFE SL SR
```

Older profile/project snapshots may show `FreeSurround Decoder` in the active slot. That is historical lineage, not the current 2026-08-10 configuration.

FreeSurround remains useful as a negative comparison because earlier listening found that it could flatten/collapse the desired 3D bubble.

## 2.2 Virtual multichannel transport

Current transport uses **VB-Audio ASIO Bridge / Hi-Fi Cable**.

Observed state:

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

ASIO is part of the current incumbent because it is an effective specialist bridge through the Hi-Fi Cable/HeSuVi stack. That does **not** make ASIO the required product-default route for Omniphony for Headphones.

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

These numbers are part of the incumbent listening reference.

They are **not** automatically Omniphony target parameters.

## 2.4 Hardware reference

```text
FiiO K7
→ Dan Clark Noire X
```

The hardware matters because a resolving chain is useful for exposing:

- false spaciousness;
- phase smear;
- transient softening;
- brittle HRTF coloration;
- bass timing damage;
- unstable localization;
- reverb haze.

Desired scaling law:

```text
better headphones / DAC chain
→ more access to the same coherent auditory world
→ more microdetail / dynamics / localization

NOT

better headphones
→ more obvious DSP artifacts
```

---

# 3. The incumbent is evidence, not a specification

The current chain is the result of years of practical compensation and experimentation.

Do not cargo-cult its implementation.

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

Instead ask:

> **What useful audible function was each stage buying, and can Omniphony provide that function more directly, coherently and with less plumbing?**

If one native Omniphony path produces a stronger result than six chained components, the six-component topology should disappear.

What the incumbent teaches us perceptually:

- a large convincing acoustic volume is desirable;
- meaningful behind-head presentation is desirable when stable;
- bass weight and timing matter strongly;
- center authority cannot be casually weakened;
- music must retain punch and identity;
- ordinary stereo music is the primary source;
- set-and-forget daily use matters;
- complicated internals are acceptable, complicated rituals for the listener are not.

---

# 4. Target product

Current product target:

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

The first successful product does **not** require:

- complete artificial hearing;
- perfect source separation;
- a giant learned model;
- a complete auditory world model;
- mobile ports;
- a driver written from scratch if a simpler integration wins;
- a research dashboard;
- per-song manual authoring;
- dozens of exposed tuning controls;
- removal of the incumbent before Omniphony is ready.

The eventual normal UX should be boring:

```text
install
→ choose / detect output + headphones
→ enable
→ play normally
```

Development builds may expose more diagnostics and A/B controls.

The product should not become a cockpit.

---

# 5. Migration law: coexist first, replace later

There is **no cold-turkey migration**.

The current foobar + HeSuVi + VB-Audio + ASIO system remains available while Omniphony develops beside it.

```text
CURRENT CHAIN
known-good daily listening
        │
        ├──────────── remains available ─────────────┐
        │                                             │
        ▼                                             │
OMNIPHONY DEVELOPMENT                                 │
        │                                             │
        ├→ protected renderer reference               │
        ├→ deterministic tests                        │
        ├→ native Windows host tests                  │
        ├→ incumbent-chain A/B                        │
        └→ matched-loudness listening                 │
        │                                             │
        ▼                                             │
Omniphony clearly earns a function                    │
        │                                             │
        └→ old component becomes redundant ───────────┘
```

Never require dismantling the working environment merely to expose an internal milestone.

The old chain is retired because Omniphony **wins**, not because the roadmap declares it obsolete.

---

# 6. Perceptual acceptance law

The goal is not simply "wider."

The desired result is a coherent acoustic volume with independent front/back, side precision, elevation, radial depth, source extent and ambience.

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

Useful renderer concepts remain:

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

These distinctions are useful because they prevent every kind of spaciousness from collapsing into reverb or decorrelation.

But they are **not** a prerequisite hierarchy for W1.

Ordinary stereo can be rendered usefully before Omniphony has a perfect persistent object graph.

A stereo master also does not reveal literal authored rear-object metadata.

Rear placement may be a valid presentation decision without being described as recovered source truth.

---

# 8. Research boundary

The broad research pass was useful and is not being thrown away.

Durable findings are parked in `docs/INFLUENCE_LEDGER.md`, with Windows-specific endpoint/API findings in `docs/WINDOWS_INTEGRATION_RESEARCH.md`. Research that is not promoted into current code is still retained there so later work can recover it without repeating the same GitHub dives.

Influences include:

- Steam Audio;
- Resonance Audio;
- Meta XR Audio SDK samples;
- Cavern;
- CamillaDSP / `wasapi-rs` and related HEnquist audio work;
- Dolby open-source work;
- Microsoft Core Audio / Spatial Sound documentation;
- SPARTA / IEM / Ambisonic tooling;
- ImmersiveFlow and learned spatial-audio work;
- HRTF/BRIR research;
- psychoacoustics and auditory neuroscience;
- QuadraphonicQuad / historical multichannel practice;
- MIR / source separation / music cognition;
- the older `spatial-dsp` experiment;
- mature realtime audio engines and DAWs.

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

Do not research because the list can always become longer.

External projects are mechanism sources and benchmarks, not architecture votes.

Academic novelty does not outrank an existing good percept.

---

# 9. libaural relationship

`libaural` is a separate artificial-hearing research/framework project.

It may later provide better evidence for adaptive presentation.

It is **not** a prerequisite for Omniphony to be useful.

Current relationship:

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
- a cheaper local heuristic may remain if it is more stable or sounds as good/better;
- research graduates only when the product benefits.

The fork may also teach libaural which auditory distinctions matter in practice.

---

# 10. `spatial-dsp` relationship

The older Real3D/`spatial-dsp` work is lineage and a mechanism mine.

Useful concepts harvested from it include:

- direct/diffuse evidence;
- center preservation;
- side-difference energy;
- rear/object energy;
- bass anchoring;
- persistence/motion memory;
- source spread;
- near-field/boundary cues;
- Windows CI lessons.

Its topology is not the target:

```text
stereo
→ synthetic multichannel bed
→ external virtualizer
```

Omniphony can express useful ideas inside its own renderer instead of rebuilding the old stereo→multichannel→HeSuVi detour.

---

# 11. Windows is the product now

Earlier planning over-promoted cross-platform portability.

Correct current rule:

> **Build the Windows product first.**

Portability remains only a guardrail:

- keep OS device code above the renderer where practical;
- avoid unnecessary Windows semantics inside scene/HRTF math;
- keep a small host/engine boundary;
- do not spend current time implementing macOS/Linux/Android/iOS products.

Future relationship:

```text
NOW
Windows
→ real listening
→ iterate
→ beat incumbent

LATER, IF WORTH IT
same proven engine
→ another thin host
```

See `docs/PLATFORM_PORTABILITY.md` as a **boundary document**, not the active roadmap.

---

# 12. Current native Windows progress

W1 has already begun. This is saved code, not merely a proposal.

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
- packaging in the Windows artifact workflow.

The P0 prototype is deliberately bounded:

```text
bundled 7.1.4 reference WAV
→ reference_bridge
→ protected Omniphony binaural renderer
→ stereo PCM
→ realtime_ffi identity seam
→ native WASAPI output
```

This is the first listening object, not the final daily route.

Important limitation:

> **loopback is a copy, not an intercept.**

The dry Windows mix still reaches its normal endpoint.

Therefore replaying the processed capture to the same endpoint would create dry + processed playback.

So loopback is diagnostic/experimental only, not the final HeSuVi replacement.

## 12.2 `realtime_ffi` — EXISTS

Workspace crate:

```text
omniphony-renderer/realtime_ffi/
```

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

The first implementation is deliberately **bit-exact identity**.

That provides a transport oracle:

```text
input PCM
→ realtime_ffi
→ identical PCM
```

before the persistent realtime renderer is connected behind the boundary.

The ABI has tests and CI/package integration in the committed W1 batch.

## 12.3 What does NOT exist yet

Do not infer these from the scaffolding or P0:

- no completed transparent system-wide Omniphony route yet;
- no chosen production APO yet;
- no finished virtual render endpoint yet;
- loopback capture is not the final route;
- P0 renders the controlled reference through the real engine and then plays it through the identity seam, but the protected renderer is **not yet the persistent callback-time processor behind `realtime_ffi` for ordinary daily PCM**;
- no arbitrary-stereo music product path has been judged yet;
- no claim yet that native Omniphony has replaced the incumbent.

---

# 13. Windows single-path routing problem

The final product needs:

> ordinary Windows playback to reach the listener **once**, through Omniphony.

Current candidate families:

## A. Endpoint/system-effect APO

```text
application
→ Windows shared audio engine
→ Omniphony APO
→ physical endpoint
```

Potential advantages:

- true in-place single-path processing;
- normal apps use the ordinary endpoint;
- set-and-forget behavior similar in spirit to what made Equalizer APO/HeSuVi practical.

Risks/costs:

- realtime in-process COM/Windows audio-engine constraints;
- installation/signing/device association;
- crash containment;
- full renderer may need a carefully bounded realtime projection.

## B. Virtual render endpoint

```text
application
→ Omniphony virtual endpoint
→ Omniphony host process
→ realtime_ffi
→ renderer
→ physical endpoint
```

Potential advantages:

- explicit single path;
- renderer lives out of the Windows audio-engine process;
- maps naturally onto the Rust process/core boundary.

Risks/costs:

- driver/WDK/signing work;
- another visible endpoint;
- buffering/clock-domain complexity;
- product owns device switching/recovery.

## C. ASIO specialist route

ASIO remains useful for specialist hardware/workflows and as a compatibility/reference route.

It is not sufficient as the ordinary system-wide solution by itself.

Correct product relation:

```text
normal Windows route
→ native system integration

optional specialist route
→ ASIO
```

Microsoft Spatial Sound's public app APIs are also useful as a future **input semantic** for static beds/dynamic objects, but current reviewed documentation does not establish a simple public registration API for making an arbitrary renderer appear beside Sonic/Atmos/DTS in the Spatial Sound dropdown. Keep that product dream separate from what is currently proven implementable.

See `docs/WINDOWS_AUDIO_ROUTE.md` and `docs/WINDOWS_INTEGRATION_RESEARCH.md` for the detailed decision gates and parked platform findings.

---

# 14. Current renderer foundation

Do not rewrite useful inherited machinery for aesthetics.

Important retained substrate includes:

- stateful binaural DSP;
- analytic ITD;
- interpolated HRTF/HRIR rendering;
- embedded SAF/KEMAR, parametric and SOFA-capable providers;
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

## `upstream-demo-reference.yaml`

**Perceptual ancestor.**

Minimal stock-style approximation of the published upstream demo.

## `baseline-room.yaml`

**Fork room-assisted comparison.**

Useful 3 m / early-reflection / short-FDN comparison.

It does not outrank the upstream control merely because it contains more room processing.

## `dry-binaural.yaml`

Fork HRTF/scale/air policy with reflections and late reverb disabled.

Useful for isolating fork room contribution.

Experimental algorithms get separate explicit configs/flags.

Never overwrite a protected control to make a candidate look better.

---

# 16. Upstream active-branch sweep: already done

Do not repeatedly mine the same branch list unless upstream changes.

August 2026 findings:

- `feat/spectral-3d-phantom`: important implementation already byte-identical in this fork;
- `feat/diffuse-mirror-axes`: important implementation already byte-identical;
- `feat/workflow-runtime-isolation`: core runtime-isolation implementation already present;
- `ci/skip-unchanged-integration-build`: useful CI ideas but tied to old Studio/integration-release goals;
- `feat/release-0.4.2` / `release`: no hidden ahead-of-main Windows DSP payload found during the sweep;
- macOS signing branch: not relevant to current Windows milestone.

Upstream remains a technical ancestor and continuing source of exact fixes/mechanisms.

Use:

```text
inspect diff
→ check whether already present
→ import smallest useful missing part
→ validate against local product
```

See `docs/FORK_POLICY.md`.

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

Answers:

> if the scene is known, is the renderer correct/convincing?

## Lane C · Stereo evidence / scene hypothesis

```text
controlled stereo
→ evidence
→ persistence / confidence
→ scene state
```

Answers:

> did inference preserve evidence and avoid unsupported specificity?

## Lane D · Native Windows transport

```text
same PCM / engine
→ host route A
→ host route B
→ compare timing / glitches / latency / semantics
```

Answers:

> did Windows plumbing change the engine or merely carry it?

## Lane E · Renderer perceptual comparison

```text
controlled source/scene
→ upstream Omniphony reference
↔ fork candidate
```

Answers:

> did the renderer itself improve?

## Lane F · End-to-end product comparison

```text
CURRENT
foobar + VB-Audio + HeSuVi + FiiO ASIO

VERSUS

TARGET
native Omniphony + FiiO K7 / Noire X
```

Answers:

> would the listener actually stop using the old chain?

---

# 18. Listening scorecard

Score dimensions separately rather than collapsing everything into one “spatial” rating:

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

Loudness-match comparisons.

Do not normalize candidates independently in a way that turns level into “quality.”

---

# 19. Realtime law

One important engineering rule survived the research/refactor phase and remains correct:

> **host callback size is an implementation detail, not a coordinate system for the auditory world.**

Gain, movement, HRTF transitions, room changes and other continuous state should be defined in sample/time coordinates when they are supposed to be continuous.

The post-August renderer/core CI has passed the currently mandatory callback-related gate used in that workflow, as visually verified by the repository owner.

Do **not** generalize that into “all motion is solved.”

Position/HRTF movement remains a separate candidate defect where block-start publication can still matter.

See `docs/REALTIME_CONTROL_CONTRACT.md` and `docs/SCENE_RENDERER_CONTRACT.md`.

---

# 20. CI status and durable repair history

Important baseline/context-repair sequence:

```text
7bc97aca  fix(rt): reset stream-lifetime DSP state in place
6fe74dd9  ci: rerun baseline renderer repair after stream reset
1700226f  fix(binaural): align measured HRIR validation and KEMAR baseline
6e9ccf7d  docs: make artificial hearing a compiled research input
73488c25  test(renderer): make backend file paths host-native
e6d978ea  test(audio): preserve upstream demo-style binaural reference
caf9372a  docs(audio): separate upstream demo oracle from fork room tuning
4b8dd970  docs(audio): stop room-tail control from claiming upstream baseline
```

The failure at `6e9ccf7d` was caused by Unix absolute-path assumptions in renderer backend-file tests on Windows, not by the DSP.

`73488c25` made those tests host-native.

**The post-fix Actions result was visually verified green by the repository owner on 2026-08-10.**

Treat that incident as closed unless a new run regresses.

W1 Windows-native progress after that checkpoint includes:

```text
077f15ac  add WASAPI-first native host crate
6fd1ea26  add native WASAPI transport probe
0bf5dcff  CI gate native WASAPI host without ASIO SDK
6dbc0267  add self-excluding WASAPI loopback probe
409a2f58  expose manual process-loopback probe
2a217f4e  mark loopback diagnostic-only
abacabe0  package native transport probe with engine
fb1cc719  define single-path routing / APO decision gates
c87a4a8d  add realtime PCM FFI crate
30534230  define bit-exact realtime PCM boundary
fd77a7bc  publish C header
ee9d2b5b  register realtime PCM FFI workspace member
17ca30d5  test/package realtime PCM ABI
97688264  fix Windows COM HRESULT conversion for loopback host
bcaff271  add audible realtime WASAPI output smoke path
f56b5e54  play protected Omniphony reference over WASAPI
35719439  align P0 code with CPAL 0.15 sample formats
a78d4316  preserve external audio influence ledger
77df8f04  park Core Audio / endpoint integration findings
ad98ec33  package first audible Windows reference prototype
```

These commits are durable progress.

The connector used for this project does not expose push-triggered Actions runs through its commit-run wrapper, so a newly pushed P0 run must not be described as green until its result is actually observed. The earlier verified-green statement above specifically closes the renderer/backend-path incident.

---

# 21. Current implementation frontier

## W0 · Protect/reproduce the renderer sound — ESTABLISHED

- upstream-demo-style local perceptual control exists;
- fork room control is explicitly not the perceptual ancestor;
- deterministic known-scene/file routes remain;
- Windows backend path test failure is fixed;
- baseline Actions checkpoint is owner-verified green.

## W1 · Coexisting native Windows listening lane — IN PROGRESS

Already built:

```text
windows_host
realtime_ffi identity seam
Windows output-device discovery
self-excluding loopback diagnostic probe
native WASAPI output smoke mode
protected upstream-reference render/playback mode
self-contained P0 Actions artifact packaging
single-path route decision document
CI/artifact integration for host/ABI scaffolding
```

Current P0 acceptance sequence:

```text
1. Actions compiles/packages current main
2. run windows_host.exe --smoke-output on the intended endpoint
3. run windows_host.exe --reference-demo
4. confirm the protected reference is audible, stable and recognizably Omniphony
5. compare against the hosted upstream perceptual ancestor
```

Then the next concrete engineering steps are:

```text
1. move from controlled offline reference render + native playback to persistent realtime rendering behind the same host seam
2. prove callback/stream output matches controlled reference semantics
3. add the simplest ordinary-stereo music input path without experimental DSP
4. establish fast incumbent ↔ Omniphony A/B
5. prototype the smallest viable single-path system route
6. compare APO / virtual-endpoint approaches only as needed
7. harden WASAPI transport from evidence; direct wasapi-rs is a parked candidate
```

No new external DSP should be required merely to prove this route.

## R1 · Fix actual renderer weaknesses exposed by W1/A-B

Candidates, not mandatory queue:

- sample-time position/HRTF motion;
- source extent / BroadSource behavior;
- directional early-reflection consistency;
- front/back/elevation robustness;
- direct/room separation;
- low-frequency integrity.

## S1 · Small persistent stereo scene

Once native listening is easy enough to judge:

- wire current stereo evidence into bounded persistent state;
- preserve center/foundation;
- distinguish direct/broad/diffuse only where useful;
- allow meaningful rear/depth presentation without turning everything into room wash;
- remain conservative under uncertainty.

## P1 · Optional music-aware presentation

Later:

- add artistic degrees of freedom one at a time;
- use local evidence where sufficient;
- introduce libaural where it demonstrates a measurable/listenable advantage;
- keep protected no-adaptive route for attribution.

## C1 · Calibration/personalization

Later:

- per-device profiles;
- optional headphone correction;
- HRTF selection/import;
- headroom management;
- deeper driver-ear/BRIR personalization only after the core experience is strong.

## X · Other operating systems

Deferred until Windows is a product worth porting.

---

# 22. Product anti-goals

## Do not replace good sound with research

```text
new theory
→ rewrite renderer
```

Wrong.

Use:

```text
actual weakness
→ smallest tested mechanism
```

## Do not build a settings forest

Power belongs under good defaults and bounded profiles.

## Do not detour into cross-platform shells

Windows comes first.

## Do not hallucinate object truth

Presentation is not recovered metadata.

## Do not equate reverb with 3D

Direct source, source extent, diffuse musical field and room are separate jobs.

## Do not make AI availability a playback dependency

Baseline Omniphony must remain independently useful.

## Do not let infrastructure consume the product

Build enough host/driver/ABI machinery to enable the next meaningful listening comparison.

Then listen.

## Do not rewrite history because a newer plan is cleaner

Keep checkpoints, baselines and failed experiments legible.

---

# 23. Repository contraction law

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

Already removed/retired inherited surfaces include Studio-centric product code, obsolete suite packaging/release surfaces, JACK/mpv product routes, generic demonstration/script backends and historical implementation diaries.

Do not delete reference layouts, deterministic fixtures, protected configs, useful ASIO support, or renderer internals merely because the final listener UI will never expose them.

See `docs/CONTRACTION_LEDGER.md`.

---

# 24. Documentation precedence

This README owns:

- product identity;
- current listener/incumbent context;
- baseline hierarchy;
- migration law;
- roadmap priority;
- research/libaural boundary;
- current phase status.

Supporting docs own narrower technical contracts.

Important current docs:

- `docs/WINDOWS_AUDIO_ROUTE.md` — single-path Windows route, APO vs virtual endpoint and transport ladder;
- `docs/WINDOWS_INTEGRATION_RESEARCH.md` — parked Microsoft Core Audio/Spatial Sound, endpoint/APO and driver-integration findings;
- `docs/INFLUENCE_LEDGER.md` — durable external GitHub/research findings, including adopted-vs-parked status so source dives do not evaporate with chat context;
- `docs/headphone-rendering-research.md` — practical renderer experiments and Windows listening plan;
- `docs/SCENE_RENDERER_CONTRACT.md` — evidence/scene/rendering distinctions and current renderer gaps;
- `docs/REALTIME_CONTROL_CONTRACT.md` — sample-time/realtime correctness without taking over roadmap priority;
- `docs/PLATFORM_PORTABILITY.md` — deferred portability guardrail, not active platform roadmap;
- `docs/MUSIC_PRESENTATION_CONTRACT.md` — optional future adaptive-presentation rules;
- `docs/HEADPHONE_CALIBRATION.md` — later listener/headphone calibration architecture;
- `docs/FORK_POLICY.md` — upstream ancestry and selective integration law;
- `docs/CONTRACTION_LEDGER.md` — current crate/surface ownership;
- `omniphony-renderer/assets/binaural-baselines/README.md` — protected sound controls and A/B procedure;
- `CONTRIBUTING.md` — private development rules for us/Codex/tooling.

If a supporting document conflicts with this README's product priority, the README wins until explicitly revised.

Do **not** create another master-plan document beside this one.

Amend this README when the product itself changes.

---

# 25. Working development loop

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

This is the preferred loop.

It is intentionally resistant to research drift.

---

# 26. Re-entry checkpoint

If conversational context is lost, do not reconstruct the project from memory.

Start with:

1. this README;
2. recent commits on `main`;
3. `docs/WINDOWS_AUDIO_ROUTE.md` for the live native-host frontier;
4. `docs/INFLUENCE_LEDGER.md` when recovering external research/influences;
5. `docs/WINDOWS_INTEGRATION_RESEARCH.md` for system-route/Spatial Sound/APO questions;
6. the protected binaural baselines;
7. the real incumbent snapshot in this file.

The durable hierarchy is:

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