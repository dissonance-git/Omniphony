# Omniphony

**Windows-first real-time music enhancement: stereo library → libaural-informed remix decisions → full-sphere binaural headphones.**

Omniphony is an independent research/product fork of [`mgth/Omniphony`](https://github.com/mgth/Omniphony), rebuilt around one deliberately narrow product idea:

> **Make ordinary music playback feel as though an elite mix / mastering / immersive-audio engineer were personally adapting each song for the listener in real time, with very low latency, into a coherent 360° headphone world while preserving the recording's musical identity and intent.**

The target is not a generic upmixer.

It is not:

```text
stereo
→ wider stereo
→ some rear reverb
→ headphones
```

The intended long-term behavior is closer to:

```text
ordinary stereo song
        ↓
libaural hears what the music is doing
        ↓
Omniphony interprets how that specific recording can safely expand
        ↓
anchors / direct objects / broad sources / diffuse musical fields / room
        ↓
artistically conservative real-time immersive remix decisions
        ↓
full-sphere binaural renderer
        ↓
listener HRTF + room cues + headphone calibration
        ↓
headphones
```

The analogy is a world-class engineer, not an automatic effect preset.

A great engineer does not make every element maximally wide or dramatic. They understand what the song is doing, protect what carries the music, create separation where it helps, preserve punch and intimacy when they matter, and use space deliberately.

Omniphony aims to automate that kind of *presentation intelligence* continuously during normal playback.

It cannot literally recover an artist's private intention or missing multitrack session. The practical target is **artistically faithful inference from the audible recording**: make the enhanced presentation feel purpose-built for the song rather than imposed on it.

The product should eventually feel almost boring operationally:

```text
install once
choose / calibrate headphones once
play music normally
```

No exporting stems. No manually authoring scenes. No per-song setup ritual.

The intelligence belongs in the playback path.

---

## North star

One song should make the ambition obvious.

After acclimation and at matched loudness, bypass should make ordinary headphone playback feel **dimensionally collapsed**, as though the same recording suddenly lost a spatial layer of information and presentation.

But bypass must **not** restore something Omniphony damaged.

Non-negotiable preservation targets:

- clarity;
- transient shape;
- bass timing and weight;
- timbre;
- vocal/instrument identity;
- stereo relationships that are musically important;
- microdynamics and macrodynamics;
- rhythmic precision;
- musical hierarchy;
- emotional focus;
- recording character.

A giant 360° sphere with smeared music is a failure.

A technically impressive rendering that makes the singer less convincing is a failure.

A spatial effect that works on one genre and fights another is a failure.

The ideal result should feel less like "DSP was added" and more like **the song had always been mixed for this listening space**.

---

## What libaural contributes

[`libaural`](https://github.com/dissonance-git/libaural) owns the general artificial-hearing problem.

Its job is not specifically to spatialize music. Its job is to let an AI hear digital audio in a functionally human-like way.

```text
song
  ↓
libaural
"what am I hearing?"
  ↓
perceptual + musical state
  ↓
Omniphony
"how can this particular music be enhanced spatially without betraying it?"
```

That means Omniphony can eventually make decisions from richer information than raw panning and spectral energy alone.

Potentially useful libaural hearing state includes:

- what sounds and musical layers are present;
- what belongs together;
- what is foreground or background;
- pitch, timbre, rhythm and musical role;
- masking and audibility;
- source continuity;
- transient ownership;
- broad versus compact material;
- direct versus room-like energy;
- recurrence and section structure;
- expectation and musical change;
- spatial evidence;
- confidence and ambiguity.

These are hearing capabilities, not Omniphony's product ontology.

Omniphony owns the presentation decision.

A rendering trick does not become a law of hearing merely because it sounds good here.

A libaural observation does not automatically mean "move this behind the listener." It becomes one input to a music-aware mix decision.

---

## The real-time engineer model

A useful mental model for Omniphony is:

```text
LISTEN
what is this song doing right now?
        ↓
PROTECT
what must not be destabilized?
        ↓
SEPARATE
what can gain clarity from spatial distinction?
        ↓
EXPAND
what can occupy depth, height, rear space or broader extent?
        ↓
BIND
what should remain perceptually connected?
        ↓
RENDER
how should that decision reach this listener's two ears?
        ↓
CHECK
was punch / tone / hierarchy / timing / loudness damaged?
```

The important word is **specific**.

The same rule should not be applied equally to every song, every section, every source, or every moment.

A dry intimate vocal may need authority and proximity.

A wide shoegaze texture may benefit from becoming a large surrounding field.

A hard-panned guitar may support a stable direct object.

A bass line may need to remain a physical floor even while higher harmonics participate in a larger scene.

A tiny production detail may deserve rear or height placement if doing so reveals it without changing the song's center of gravity.

A dense climax may need *less* spatial aggression than a sparse verse because preserving hierarchy matters more than filling every direction.

That is why libaural matters to the product: better hearing should permit better mixing decisions.

---

## Scene semantics

Omniphony currently keeps several useful spatial presentation entities distinct.

These are renderer concepts, not claims that every recording literally contains authored objects with these labels.

### `FrontalAnchor`

Musically authoritative material whose relocation would destabilize the presentation, such as a coherent center or source-like low-frequency foundation.

### `DirectObject`

Persistent source-like material that can support a spatially specific presentation.

### `BroadSource`

Coherent material with meaningful extent, or content for which a single point is too specific.

### `DiffuseField`

Musical/ambient material better presented as a directional distribution than as a point source.

### `RoomField`

Presentation-environment energy: early reflections and late reverberant field.

The distinction is load-bearing:

```text
DIRECT OBJECT
≠ BROAD SOURCE
≠ DIFFUSE MUSICAL FIELD
≠ ROOM FIELD
```

Rear objects and rear reverberation are also different things.

Stereo usually does **not** contain literal rear-position ground truth, so Omniphony separates:

```text
acoustic / auditory evidence
from
music interpretation
from
presentation choice
```

A convincing rear direct element can be a valid immersive mix decision without pretending that the original master contained hidden rear metadata.

---

## Current renderer foundation

The fork retains the strongest parts of the original Omniphony renderer instead of rewriting them for aesthetic cleanliness.

Current useful substrate includes:

- stateful per-channel binaural DSP;
- analytic ITD;
- interpolated HRTF/HRIR rendering;
- SAF KEMAR, parametric and SOFA-capable HRTF providers;
- safe moving-filter crossfades;
- object position/size state;
- VBAP and known speaker layouts for calibration truth;
- early image-source reflections;
- late FDN room field;
- deterministic DSP fixtures;
- file/reference rendering paths;
- Windows audio/output infrastructure.

Recent fork-specific corrections include:

- true complex M/S stereo evidence;
- time-constant-based persistence;
- symmetric object/field evidence separation;
- bass protection separated from object identity;
- explicit scene candidate evidence;
- deterministic async HRTF source switching;
- measured-HRIR direct-arrival validation rather than invalid zero-cross-correlation assumptions;
- source-tagged async HRIR rebuilds so stale builds cannot win late;
- per-ear ITD for early image-source reflections;
- sample-time-invariant FDN modulation;
- true zero-predelay behavior;
- reusable fidelity metrics for null/RMS/crest/DC/level comparisons.

See [`docs/SCENE_RENDERER_CONTRACT.md`](docs/SCENE_RENDERER_CONTRACT.md).

---

## Current missing bridges

The architecture is clearer than the end-to-end product. Important gaps remain explicit.

### libaural is not yet the live hearing source

The current fork has local stereo evidence and scene-inference code, but the mature product should increasingly consume richer libaural hearing state instead of growing an independent duplicate machine-hearing stack inside Omniphony.

Current local heuristics are practical scaffolding and test instruments.

### Stereo inference is not yet the live scene source

`renderer::stereo_inference` and `renderer::scene_inference` are real code, but ordinary two-channel playback is not yet driving a complete persistent object/field scene in realtime.

### Music-aware presentation policy is still primitive

The renderer can increasingly describe spatial possibilities, but the "elite engineer" layer is still largely missing.

It eventually needs to combine:

```text
libaural hearing
+
musical hierarchy
+
spatial opportunity
+
fidelity risk
+
listener calibration
+
short and long temporal context
```

into conservative presentation decisions.

### Binaural motion/gain still needs sample-accurate trajectory plumbing

The inherited speaker path carries sample-accurate ramps more completely than the current binaural handoff. Position and gain continuity should be made host-block-size invariant without creating a second competing state machine.

### Broad-source extent is not yet fully preserved in headphone rendering

The inherited scene state contains object size/extent, but binaural rendering currently collapses too much of that information to a point.

### `DiffuseField` needs a first-class spherical renderer

The late FDN is a **room field**, not a substitute for diffuse musical content. A spherical/Ambisonic or experimentally equivalent field basis is a strong candidate.

### Listener/headphone calibration is still early

SOFA/HRTF support exists, but the mature product should distinguish:

```text
listener HRTF
headphone response
headphone-driver ↔ pinna interaction
room / BRIR target
low-frequency integration
safety headroom
```

See [`docs/HEADPHONE_CALIBRATION.md`](docs/HEADPHONE_CALIBRATION.md), [`docs/FORK_POLICY.md`](docs/FORK_POLICY.md), and the libaural influence ledger.

### The Windows product audio shell is still transitional

The inherited Windows host currently hard-wires CPAL's ASIO feature, which makes the full executable depend on separately licensed Steinberg SDK material.

That is not the intended default architecture for a normal install-once Windows music product.

Current direction:

```text
renderer engine
→ proven independently in CI

Windows system/player capture + normal output route
→ explicit product layer still to be simplified

ASIO
→ optional low-latency/audiophile route, not mandatory build infrastructure
```

---

## Repository scope

This fork is physically removing inherited suite surfaces rather than maintaining them for structural parity.

### Keep

- `omniphony-renderer/` — realtime renderer and transitional Windows host path;
- `layouts/` — known-scene calibration geometry;
- deterministic assets/fixtures needed for regression tests;
- `docs/` — current fork contracts, validation reports and research decisions;
- `.github/workflows/` — reproducible renderer-engine validation.

### Removed from this fork

The original upstream remains the archive/source for these surfaces:

- Omniphony Studio;
- Arch packaging;
- mpv-oriented product documentation;
- JACK service helper scripts;
- old Studio/WebGL/Three.js debugging notes;
- Linux/PipeWire-specific product plans and investigation diaries;
- obsolete upstream refactor plans that no longer describe this product;
- upstream Studio/cross-platform release workflows;
- demonstration backend crate;
- Lua/scriptable backend crate.

More contraction happens dependency-first. See [`docs/CONTRACTION_LEDGER.md`](docs/CONTRACTION_LEDGER.md).

---

## Upstream relationship

`mgth/Omniphony` is a **source / peer / ancestor**, not the canonical product tree.

```text
upstream mechanism or fix
→ inspect
→ take only what serves this fork
→ validate locally
→ improve further
→ optionally send a general fix back upstream
```

We do not keep broad structural parity for its own sake.

General fixes should be offered upstream only after they are proven here and separated from fork-specific assumptions.

Full policy: [`docs/FORK_POLICY.md`](docs/FORK_POLICY.md).

Attribution remains permanent. The fork keeps the original Git history and GPL licensing. See [`NOTICE.md`](NOTICE.md).

---

## External influence policy

GitHub projects are treated as mechanism sources, not templates to absorb wholesale.

Current major influence families include:

- Steam Audio / SPARTA / IEM / OpenAL Soft for spatial rendering laws;
- Dolby open tooling for signal/object/presentation boundaries and objective audio validation;
- RoomAcoustiCpp and measured BRIR work for room/externalization structure;
- ASH Toolset for listener + headphone + BRIR calibration layering;
- HeSuVi and HRIR conversion projects as large behavioral/reference corpora;
- `fft-convolver` for allocation-free partitioned realtime convolution;
- Airwave for set-and-forget, per-device system-wide product behavior;
- MidiTok/Symusic and game/chip synthesis tooling for controlled musical ground truth;
- Microsoft Windows Audio repositories for diagnostics and low-latency platform contracts.

The durable influence ledger lives in libaural so chat compaction is not project memory.

---

## Validation strategy

There are three acceptance lanes.

### Hearing

Can libaural expose perceptual and musical state that tracks useful human listening behavior?

This is primarily a libaural research problem.

### Known scene → headphones

Hold source geometry constant and test:

```text
known objects / beds
→ HRTF + ITD
→ broad/field rendering
→ early room + late field
→ headphones
```

This isolates renderer quality.

### Music → immersive presentation

Hold renderer behavior understood and test:

```text
song
→ hearing
→ mix / presentation policy
→ immersive scene
```

This isolates whether Omniphony is making musically intelligent choices rather than merely dramatic ones.

Only after these work independently should end-to-end listening decide product quality.

Objective fidelity measurements include:

- bypass/null residual;
- peak and RMS level;
- crest factor;
- DC offset;
- frequency response;
- lag/ITD;
- dynamic/transient preservation;
- clipping/headroom.

Human listening remains required for:

- externalization;
- front/back discrimination;
- elevation plausibility;
- source stability;
- envelopment;
- image depth;
- fatigue;
- musical hierarchy;
- emotional focus;
- whether the enhancement feels native to the song rather than imposed;
- preference.

---

## Build / CI

The authoritative repository workflow is:

```text
.github/workflows/windows-renderer.yml
```

It separates:

```text
portable renderer core
Windows renderer core
Windows x64 renderer-engine artifact
```

The renderer-engine artifact intentionally excludes the host audio layer, so it can prove/package the actual engine without depending on the separately licensed ASIO SDK.

The old inherited CI/release workflows were removed because they built deleted Studio, Linux/PipeWire, macOS and cross-platform release products.

The workflow also resolves dependencies normally because this repository does not track `omniphony-renderer/Cargo.lock`.

A full listening executable returns to CI when the fork's Windows audio shell has a clean normal-system default route. ASIO may remain an optional specialist backend.

---

## Near-term development order

```text
1. keep CI/compiler signal trustworthy
2. finish sample-accurate binaural trajectory/gain plumbing
3. preserve source extent into binaural rendering
4. implement first-class broad/diffuse musical-field rendering
5. wire ordinary stereo into persistent realtime scene state
6. define the first music-aware presentation-policy layer
7. progressively replace local hearing heuristics with libaural capabilities
8. build listener/headphone calibration stack
9. establish normal Windows system/player capture + output route
10. make ASIO optional rather than mandatory
11. continue deleting inherited surfaces with no remaining owner
12. listening + fidelity optimization across real music
```

The product goal remains intentionally unreasonable:

> **Make ordinary headphone playback feel like the lower-dimensional version after Omniphony is bypassed.**

The product metaphor is:

> **A world-class immersive mix engineer for every song, every time it plays.**

And the engineering law that keeps that metaphor honest is:

> **Never buy dimension by damaging the music.**

---

## License and origin

GPL-3.0-or-later, inherited from the original Omniphony project.

See [`LICENSE`](LICENSE), [`NOTICE.md`](NOTICE.md), and [`docs/FORK_POLICY.md`](docs/FORK_POLICY.md).
