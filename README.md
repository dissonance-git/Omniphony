# Omniphony

> **Private master project plan.**
>
> This README is the canonical re-entry surface for the fork. It is intentionally written for development rather than marketing. If chat context disappears, research sprawls, or a future refactor starts optimizing the wrong object, recover the project from here first.

Omniphony is a fork of [`mgth/Omniphony`](https://github.com/mgth/Omniphony) being turned into a **native Windows headphone spatial-audio system for ordinary music**.

The project exists for a very specific reason:

> **Upstream Omniphony already sounds unusually good. Preserve that spatial character, make it practical for normal Windows listening, and improve it only when an improvement earns itself.**

The immediate job is not to invent a new artificial-hearing architecture, not to become a cross-platform framework, and not to replace proven DSP because a cleaner abstraction exists.

The immediate job is:

```text
ordinary Windows audio
        ↓
Omniphony's already-good spatial foundation
        ↓
carefully validated improvements
        ↓
headphones
```

with enough native Windows plumbing that the current HeSuVi/VB-Audio chain can eventually disappear.

The long-term aspiration is intentionally extreme: after acclimation, ordinary headphone playback should feel lower-dimensional by comparison. The engineering constraint is equally strict: **dimension may not be purchased by damaging the music.**

---

# 1. The object we are preserving

The fork did not begin from a generic requirement such as "make stereo wider."

It began because the upstream Omniphony headphone demo already produced a convincing spatial volume. The rotating test sound felt like a real orbit around the listener rather than a flattened left/right trick. That existing effect is the reason this repository is worth developing.

Therefore the upstream sound is not disposable scaffolding.

```text
UPSTREAM OMNIPHONY
already convincing binaural / 360° behavior
        ↓
PERCEPTUAL FLOOR
must remain reproducible
        ↓
FORK CHANGES
must preserve or improve it
        ↓
PRODUCT
```

A technically sophisticated fork that loses the original sense of acoustic volume has failed.

A research result that sounds worse has failed.

A cleaner architecture that weakens the listening experience has failed.

A new mechanism is allowed to replace old behavior only after it demonstrates that the old behavior is the limitation and that the replacement preserves the useful perceptual properties.

The practical rule is:

```text
new idea
→ isolate it
→ keep the old path available
→ objective regression checks
→ matched-loudness listening
→ compare intended gain AND unintended loss
→ promote only if it earns the default
```

---

# 2. Two references, not one

Omniphony has **two different baselines**. Confusing them causes bad decisions.

## 2.1 Upstream Omniphony = renderer perceptual ancestor

The hosted upstream headphone demo is the primary listening oracle for the renderer's starting character.

The published local approximation is:

```text
stock Omniphony renderer
+ embedded SAF/KEMAR HRTF
+ early reflections enabled
+ no fork-added late reverb
```

Local control:

```text
omniphony-renderer/assets/binaural-baselines/upstream-demo-reference.yaml
```

The upstream website does not pin the exact commit/render command behind the hosted demo, so the local file is a reproducible approximation of the published contract, not a claim of byte identity.

The important thing is perceptual ancestry: **a richer fork configuration is not automatically a better Omniphony.**

## 2.2 Current Windows setup = end-to-end incumbent

The second reference is the listener's actual daily playback system.

Omniphony ultimately has to make that chain unnecessary.

That means the project does not graduate merely because it beats dry stereo or a default HRTF test. It must eventually be preferable to a long-tuned real setup on the actual headphones and DAC/amp.

---

# 3. Current incumbent listening chain

Snapshot supplied from the real Windows setup on **2026-08-10**.

This is evidence and a benchmark. It is **not** a mandate to clone every stage.

## 3.1 foobar2000 DSP chain

Current order:

```text
Resampler (SoX)
→ Skip Silence (alternative)
→ Vocal Exciter
→ Reverb
→ Upmix to 5.1/side
→ Advanced Limiter
```

The active upmix produces:

```text
FL FR C LFE SL SR
```

Older project/profile material may mention `FreeSurround Decoder` as the active upmixer. That is historical lineage, not the current 2026-08-10 snapshot. FreeSurround remains useful as a negative comparison because earlier listening found that it could collapse/flatten the desired 3D bubble.

## 3.2 Virtual multichannel transport

Current transport uses **VB-Audio ASIO Bridge / Hi-Fi Cable**.

Observed configuration:

```text
input transport:  8 channels
sample rate:      48,000 Hz
resolution:       24-bit
ASIO device:      FiiO ASIO Driver
ASIO output:      2 channels
buffer:           512 samples
ASIO sample rate: 48,000 Hz
```

The foobar upmix itself is six active channels inside that multichannel route.

## 3.3 HeSuVi / Equalizer APO

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

These values describe the incumbent. They are not automatically Omniphony target parameters.

## 3.4 Hardware reference

```text
FiiO K7
→ Dan Clark Noire X
```

This hardware matters because the renderer must scale upward with revealing headphones. Better transducers should expose **more scene and musical information**, not more phase smear, reverb haze, tonal damage, or unstable localization.

---

# 4. What the incumbent teaches us

The old chain is a record of perceptual problems that were solved through accumulated workarounds.

Do not cargo-cult its implementation.

The project does **not** need literal permanent equivalents of:

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

> **What audible function was this stage buying, and can Omniphony provide that function more directly, more coherently, and with less plumbing?**

If one native Omniphony path produces a stronger result than six chained components, the six-component topology should disappear.

The incumbent therefore behaves as a perceptual requirements mine:

- a large, convincing acoustic volume is desirable;
- behind-head presentation is acceptable and valuable when stable;
- bass weight matters strongly;
- music must retain punch and identity;
- ordinary stereo music is the primary source;
- a daily-use system must be set-and-forget enough to live in the playback path;
- complexity behind the UI is acceptable, complexity demanded from the listener is not.

---

# 5. Target product

The practical product is deliberately narrower than some earlier research documents implied.

```text
WINDOWS AUDIO SOURCE
system / player / foobar / game
        ↓
NATIVE WINDOWS INPUT
ordinary PCM first
        ↓
OMNIPHONY REALTIME CORE
existing upstream renderer behavior
+ bounded scene state
+ only proven fork improvements
        ↓
BINAURAL OUTPUT
        ↓
WINDOWS OUTPUT DEVICE
        ↓
FiiO K7 / headphones
```

The first successful product does **not** require:

- a giant AI model running on the audio thread;
- perfect source separation;
- a complete auditory world model;
- mobile ports;
- a driver written from scratch;
- a research dashboard;
- per-song manual authoring;
- dozens of exposed spatial knobs;
- replacement of the current listening chain before Omniphony is ready.

The normal UX should eventually be boring:

```text
install
→ choose output / headphones
→ enable
→ play music normally
```

A/B and diagnostics can be richer in development builds, but the product is not supposed to become a cockpit.

---

# 6. Migration law: coexist first, replace later

There is **no cold-turkey migration**.

The current foobar + HeSuVi + VB-Audio + ASIO route stays intact while Omniphony grows beside it.

```text
CURRENT CHAIN
known-good daily listening
        │
        ├──────────── remains available ────────────┐
        │                                            │
        ▼                                            │
OMNIPHONY DEVELOPMENT                                │
        │                                            │
        ├→ deterministic renderer tests              │
        ├→ upstream-demo reference A/B                │
        ├→ incumbent-chain A/B                        │
        ├→ native Windows realtime listening          │
        └→ matched-loudness adjudication              │
        │                                            │
        ▼                                            │
Omniphony clearly earns a function                   │
        │                                            │
        └→ old component becomes redundant ──────────┘
```

Never require dismantling the working setup merely to test an engineering milestone.

The first normal listening build should already be worth listening to. Internal milestones can be ugly; user-facing milestones should not demand disruption for no audible reward.

---

# 7. Perceptual acceptance law

The desired result is not merely "wider."

The desired result is a coherent three-dimensional listening world with independent front/back, height, radial depth, source extent and ambience, while the master remains musically authoritative.

At matched loudness, bypass should ideally collapse:

```text
perceived acoustic volume
externalization
front/back organization
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

If bypass sounds clearer, tighter, more tonally convincing, or more emotionally coherent, the spatial path has paid too much for its effect.

A huge bubble with smeared music is a failure.

A dramatic rear image with weak center authority is a failure.

A clever room model that turns transients to fog is a failure.

A subtler improvement that preserves the song is better than a spectacular effect that makes the recording fight the DSP.

---

# 8. Renderer vocabulary

Useful fork concepts remain valid, but they are tools for rendering and presentation, not proof that a stereo master contained hidden authored objects.

```text
FrontalAnchor
DirectObject
BroadSource
DiffuseField
RoomField
```

Keep these distinctions:

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

The distinctions matter because the old HeSuVi-style route demonstrated that rear energy can be valuable, but Omniphony should eventually create that world more coherently than simply throwing decorrelated energy behind the listener.

At the same time, do not force every useful stereo phenomenon into a fully inferred object graph before a native product exists.

---

# 9. Research boundary

The broad research phase produced useful ideas. It does **not** own the product.

Influences include, among others:

- Steam Audio;
- Dolby open-source work;
- SPARTA / IEM / Ambisonic tooling;
- ImmersiveFlow and learned spatial-audio approaches;
- HRTF/BRIR and psychoacoustic literature;
- QuadraphonicQuad and historical multichannel practice;
- artificial-hearing and auditory-neuroscience research;
- source-separation / MIR / music-cognition work;
- the older `spatial-dsp` experiment;
- realtime audio and host architecture from mature DAWs/engines.

Their correct role is:

```text
missing capability or audible weakness
        ↓
search research / implementations
        ↓
identify exact candidate mechanism
        ↓
small isolated experiment
        ↓
measure + listen
        ↓
keep only if earned
```

Do not research because the influence list can always become longer.

Do not let academic novelty outrank an existing good percept.

Do not let a learned model become mandatory merely because it is impressive.

External projects are mechanism sources and benchmarks, not architecture votes.

---

# 10. libaural relationship

`libaural` is a separate artificial-hearing research/framework project.

It may become an important source of better auditory evidence and presentation decisions **later**.

It is not the prerequisite for making Omniphony useful.

Correct relationship for current development:

```text
Omniphony today
existing renderer
+ local bounded evidence
+ native Windows product work
        │
        │ works independently
        ▼
useful listening product
        │
        │ optional richer evidence when proven
        ▼
libaural-informed improvements
```

Not:

```text
Omniphony
→ wait for general artificial hearing
→ rebuild around libaural
→ eventually recover the original good sound
```

Rules:

- libaural never replaces the upstream perceptual baseline by fiat;
- Omniphony must remain runnable without a giant hearing stack;
- any libaural contribution should enter through a bounded state/projection rather than by moving research machinery onto the realtime thread;
- libaural observations are evidence for presentation, not spatial commands;
- a useful local heuristic may remain if it is cheaper, clearer and sounds better;
- research graduates only when the product benefits.

The fork can teach libaural too. A rendering result does not automatically become a general law of hearing, but Omniphony is a valuable practical testbed for discovering which auditory distinctions matter.

---

# 11. `spatial-dsp` relationship

The older `spatial-dsp` / Real3D-style work is **lineage and a mechanism mine**, not the architecture to reproduce.

Useful ideas harvested from it include concepts such as:

- direct/diffuse evidence;
- coherent center preservation;
- side-difference energy;
- rear/object energy;
- bass anchoring;
- persistence/motion memory;
- source spread;
- near-field/boundary cues;
- practical Windows CI lessons.

Its old topology is not the target:

```text
stereo
→ synthetic multichannel bed
→ external virtualization
```

Omniphony's advantage is that those useful perceptual ideas can be expressed inside a native binaural/object renderer without recreating the detour.

Mine mechanisms selectively. Do not resurrect the old project as a hidden dependency.

---

# 12. Windows is the product now

Earlier planning over-promoted cross-platform portability.

Correct current priority:

> **Build the Windows product first.**

Portability remains an engineering guardrail only:

- keep the renderer core free from unnecessary OS-specific semantics;
- keep Windows capture/output above the headless core;
- avoid irreversible assumptions when a neutral contract is cheap;
- do not spend current project time implementing macOS, Linux, Android or iOS shells.

The second platform is a future decision made after the Windows product earns itself.

That means:

```text
NOW
Windows
→ real listening
→ product iteration
→ beat incumbent

LATER, IF WORTH IT
same proven core
→ another thin host shell
```

Portability must not delay the first useful Windows listening path.

See `docs/PLATFORM_PORTABILITY.md`, which should be read as a **boundary/guardrail document**, not the active roadmap.

---

# 13. Windows audio strategy

The inherited host path contains useful ASIO support, but the product should not require a specialist ASIO topology for ordinary users.

Target relationship:

```text
normal Windows path
→ native Windows system audio

optional specialist path
→ ASIO
```

For this listener specifically, the FiiO ASIO route may remain useful and should not be deleted merely to make the architecture ideologically pure.

The migration order should be empirical:

1. establish a native Windows path that can coexist with the incumbent;
2. preserve the current ASIO/FiiO route where useful for direct testing;
3. implement ordinary Windows output without requiring the Steinberg path;
4. implement practical Windows capture/loopback or a virtual-endpoint route as needed;
5. compare latency, stability and sound;
6. keep both paths if they serve different users/hardware.

Do not turn "WASAPI is the normal Windows API" into "ASIO must disappear."

Do not turn "the listener already uses ASIO" into "every future user must install an ASIO bridge."

---

# 14. Current renderer foundation

Do not rewrite useful inherited machinery for aesthetics.

Important retained substrate includes:

- stateful per-channel binaural DSP;
- analytic ITD;
- interpolated HRTF/HRIR rendering;
- embedded SAF KEMAR, parametric and SOFA-capable providers;
- moving-filter crossfades;
- object position/size state;
- known-scene VBAP/layout machinery used for calibration and regression truth;
- early image-source reflections;
- late FDN room-field machinery;
- deterministic DSP fixtures;
- headless engine / FFI boundaries.

Fork work already contains or has absorbed:

- complex M/S stereo evidence;
- persistence-aware stereo evidence;
- symmetric object/field evidence separation;
- bass protection separated from object identity;
- conservative scene-evidence inference;
- deterministic async HRTF switching and stale-result rejection;
- measured-HRIR direct-arrival validation;
- per-ear early-reflection delays / directional timing work;
- sample-time-oriented FDN modulation;
- true zero predelay;
- reusable null/RMS/crest/DC/level fidelity metrics;
- optional upstream spectral phantom extraction;
- optional distance-diffuse mirror-axis behavior;
- upstream runtime-isolation mechanics.

The active upstream branches `feat/spectral-3d-phantom`, `feat/diffuse-mirror-axes`, and the core of `feat/workflow-runtime-isolation` were checked against this fork in August 2026 and their important implementation files were already byte-identical here. Do not repeatedly "merge" them because the branch names still exist upstream.

---

# 15. Protected binaural controls

Directory:

```text
omniphony-renderer/assets/binaural-baselines/
```

Current controls:

### `upstream-demo-reference.yaml`

Perceptual ancestor. Minimal stock-style local approximation of the published upstream demo:

- binaural output;
- SAF/KEMAR;
- early reflections on;
- late reverb off;
- no experimental fork stack silently added.

### `baseline-room.yaml`

A **fork room-assisted comparison**, currently using the fork's 3 m / early-reflection / short-FDN tuning.

It is not the perceptual ancestor and may not silently become the product floor merely because it contains more DSP.

### `dry-binaural.yaml`

Fork HRTF/scale/air policy with early reflections and late reverb disabled. Useful for isolating room contribution.

Experimental algorithms should use separate explicit configs/flags. Never overwrite an accepted control to make a candidate look better.

---

# 16. Two mandatory listening lanes

A major source of drift was mixing renderer quality with whole-product quality.

Keep them separate.

## Lane A: renderer comparison

Purpose: isolate whether the binaural renderer itself improves.

```text
known / controlled scene
        ├→ upstream Omniphony reference
        ├→ fork candidate
        └→ strong external virtualization reference where practical
```

Measure/listen for:

- front externalization;
- rear discrimination;
- side precision;
- elevation;
- radial distance;
- apparent source width;
- listener envelopment;
- source extent;
- source separation;
- source stability;
- room presence / scale;
- ambient continuity;
- transient clarity;
- direct/vocal solidity;
- timbral fidelity;
- bass stability;
- fatigue.

Question answered:

> **Did the renderer get better?**

## Lane B: end-to-end product comparison

Purpose: decide whether the native product actually replaces the incumbent.

```text
CURRENT
foobar DSP
→ VB-Audio multichannel route
→ HeSuVi / DTS Virtual:X
→ FiiO K7
→ Noire X

VERSUS

TARGET
ordinary Windows audio
→ Omniphony native realtime path
→ FiiO K7
→ Noire X
```

Question answered:

> **Would the listener actually prefer to stop using the old chain?**

A renderer win does not imply a product win. The incumbent may contain useful exciter, room, bass, upmix or routing behavior that still needs to be understood or replaced.

---

# 17. Objective validation lanes

Listening is mandatory but not sufficient for engineering attribution.

Keep independent lanes:

## A. Compiler / deterministic DSP

```text
compile
→ unit tests
→ deterministic fixtures
→ null/fidelity tests
→ callback-size invariance
```

## B. Known scene → binaural

```text
known geometry
→ HRTF / ITD
→ extent / fields
→ early room / late room
→ binaural output
```

## C. Stereo evidence / scene inference

```text
controlled stereo
→ evidence
→ persistence / confidence
→ scene state
```

No renderer changes allowed to hide an inference failure.

## D. Music → presentation

```text
real song
→ current bounded evidence/policy
→ render
→ fidelity metrics
→ matched-loudness listening
```

libaural can later supply richer evidence to this lane without changing the existence of the lane.

## E. Windows transport

```text
same engine
→ Windows host path A
→ Windows host path B
→ compare glitches / timing / latency / device behavior
```

Platform transport should not silently change the intended auditory world.

---

# 18. Time and realtime law

The good engineering insight from the callback-invariance work remains valid:

> **Host callback size is an implementation detail, not a coordinate system for the auditory world.**

Gain, position, movement, HRTF transitions, room changes and other continuous state should be defined in sample/time coordinates rather than inheriting arbitrary buffer boundaries.

This does not mean callback-invariance research outranks the product. It means the realtime engine should not sound different because an audio host chose a different chunk size.

Position/HRTF motion still deserves an explicit sample-time trajectory path where the inherited block-start publication remains quantized.

Fix such defects when they threaten audible correctness or block the native path, without letting them become an endless prerequisite ladder.

---

# 19. CI status and repair history

Current workflow:

```text
.github/workflows/windows-renderer.yml
```

It validates the retained renderer/core on clean CI and produces the Windows headless renderer-engine artifact.

Important August 2026 repair sequence:

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

The Windows path-test failure at `6e9ccf7d` came from Unix absolute-path assumptions in `renderer/src/backend_files.rs`, not from the renderer DSP. Commit `73488c25` made those tests host-native.

**The post-fix GitHub Actions run has been visually verified green by the repository owner on 2026-08-10.** Treat that incident as closed unless a new run regresses.

Do not keep old README language saying the repair is still waiting to earn green status.

The headless artifact intentionally avoids making the separately licensed Steinberg ASIO SDK a requirement for validating the renderer core.

---

# 20. Upstream relationship

`mgth/Omniphony` remains:

- the permanent ancestry and attribution source;
- a source of mature renderer fixes;
- a source of useful active-branch experiments;
- a peer whose changes should be inspected selectively.

It does not define this fork's roadmap.

Use:

```text
upstream mechanism / fix
→ inspect exact diff
→ determine whether already present
→ import only useful missing behavior
→ validate against local product constraints
```

Do not merge broad upstream product work merely to keep histories visually synchronized.

Do not delete useful inherited audio behavior merely because the fork's final UX is narrower.

Recent active-branch sweep result:

- `feat/spectral-3d-phantom`: important implementation already present byte-for-byte;
- `feat/diffuse-mirror-axes`: important implementation already present byte-for-byte;
- `feat/workflow-runtime-isolation`: core runtime-isolation implementation already present;
- `ci/skip-unchanged-integration-build`: useful CI ideas, but its Studio/integration-release workflow is not the current product;
- `feat/release-0.4.2` / `release`: no hidden ahead-of-main Windows DSP payload found in the sweep;
- macOS signing branch: irrelevant to the current Windows milestone.

Stop treating the branch list as an unmined treasure chest unless upstream changes again.

See `docs/FORK_POLICY.md`.

---

# 21. Repository contraction law

The fork should become easier to understand, but deletion is not a sport.

Keep code when it serves at least one of:

```text
1. Windows stereo → binaural daily playback
2. retained realtime renderer behavior
3. deterministic renderer / known-scene testing
4. HRTF / calibration truth
5. current migration / A-B observability
6. a clearly isolated experiment with an active question
```

Remove or retire code whose only owner is an abandoned upstream product surface.

Already removed/contracted surfaces include Studio-centric product code, old packaging/release surfaces, JACK/mpv-oriented product routes, generic demonstration/script backends, and obsolete implementation diaries.

Do not delete known-scene layouts, test assets, renderer internals, or old-path controls merely because the final listener will never see them.

Observability is part of the product-development machinery.

See `docs/CONTRACTION_LEDGER.md`.

---

# 22. Calibration and audiophile scaling

The mature product should work well on ordinary headphones and scale upward on revealing equipment.

Keep these concepts separate internally:

```text
listener HRTF
headphone response
driver ↔ ear interaction
presentation / room target
low-frequency integration
headroom
```

Do not make advanced calibration a prerequisite for the first native Windows listening build.

Current hardware reference is the Noire X + K7. That is useful precisely because a resolving chain is good at exposing false spaciousness, tonal damage, transient softening and unstable phase behavior.

Desired scaling law:

```text
better headphones / DAC chain
→ more access to the same coherent scene
→ more microdetail / localization / dynamics

NOT

better headphones
→ more obvious DSP artifacts
```

See `docs/HEADPHONE_CALIBRATION.md` for later calibration architecture.

---

# 23. Product anti-goals

Do not let the project drift into any of these:

### Research replacement

```text
"we learned more about hearing"
→ rewrite the renderer from scratch
```

No. Research amends proven sound.

### Settings forest

```text
power
→ expose every internal scalar to the listener
```

No. Defaults and bounded profiles should carry the complexity.

### Cross-platform detour

```text
Windows is working
→ stop and build five shells before listening
```

No. Portability is deferred.

### Fake object certainty

```text
stereo evidence
→ claim recovered authored object metadata
```

No. Presentation choices and source truth remain distinct.

### Reverb as spatiality

```text
more tail
→ call it more 3D
```

No. Direct source geometry, source extent, ambient field and room are different perceptual jobs.

### AI dependency

```text
no model
→ no Omniphony
```

No. The good existing renderer must remain independently useful.

### Infrastructure before listening

```text
perfect packaging / abstractions / driver framework
→ someday hear it
```

No. Build enough infrastructure to make the next meaningful listening comparison possible.

---

# 24. Development phases from here

The order below supersedes older plans that placed general libaural integration or multi-platform shells ahead of the native Windows product.

## Phase W0 · Protect and reproduce the sound — MOSTLY COMPLETE

- retain upstream renderer machinery;
- establish local upstream-demo perceptual control;
- separate fork room tuning from the upstream reference;
- retain deterministic file/known-scene rendering;
- repair Windows CI regressions;
- confirm green CI.

Current status: the reference controls exist and the latest Windows Actions repair is owner-verified green.

## Phase W1 · First native Windows listening lane — NEXT

Goal: make Omniphony easy to hear on the real machine without dismantling the incumbent.

Priorities:

1. inventory the inherited `host_audio` / `audio_input` / `audio_output` paths against the actual Windows need;
2. retain ASIO as a useful specialist/test route;
3. add a normal Windows output route that does not require ASIO;
4. add practical ordinary-audio input/capture for the first listening path;
5. keep the headless renderer core unchanged behind platform transport;
6. expose simple enable/bypass and device selection sufficient for development listening;
7. prove stable 48 kHz realtime playback on the K7/Noire X path.

This phase should be engineered around coexistence with the current system.

## Phase W2 · Establish the product A/B harness

- rapid matched-loudness incumbent ↔ Omniphony switching where practical;
- stable test playlist / passages;
- upstream-demo reference renders;
- level/headroom logging;
- glitch/underrun/latency diagnostics;
- written listening dimensions rather than one vague score.

The listener should spend time hearing differences, not rebuilding the environment for every comparison.

## Phase R1 · Repair only renderer defects that block the target

Candidates include:

- sample-time position/HRTF motion;
- source extent reaching the binaural stage;
- early-reflection directional consistency;
- front/back/elevation robustness;
- room versus direct-source separation;
- low-frequency integrity.

Do not implement every candidate merely because it exists on this list.

The upstream perceptual floor remains the control.

## Phase S1 · Ordinary stereo → useful persistent scene

Once the native path is listenable:

- connect current stereo evidence to a small stable realtime scene;
- preserve center/foundation relationships;
- distinguish compact/direct from broad/diffuse where evidence is strong;
- allow real rear/depth presentation without turning everything into room wash;
- prefer conservative/reversible behavior under uncertainty.

Start with the smallest scene model that audibly beats static heuristics.

## Phase P1 · Music-aware presentation

Only after the renderer and native path are strong enough to judge it:

- test which musical relations actually benefit from adaptive presentation;
- preserve groove, center authority, bass, transients and hierarchy;
- add degrees of artistic freedom one at a time;
- use libaural evidence where it is better than local bounded analysis;
- keep a no-libaural route for attribution.

## Phase C1 · Headphone/listener calibration

- persistent per-device profile;
- optional headphone correction;
- HRTF selection/import;
- headroom prediction;
- only later, driver-ear / BRIR / deeper personalization experiments.

## Phase X · Other platforms

Deferred until Windows is a product worth porting.

---

# 25. Immediate next work

Unless a new regression appears, the next engineering question is **not** "what other spatial-audio research exists?"

It is:

> **What is the smallest safe change that gives this fork a normal, coexisting, native Windows listening lane while preserving the existing Omniphony renderer unchanged underneath it?**

Concrete inspection order:

```text
host_audio
→ audio_output Windows backend
→ audio_input Windows gap
→ device / stream contracts
→ coexistence with current ASIO route
→ first realtime stereo test
```

Then:

```text
hear it
→ compare against upstream reference
→ compare against incumbent
→ discover the next actual weakness
→ research only that weakness
```

This is the preferred development loop.

---

# 26. Documentation precedence

This README owns **project intent, priority, baseline hierarchy, migration law and roadmap**.

Supporting documents own narrower technical contracts.

If a supporting document says something broader than this README, interpret it through this README unless the project direction has been explicitly changed.

Important supporting docs:

- `docs/headphone-rendering-research.md` — practical renderer experiments and Windows listening path;
- `docs/PLATFORM_PORTABILITY.md` — host/core separation guardrails, **not an active multi-platform roadmap**;
- `docs/FORK_POLICY.md` — relationship to upstream;
- `docs/CONTRACTION_LEDGER.md` — retained/removed/transitional repository surfaces;
- `docs/MUSIC_PRESENTATION_CONTRACT.md` — optional future adaptive-presentation rules;
- `docs/HEADPHONE_CALIBRATION.md` — later listener/headphone calibration architecture;
- `docs/SCENE_RENDERER_CONTRACT.md` — scene/rendering semantics;
- `docs/REALTIME_CONTROL_CONTRACT.md` — realtime/sample-time ownership;
- `omniphony-renderer/assets/binaural-baselines/README.md` — protected sound controls and A/B procedure.

Do not create another "master plan" document beside this README. Amend this file when the product itself changes.

---

# 27. Re-entry checkpoint

As of **2026-08-10**, the important pre-plan-repair repository checkpoint was:

```text
4b8dd97038c3b76a3e892898059c11bde8a8fefa
```

with the key context-restoration sequence immediately before it:

```text
73488c25  host-native Windows backend path tests
e6d978ea  upstream demo-style perceptual reference
caf9372a  upstream oracle separated from fork room tuning
4b8dd970  room-tail config explicitly demoted from product floor
```

The post-`73488c25` Windows Actions result is owner-verified green.

If conversational context is lost, resume from the repository rather than reconstructing the project from memory.

The durable hierarchy is:

```text
1. preserve the already-good upstream Omniphony percept
2. build the native Windows replacement for the current HeSuVi pipeline
3. compare against the real incumbent on the real headphones
4. improve only where listening/testing reveals an actual weakness
5. use research and libaural later as optional sources of better mechanisms
6. never trade the music for the effect
```

That is Omniphony.