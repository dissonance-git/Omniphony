# omniphony-renderer

`omniphony-renderer` is the realtime engine inside the independent Omniphony fork.

Its product role is now deliberately narrow:

```text
Windows stereo PCM
→ realtime evidence / persistent auditory scene
→ direct objects + broad sources + fields
→ binaural rendering
→ calibrated stereo headphone output
```

The workspace still contains inherited speaker, bridge and cross-platform machinery where it remains useful for deterministic calibration, comparison, or a load-bearing dependency. That temporary presence does **not** define product scope.

For fork policy and the product overview, start at [`../README.md`](../README.md) and [`../docs/FORK_POLICY.md`](../docs/FORK_POLICY.md).

---

## Runtime priorities

The engine is being optimized around:

- ordinary stereo music as the normal source;
- Windows 10/11 x64 first;
- stable realtime processing;
- sample-accurate state independent of callback block size;
- persistent scene evidence rather than transient-chasing upmix rules;
- direct-object, broad-source and diffuse-field presentation;
- binaural HRTF/ITD rendering;
- early reflections and late room field;
- listener/headphone calibration;
- strong matched-loudness bypass fidelity;
- deterministic file/known-scene validation.

The final user should not need to understand speaker layouts, bridge ABIs, OSC registries, or renderer backends to listen to music.

---

## Important crates

### `renderer`

The core DSP and scene renderer.

Current fork-specific work lives here, including:

- `stereo_inference`;
- `scene_inference`;
- binaural HRTF/ITD;
- early reflections;
- late FDN field;
- speaker/VBAP machinery retained as known-scene truth and shared renderer substrate.

### `dsp_fixtures`

Deterministic measurement and regression infrastructure.

It should remain independent enough that a broken ruler cannot silently certify broken DSP.

### `audio_output`

Realtime output/timing infrastructure. Windows is the product target.

The inherited Windows implementation currently hard-wires CPAL's ASIO feature. That is transitional: a clean normal-system Windows route should become the default, with ASIO optional.

### `audio_input`

Inherited input/control infrastructure. Much of the current Linux/PipeWire and fixed-channel behavior is transitional and will be reduced as the Windows stereo capture path becomes explicit.

### `host_audio`

Host audio integration shared by the current engine surfaces.

### `orender_engine`

Headless renderer construction and engine glue. This still carries inherited generic backend/config compatibility that is being contracted.

### `orender_ffi`

Embedding boundary. Retained while it provides a useful engine/test integration surface; it is no longer a separately released cross-platform product.

It deliberately excludes the host-audio layer, which makes it useful as the clean Windows CI engine boundary while the Windows system-audio shell is being simplified.

### `reference_bridge`

**Keep as a laboratory instrument.**

It provides deterministic known-channel / known-scene input for renderer tests. It is not the intended normal stereo-music ingestion architecture.

### `bridge_api`, `spdif`, `runtime_control`, `sys`

Inherited support crates with mixed status. Some remain load-bearing today; some contain transport/UI-era semantics that should disappear as dependencies are simplified.

### `example_backend`, `script_backend`

Inherited extensibility demonstrations. They are not part of the target product and are scheduled for removal once their engine registration edges are removed atomically.

---

## Build and test

The workspace currently requires Rust `1.87.0`.

From this directory:

```sh
cargo fmt --all -- --check
cargo test -p dsp_fixtures
cargo test -p renderer
```

The authoritative repository workflow is:

```text
../.github/workflows/windows-renderer.yml
```

It separates:

```text
portable renderer core
Windows renderer core
Windows x64 renderer-engine artifact
```

The Windows artifact packages the cpal-free headless engine/FFI and reference bridge. That gives the renderer a real compiler/test/package gate without requiring the separately licensed Steinberg ASIO SDK.

There is intentionally no longer a Studio, Linux packaging, macOS product, or cross-platform library-release workflow in this fork.

### Windows listening shell

The inherited full executable still supports an ASIO-oriented route where the required SDK/toolchain is available. That is useful for low-latency specialist listening, but it is not assumed to be the final system-wide Windows capture/output architecture.

The next host-audio refactor should make a normal Windows system backend the clean default and make ASIO opt-in.

See [`BUILDING_WINDOWS.md`](BUILDING_WINDOWS.md) only as inherited setup reference while Windows integration is being simplified.

