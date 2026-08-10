# Omniphony

**Windows-first stereo music → persistent auditory scene → binaural headphones.**

Omniphony is an independent research/product fork of [`mgth/Omniphony`](https://github.com/mgth/Omniphony), rebuilt around one deliberately narrow goal:

> Take ordinary stereo music and present it over headphones as a stable, externalized, convincing full-sphere auditory scene without sacrificing musical identity, clarity, timbre, bass relationships, transients, dynamics, or mix hierarchy.

The intended experience is not “stereo + widening + reverb.”

```text
ordinary stereo music
        ↓
realtime acoustic evidence
        ↓
libaural-informed persistent scene hypotheses
        ↓
anchors / direct objects / broad sources / diffuse fields
        ↓
Omniphony binaural renderer
        ↓
listener HRTF + room cues + headphone calibration
        ↓
headphones
```

The long-term product should feel almost boring operationally:

```text
install once
choose / calibrate headphones once
play music normally
```

The interesting part should happen in the sound, not in the setup ritual.

---

## North star

After acclimation, matched-loudness bypass should make ordinary headphone playback feel **dimensionally collapsed**.

But bypass must **not** restore anything Omniphony damaged.

Non-negotiable preservation targets:

- clarity;
- transient shape;
- bass timing and weight;
- timbre;
- vocal/instrument identity;
- stereo relationships;
- microdynamics and macrodynamics;
- rhythmic precision;
- musical hierarchy.

A huge sphere with smeared music is a failure.

---

## Scene semantics

Omniphony keeps several spatial entities distinct.

### `FrontalAnchor`

Musically authoritative material whose relocation would destabilize the mix, such as a coherent center or persistent low-frequency foundation.

### `DirectObject`

Persistent source-like evidence strong enough to support a spatially specific presentation.

### `BroadSource`

A coherent source with meaningful extent, or material for which a single point is too specific.

### `DiffuseField`

Field-like musical/ambient energy better represented as a directional distribution than a point source.

### `RoomField`

Presentation-environment energy: early reflections and late reverberant field.

The distinction is load-bearing:

```text
DIRECT OBJECT
≠ BROAD SOURCE
≠ DIFFUSE FIELD
≠ ROOM FIELD
```

Rear objects and rear reverberation are also different things.

Stereo usually does **not** contain literal rear-position ground truth, so Omniphony separates:

```text
acoustic evidence
from
scene hypothesis
from
presentation choice
```

A convincing rear object can be a valid presentation decision without pretending that the original recording contained recoverable rear metadata.

---

## libaural relationship

[`libaural`](https://github.com/dissonance-git/libaural) owns the general machine-hearing problem.

```text
libaural
"what auditory organization is supported by this sound?"
        ↓
objects / fields / relations / history / confidence
        ↓
Omniphony
"how should that scene reach two ears?"
```

libaural research includes:

- cochlear/peripheral representations;
- multiresolution spectrotemporal analysis;
- grouping and temporal coherence;
- pitch and timbre constancy;
- onset binding;
- masking and audibility;
- auditory memory and prediction;
- persistent object identity;
- competing scene hypotheses and uncertainty;
- controlled symbolic/synthesis truth for experiments.

Omniphony owns:

- Windows audio integration;
- realtime stereo evidence;
- practical scene presentation policy;
- binaural HRTF/ITD rendering;
- direct/broad/diffuse rendering;
- early reflections and late room fields;
- listener/headphone calibration;
- latency and realtime safety;
- listening validation and product defaults.

A rendering trick does not become a law of hearing merely because it sounds good here.

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

### Stereo inference is not yet the live scene source

`renderer::stereo_inference` and `renderer::scene_inference` are real code, but ordinary two-channel playback is not yet driving a complete persistent object/field scene in realtime.

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
- upstream Studio/cross-platform release workflows.

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

There are two independent acceptance lanes.

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

### Stereo → inferred scene

Hold binaural rendering fixed and test:

```text
stereo master
→ acoustic evidence
→ grouping / persistence / musical role
→ scene hypothesis
```

This isolates machine-hearing/scene inference quality.

Only after both work independently should end-to-end listening decide product quality.

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
- preference.

---

## Build / CI

The authoritative repository workflow is:

```text
.github/workflows/windows-renderer.yml
```

It now separates:

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
4. implement first-class broad/diffuse field rendering
5. wire stereo evidence into persistent realtime scene state
6. build listener/headphone calibration stack
7. establish normal Windows system/player capture + output route
8. make ASIO optional rather than mandatory
9. continue deleting inherited surfaces with no remaining owner
10. listening + fidelity optimization
```

The product goal remains intentionally unreasonable:

> Make ordinary headphone playback feel like the lower-dimensional version after Omniphony is bypassed.

The engineering rule that keeps that goal useful is simpler:

> Never buy dimension by damaging the music.

---

## License and origin

GPL-3.0-or-later, inherited from the original Omniphony project.

See [`LICENSE`](LICENSE), [`NOTICE.md`](NOTICE.md), and [`docs/FORK_POLICY.md`](docs/FORK_POLICY.md).
