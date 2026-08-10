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

The metadata/mute gain handoff now consumes a continuous sample-time segment in the binaural stage, and `dsp_fixtures::binaural_block_size` contains a mandatory 40/240/960-sample callback-invariance gate for that trajectory.

**Position/HRTF movement remains the active defect.** The parent renderer still publishes one position at the beginning of a caller block and advances the canonical ramp afterward. A separate ignored reproducer isolates that remaining staircase so the motion repair cannot hide behind the already-fixed gain path.

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

Fork-specific corrections already include:

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
- sample-time binaural metadata-gain consumption;
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

Early Helix exact-passage work found that obvious low-level activity proxies did not generally implement an authored distinction such as a bass line carrying its own melody. That means Omniphony must not spatially promote material simply because it is busy or numerically novel. Presentation eventually needs role and relation evidence from libaural, with uncertainty, rather than a DSP-excitement score.

### 4. Sample-time binaural movement

Metadata gain now has a callback-size invariance gate. Position/HRTF movement does not yet.

The parent renderer must publish an actual canonical position trajectory segment rather than one block-start position. The binaural stage should consume that trajectory on the audio timeline without inventing a second motion authority.

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

---

## Current CI

The surviving workflow is:

```text
.github/workflows/windows-renderer.yml
```

It currently checks:

```text
portable renderer core
Windows renderer core
Windows x64 headless renderer-engine artifact
```

The headless artifact intentionally excludes the transitional host-audio/ASIO layer so the renderer itself can be compiled and tested without the separately licensed Steinberg SDK.

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
- Lua/script backend crate.

Known-scene fixtures, layouts and renderer machinery remain when they are useful laboratory instruments even if they are not part of final listener UX.

Further deletion is dependency-first, not aesthetic.

See [`docs/CONTRACTION_LEDGER.md`](docs/CONTRACTION_LEDGER.md).

---

## External influences

External repositories are mechanism sources, benchmarks and possible dependencies, not architecture votes.

The broad exploratory influence phase is now intentionally frozen.

Current families include:

- spatial rendering: Steam Audio, SPARTA/IEM, OpenAL Soft, Dolby tooling, room/BRIR work;
- listener/headphone calibration: ASH, AutoEq, HeSuVi/HRIR corpora;
- realtime DSP: fft-convolver, KFR, Faust, Glicol and related references;
- artificial hearing/music cognition: classical MIR, auditory models, learned audio models, symbolic music systems and controlled music generators;
- platform transport: OS-native audio references such as Microsoft SysVAD for Windows-specific experiments.

The durable influence ledger lives in libaural.

From this checkpoint, new influences should be researched **because an experiment exposes a missing capability**, not because the list can always become longer.

---

## Near-term order

```text
completed  CI/compiler lane is meaningful and green on the established baseline
completed  binaural metadata gain has a callback-size invariance gate

1. fix sample-time binaural motion
2. preserve source extent in headphones
3. implement/test BroadSource
4. implement/test DiffuseField
5. wire ordinary stereo into persistent realtime scene state
6. integrate early libaural heard state
7. build first music-aware presentation policy
8. establish clean native Windows capture/output shell
9. listening + fidelity optimization
10. port thin host shells to other operating systems once the core earns it
```

The final product metaphor remains simple:

> **A world-class immersive mix engineer for every song, every time it plays.**

And the final engineering constraint is even simpler:

> **The better Omniphony gets, the less the listener should notice Omniphony itself.**

---

## License and origin

GPL-3.0-or-later, inherited from the original Omniphony project.

See [`LICENSE`](LICENSE), [`NOTICE.md`](NOTICE.md), and [`docs/FORK_POLICY.md`](docs/FORK_POLICY.md).