---

## Binaural path

The headphone path is independent from the speaker/VBAP output presentation.

At a high level:

```text
source position / scene state
→ listener-relative direction
→ interpolated HRTF
→ analytic per-ear ITD
→ stateful convolution
→ directional early room
→ late room field
→ stereo
```

Important current properties:

- measured and parametric HRTF providers;
- optional SOFA source support;
- direction interpolation;
- old/new filter output crossfade for movement;
- asynchronous HRTF rebuild away from the audio thread;
- request-tagged rebuild completion so stale HRTFs cannot win late;
- analytic ITD separated from measured-HRTF direct-arrival timing;
- per-ear ITD on early image-source reflections;
- frequency-dependent interaural coherence in the late field;
- FDN modulation driven by processed sample count rather than callback count;
- true zero-predelay behavior.

See [`BINAURAL.md`](BINAURAL.md) for implementation details, but treat any remaining Studio/cross-platform instructions there as inherited documentation until that file is contracted too.

---

## Stereo scene inference

`renderer::stereo_inference` currently exposes inspectable low-level evidence such as:

- L/R pan/asymmetry;
- phase coherence;
- directness / diffuseness;
- true complex mid and side magnitude;
- time-constant persistence;
- trajectory agreement and stability.

`renderer::scene_inference` adds conservative scene evidence:

- frontal/foundation anchor support;
- lateral object-candidate support;
- broad-source evidence;
- diffuse-field evidence;
- spatial specificity;
- reassignment safety.

These modules are **not yet a complete realtime stereo→scene pipeline**. They are the tested beginning of it.

Rear placement remains a presentation choice constrained by evidence, not fake recovered metadata.

---

## Known scene versus inferred scene

Keep two validation lanes independent.

### Known scene → binaural

Use the reference bridge, fixed layouts and authored fixtures to answer:

> If the renderer receives the correct source geometry, does it produce a convincing and faithful headphone scene?

### Stereo → scene hypothesis

Hold rendering constant and answer:

> Does the stereo analysis discover stable, musically useful organization without damaging or hallucinating hierarchy?

Do not debug both problems at once.

---

## Fidelity contract

Every perceptual improvement should be accompanied by evidence about what it cost.

Current reusable measurements include:

- strict peak residual/null level;
- RMS residual;
- peak and RMS level;
- crest factor;
- DC offset;
- matched RMS-level delta;
- FFT/frequency-response analysis;
- interaural lag/ITD analysis.

Additional important gates include:

- transient preservation;
- clipping/headroom;
- block/chunk-size invariance;
- movement continuity;
- bass timing and weight;
- profile-switch continuity.

Human listening remains necessary for externalization, front/back, elevation, depth, envelopment, stability, fatigue and musical hierarchy.

---

## Listener/headphone calibration

Calibration is now an explicit future layer rather than an HRTF afterthought.

Keep distinct:

```text
listener HRTF
headphone response
driver ↔ ear interaction
room/presentation target
low-frequency integration
safety headroom
```

See [`../docs/HEADPHONE_CALIBRATION.md`](../docs/HEADPHONE_CALIBRATION.md).

---

## Current contraction boundary

Safe removals already made at repository level include:

- Omniphony Studio;
- Linux/Arch packaging;
- mpv product documentation;
- old Studio/WebGL investigation material;
- obsolete suite workflows and release jobs.

The next crate-level removals must follow dependency edges rather than aesthetics.

Likely order:

```text
example_backend + script_backend
→ generic contributor-backend glue no longer required
→ PipeWire/IEC61937/SPDIF product path
→ bridge-first normal ingestion assumptions
→ cross-platform host compatibility no longer used
```

See [`../docs/CONTRACTION_LEDGER.md`](../docs/CONTRACTION_LEDGER.md).

Do not remove the reference bridge or known-scene geometry simply because they are not final product UX. They remain valuable controlled truth.

---

## North-star rule

The purpose of the engine is not to maximize a spatial-effect score.

It is to make ordinary headphone playback feel lower-dimensional when bypassed **without bypass restoring music that Omniphony damaged**.

That means:

```text
more dimension
+
more externalization
+
more stable auditory world

must coexist with

clarity
transients
bass precision
timbre
dynamics
hierarchy
```

If those trade against each other, the renderer has more work to do.
