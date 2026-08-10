# Omniphony

**Portable real-time music enhancement: ordinary music → libaural-informed presentation → full-sphere binaural headphones.**

Omniphony is an independent research/product fork of [`mgth/Omniphony`](https://github.com/mgth/Omniphony).

Its north star is deliberately ambitious:

> **Make ordinary headphone playback feel as though a world-class mix / mastering / immersive-audio engineer were adapting each song for the listener in real time, with very low latency, into a coherent 360° listening world while preserving the recording.**

Windows is the **current development and listening-validation platform**. It is not the product boundary.

The intended mature system should support thin native shells on Windows, macOS, Linux, Android and iOS around one portable hearing/presentation/rendering core.

---

## What Omniphony is trying to do

Not this:

```text
stereo
→ make it wider
→ add rear reverb
→ headphones
```

Closer to this:

```text
ordinary song
      ↓
libaural hears what the music is doing
      ↓
Omniphony decides what must be protected,
what can gain separation,
and what can safely expand into space
      ↓
anchors / direct objects / broad sources /
diffuse musical fields / room
      ↓
full-sphere binaural rendering
      ↓
listener + headphone calibration
      ↓
headphones
```

The metaphor is an engineer, not an effect preset.

A good engineer does not maximize width everywhere. They protect the center of gravity, groove, intimacy, punch, transient shape, tonal identity and musical hierarchy while using space deliberately.

Omniphony tries to automate that kind of **presentation intelligence** continuously during ordinary playback.

It cannot recover a missing multitrack session or an artist's private intention. The practical target is artistically faithful inference from the audible recording: the enhanced presentation should feel purpose-built for the song rather than imposed on it.

---

## Preserve the good starting point

This fork is not a rewrite contest.

The upstream Omniphony renderer is the foundation because it already contains useful spatial-audio engineering and an existing listening baseline. Fork work should improve that foundation rather than replace proven behavior merely because a different architecture looks cleaner on paper.

Therefore:

```text
new mechanism
→ prove the old path is insufficient
→ isolate the intended improvement
→ preserve unrelated audible behavior
→ objective regression checks
→ matched-loudness listening
→ only then become the new default
```

A technically sophisticated change that makes music sound worse is a regression.

A cleaner abstraction that adds glitches is a regression.

A more dramatic spatial image that weakens punch, tone, timing or musical focus is a regression.

> **Enhancement means strict addition of useful perception, not exchanging one good property for another.**

---

## Product experience

The eventual UX should be boring:

```text
install
→ choose / detect headphones
→ calibrate or select profile
→ play music normally
```

No stem export.
No manually authored scene.
No per-song ritual.
No requirement that the listener understand the internal renderer.

The intelligence belongs in the playback path.

---

## North-star listening test

After acclimation and at matched loudness, bypass should make ordinary headphone playback feel **dimensionally collapsed**.

But bypass must never restore something Omniphony damaged.

Non-negotiable preservation targets include:

- clarity;
- transient shape;
- bass timing and weight;
- timbre;
- vocal/instrument identity;
- microdynamics and macrodynamics;
- rhythmic precision;
- stereo relationships that matter musically;
- musical hierarchy;
- emotional focus;
- recording character.

A giant spatial bubble with smeared music is a failure.

A renderer that sounds spectacular but weakens the singer is a failure.

A presentation that needs the song to fight the DSP is a failure.

> **Never buy dimension by damaging the music.**

---

## Lightweight by design

Power does not require permanent computational bulk.

Omniphony should be built around the smallest mechanism that preserves the behavior currently required.

```text
CONTROL / HEARING PLANE
rich analysis
libaural research / models
profile construction
slow musical context
        ↓ compact bounded publication

REALTIME AUDIO PLANE
sample clock
small persistent scene state
bounded trajectories
binaural / field / room DSP
        ↓
headphones
```

Rules:

- the audio thread never waits for AI reasoning;
- heavy models are optional, asynchronous or slower-cadence unless measurement proves otherwise;
- stable objects should be cheap;
- unchanged HRTFs, profiles and graphs should be near no-op updates;
- precompute/cache where that preserves semantics;
- use per-sample work only for dimensions that actually need it;
- prefer a fixed audio-time control quantum plus interpolation over host-callback-shaped updates;
- publish complete validated state atomically and keep the last known-good state on failure;
- a small realtime consumer projection is preferable to copying libaural's entire research state into the renderer;
- every new dependency, thread, buffer and model must earn its place.

The target is a renderer that feels unusually powerful **because it spends computation on the perceptually important coordinates**, not because it keeps every possible algorithm running all the time.

---

## libaural relationship

[`libaural`](https://github.com/dissonance-git/libaural) is the parent artificial-hearing project.

Its mission is broader than spatial audio:

> **Give an AI a functional sense of hearing from ordinary digital audio.**

Omniphony is one consumer of that hearing.

```text
song
 ↓
libaural
"what am I hearing?"
 ↓
auditory + musical state
 ↓
Omniphony
"how should this recording be presented over headphones?"
```

Potentially useful hearing state includes:

- what sounds and musical layers are present;
- what belongs together;
- foreground / background relations;
- pitch, timbre, rhythm and musical role;
- audibility and masking;
- source continuity;
- transient ownership;
- compact versus broad material;
- direct versus diffuse/room-like energy;
- recurrence and section structure;
- musical expectation/change;
- spatial evidence;
- uncertainty.

These are hearing capabilities, not hard-coded commands to move something behind the listener.

A libaural observation is evidence for a presentation decision.

A successful Omniphony rendering trick does not automatically become a law of hearing.

---

## Portable architecture

Windows is where the system is being proven today.

The core should not know which operating system delivered its samples.

```text
PLATFORM INPUT
system / app / player / file / stream
        ↓
========================================================
PORTABLE CORE

sample timeline
→ libaural hearing input
→ music-aware presentation policy
→ persistent scene
→ binaural / field / room renderer
→ listener + headphone calibration

========================================================
        ↓
PLATFORM OUTPUT
native device / host / plugin
```

Platform adapters own things such as:

- system/app capture;
- virtual endpoints or drivers when necessary;
- device enumeration;
- sample-format negotiation;
- shared/exclusive modes;
- permissions and service lifecycle;
- OS-native low-latency APIs;
- installer/signing details;
- mobile audio-session/focus behavior.

The portable core owns things such as:

- auditory/scene contracts;
- music-aware presentation policy;
- sample-time trajectories;
- HRTF/binaural rendering;
- direct/broad/diffuse/room distinctions;
- headphone/listener calibration;
- deterministic fidelity tests.

See [`docs/PLATFORM_PORTABILITY.md`](docs/PLATFORM_PORTABILITY.md).

---

## Time is not a callback

One of the project's strongest engineering laws is:

> **Changing host callback size must not change the intended auditory world.**

```text
WASAPI / CoreAudio / PipeWire / AAudio / Core Audio / file block
                              ↓
                    one logical sample timeline
```

Gain, movement, HRTF changes, room changes and scene transitions should live in sample/time coordinates rather than inheriting arbitrary host-buffer boundaries.

`dsp_fixtures::binaural_block_size` now contains a mandatory 40/240/960-sample callback-invariance gate for metadata/mute gain.

A saved Actions run at `a1a4a76` proved that the path was **not yet invariant**: the 40-vs-240 and 40-vs-960 outputs differed by roughly -39.28 dBFS while the gate requires at most -90 dBFS. The first identified hidden dependency was initial HRIR installation fading from an all-zero kernel for a callback-sized duration. The current candidate repair installs the first kernel immediately and preserves smooth crossfades only for later kernel changes.

That repair is not considered complete until the mandatory callback-invariance gate passes in CI.

**Position/HRTF movement is separately known to remain callback-quantized.** Its ignored reproducer stays isolated so the motion repair cannot hide behind gain initialization work.

The target is one canonical scene trajectory whose audible result is invariant to host partitioning.

---

## Scene vocabulary

These are **presentation entities**, not claims that stereo masters secretly contain authored object metadata.

### `FrontalAnchor`

Material whose movement would destabilize the song's center of gravity.

### `DirectObject`

Persistent compact/source-like material that can support spatially specific presentation.

### `BroadSource`

Coherent material with meaningful spatial extent.

### `DiffuseField`

Musical or ambient material better represented as a directional distribution than a point.

### `RoomField`

Presentation-environment energy such as early reflections and late reverberation.

Load-bearing distinction:

```text
DIRECT OBJECT
≠ BROAD SOURCE
≠ DIFFUSE MUSICAL FIELD
≠ ROOM FIELD
```

And:

```text
rear direct object
≠ rear reflection
≠ diffuse rear field
```

Stereo usually does not identify a literal hidden rear source. Rear placement can still be a valid immersive presentation choice when the hearing/music state supports it.

See [`docs/SCENE_RENDERER_CONTRACT.md`](docs/SCENE_RENDERER_CONTRACT.md).

---

## Current renderer foundation

The fork keeps mature upstream machinery that still serves the product instead of rewriting it for aesthetic cleanliness.

Useful retained substrate includes:

- stateful per-channel binaural DSP;
- analytic ITD;
- interpolated HRTF/HRIR rendering;
- embedded SAF KEMAR, parametric and SOFA-capable providers;
- continuous moving-filter crossfades;
- object position/size state;
- known-scene VBAP/layout machinery for calibration and regression truth;
- early image-source reflections;
- late FDN room field;
- deterministic DSP fixtures;
- headless engine/FFI surfaces.

Fork-specific work already includes:

- true complex M/S stereo evidence;
- persistence-aware stereo evidence;
- symmetric object/field evidence separation;
- bass protection separated from object identity;
- conservative scene-evidence inference;
- deterministic async HRTF source switching;
- stale HRIR rebuild rejection;
- measured-HRIR direct-arrival validation;
- per-ear early-reflection delays and directional ITD;
- sample-time-invariant FDN modulation;
- a sample-time metadata-gain path with a mandatory callback-invariance reproducer still under repair;
- true zero predelay;
- reusable null/RMS/crest/DC/level fidelity metrics.

---

## Current missing bridges

The architecture is ahead of the finished product.

### 1. Real stereo → persistent heard scene

Local stereo/scene evidence modules exist, but normal two-channel playback is not yet producing a complete persistent object/field world in the realtime path.

### 2. libaural → Omniphony

The local heuristics are scaffolding. The mature product should consume increasingly rich libaural hearing state instead of growing a second independent hearing stack.

### 3. Music-aware presentation policy

The renderer knows increasingly much about *how* to render a scene. The elite-engineer layer that decides *what this song should do* is still early.

A critical negative constraint is already known:

```text
musical importance / independence
≠ raw activity
≠ more notes
≠ wider pitch range
≠ more spectral change
≠ more energy
```

Early Helix exact-passage work found that obvious low-level activity proxies did not generally implement an authored distinction such as a bass line carrying its own melody. That means Omniphony must not spatially promote material simply because it is busy or numerically novel. Presentation increasingly needs role and relation evidence from libaural, with uncertainty, rather than a DSP-excitement score.

See [`docs/MUSIC_PRESENTATION_CONTRACT.md`](docs/MUSIC_PRESENTATION_CONTRACT.md).

### 4. Sample-time binaural trajectories

Metadata gain has a mandatory callback-size gate, but the latest saved Actions evidence still has that gate red and the current first-HRIR repair must earn green status.

Position/HRTF movement is a separate known defect. The parent renderer still publishes one block-start position rather than an actual canonical position trajectory segment.

The binaural stage should consume the same authoritative sample-time trajectory used by the scene state. It must not invent a second motion authority.

### 5. Source extent

The inherited scene state already carries object size. The binaural path still collapses too much of it to a point.

### 6. First-class `BroadSource`

A coherent wide source needs an actual headphone rendering strategy, not just a wider point-source parameter.

### 7. First-class `DiffuseField`

The FDN is a room field. It is not a substitute for diffuse musical content. A spherical/Ambisonic or experimentally equivalent field basis is a major candidate.

### 8. Listener/headphone calibration

Keep these layers separate until experiments justify combining them:

```text
listener HRTF
headphone response
driver ↔ ear interaction
BRIR / room target
low-frequency integration
safety headroom
```

See [`docs/HEADPHONE_CALIBRATION.md`](docs/HEADPHONE_CALIBRATION.md).

### 9. Platform shells

Windows capture/output is the first implementation target. It should become one thin host around the same portable core.

ASIO may remain an optional specialist route, not a mandatory architecture dependency.

---

## Validation lanes

Keep failures attributable by testing independent layers.

### A. Compiler + deterministic DSP

```text
compile
→ unit tests
→ deterministic fixtures
→ null/fidelity tests
→ callback-size invariance
```

### B. Known scene → headphones

```text
known geometry
→ HRTF / ITD
→ extent / fields
→ early room / late room
→ binaural output
```

This isolates renderer quality.

### C. Artificial hearing

libaural uses controlled fixtures and perturbations to test pitch, masking, grouping, temporal organization, space, music cognition and eventually an AI-facing heard-state interface.

### D. Music → immersive presentation

```text
song
→ hearing
→ presentation decision
→ scene
→ render
→ objective fidelity checks
→ human listening
```

This isolates whether the product is making musically intelligent choices rather than merely dramatic ones.

### E. Starting-sound preservation

Every major renderer change should retain a reference route that lets us compare against the useful upstream/fork baseline at matched loudness.

Test both intended gains and unintended losses:

```text
externalization / width / depth / height
AND
clarity / punch / tone / timing / bass / dynamics / focus
```

If the new path improves one column by degrading the other, it has not graduated.

### F. Competitive reference listening

Dolby, DTS, Waves, HeSuVi-style virtualization/HRIR chains and other strong spatial-audio systems are useful external references.

The project goal is to become a stronger general headphone presentation system than those prior approaches where the comparison is fair.

That is an **aspiration and benchmark program**, not a current superiority claim.

Compare dimensions separately:

- front/back/elevation localization;
- externalization;
- depth and source extent;
- direct-source solidity;
- diffuse-field quality;
- room naturalness;
- bass and groove integrity;
- transient and timbral preservation;
- fatigue;
- latency/glitch behavior;
- performance on ordinary stereo music rather than only authored immersive content.

---

## Current CI

The surviving workflow is:

```text
.github/workflows/windows-renderer.yml
```

It checks:

```text
portable renderer core
Windows renderer core
Windows x64 headless renderer-engine artifact
```

The headless artifact intentionally excludes the transitional host-audio/ASIO layer so the renderer itself can be compiled and tested without the separately licensed Steinberg SDK.

The latest saved run supplied for this checkpoint (`31354955851`, commit `a1a4a76`) failed both Linux/portable and Windows core jobs on the same binaural callback-invariance test. The Windows artifact job was therefore skipped by dependency.

That failure is treated as useful signal. CI must not be made green by weakening the -90 dBFS invariance requirement.

Windows remains the first live-listening platform after the portable engine is stable.

---

## Fork relationship

`mgth/Omniphony` remains the ancestor, attribution source and an ongoing source of mechanisms/fixes.

It is no longer the canonical product tree.

```text
upstream idea/fix
→ inspect
→ adopt only if useful here
→ validate locally
→ improve
→ optionally return a portable/general fix upstream
```

This fork does not preserve broad suite structure merely to make upstream merging easy.

It also does not delete useful upstream audio behavior simply because the product scope is narrower.

See [`docs/FORK_POLICY.md`](docs/FORK_POLICY.md) and [`NOTICE.md`](NOTICE.md).

---

## Contraction

Already removed from this fork include:

- Omniphony Studio;
- Arch/Linux packaging surfaces unrelated to the portable core;
- JACK helper/service shell;
- mpv-oriented product documentation;
- old Studio/WebGL/Three.js archaeology;
- obsolete upstream refactor diaries;
- inherited Studio/release workflows;
- demonstration backend crate;
- Lua/script backend crate;
- an orphaned manual PI adaptive-resampling tuning procedure whose durable clock-domain laws already live in the realtime control contract.

Known-scene fixtures, layouts and renderer machinery remain when they are useful laboratory instruments even if they are not part of final listener UX.

Further deletion is dependency-first, not aesthetic.

See [`docs/CONTRACTION_LEDGER.md`](docs/CONTRACTION_LEDGER.md).

---

## External influences

External repositories are mechanism sources, benchmarks and possible dependencies, not architecture votes.

The broad exploratory influence phase is now intentionally frozen unless a concrete experiment exposes a missing capability.

Current families include:

- spatial rendering: Steam Audio, SPARTA/IEM, OpenAL Soft, Dolby tooling, room/BRIR work;
- listener/headphone calibration: ASH, AutoEq, HeSuVi/HRIR corpora;
- realtime DSP: fft-convolver, KFR, Faust, Glicol and related references;
- artificial hearing/music cognition: psychoacoustic/auditory models, classical MIR, learned audio models, symbolic music systems and controlled music generators;
- mature realtime systems: s&box audio snapshots, Ardour/LMMS/Zrythm host/automation patterns, and openDAW's small headless audio-engine discipline;
- AI/research systems: Jukebox's hierarchical temporal representation as a conceptual influence and GABRIEL-style auditable qualitative evaluation outside realtime;
- platform transport: OS-native audio references such as Microsoft SysVAD for Windows-specific experiments.

The durable human-hearing program and influence ledger live in libaural.

From this checkpoint, new influences should be researched **because an experiment exposes a missing capability**, not because the list can always become longer.

---

## Near-term order

```text
0. make the binaural gain callback-invariance gate genuinely pass in CI
1. fix sample-time binaural position/HRTF movement
2. preserve source extent in headphones
3. implement/test BroadSource
4. implement/test DiffuseField
5. wire ordinary stereo into persistent realtime scene state
6. integrate the bounded libaural→Omniphony heard-state projection
7. build and falsify the first music-aware presentation policy
8. establish clean native Windows capture/output shell
9. matched-loudness listening + fidelity + competitive-reference optimization
10. port thin host shells to other operating systems once the core earns it
```

At every step:

```text
simplest mechanism first
→ measure
→ listen
→ keep only what earns its cost
```

The final product metaphor remains simple:

> **A world-class immersive mix engineer for every song, every time it plays.**

And the final engineering constraint is even simpler:

> **The better Omniphony gets, the less the listener should notice Omniphony itself.**

---

## License and origin

GPL-3.0-or-later, inherited from the original Omniphony project.

See [`LICENSE`](LICENSE), [`NOTICE.md`](NOTICE.md), and [`docs/FORK_POLICY.md`](docs/FORK_POLICY.md).
