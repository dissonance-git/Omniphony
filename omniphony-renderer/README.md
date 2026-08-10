# omniphony-renderer

`omniphony-renderer` is the realtime engine inside the independent Omniphony fork.

Its role is portable even though Windows is the current development/listening host:

```text
ordinary PCM / trusted scene input
→ realtime evidence / persistent auditory scene
→ direct objects + broad sources + fields
→ binaural rendering
→ listener/headphone calibration
→ stereo headphone output
```

The workspace still contains inherited speaker, bridge and host-specific machinery where it remains useful for deterministic calibration, comparison, or a load-bearing dependency. That temporary presence does **not** define the mature product boundary.

For the product overview, portability boundary and fork policy, start at:

- [`../README.md`](../README.md)
- [`../docs/PLATFORM_PORTABILITY.md`](../docs/PLATFORM_PORTABILITY.md)
- [`../docs/FORK_POLICY.md`](../docs/FORK_POLICY.md)

---

## Runtime priorities

The engine is being optimized around:

- ordinary stereo music as the normal consumer source;
- one portable audio/sample timeline independent of host callback size;
- stable realtime processing;
- persistent scene evidence rather than transient-chasing upmix rules;
- direct-object, broad-source and diffuse-field presentation;
- binaural HRTF/ITD rendering;
- early reflections and late room field;
- listener/headphone calibration;
- strong matched-loudness bypass fidelity;
- deterministic file/known-scene validation;
- thin OS-specific capture/output shells around the same core.

Windows 10/11 x64 is the **current live integration and critical-listening environment**. It is not the permanent architecture.

The final listener should not need to understand speaker layouts, bridge ABIs, OSC registries, renderer algorithms, virtual endpoints or host APIs to listen to music.

---

## Workspace ownership

### `renderer`

**Portable core / KEEP.**

Current work includes:

- `stereo_inference`;
- `scene_inference`;
- binaural HRTF/ITD;
- early reflections;
- late FDN room field;
- speaker/VBAP machinery retained as known-scene truth and shared comparison substrate.

### `dsp_fixtures`

**Portable test ruler / KEEP.**

Deterministic measurement and regression infrastructure.

It should remain independent enough that a broken ruler cannot silently certify broken DSP.

Current/future responsibilities include:

- null/residual metrics;
- ITD/ILD checks;
- known scenes;
- block-size invariance;
- movement/gain continuity;
- performance calibration.

### `orender_engine`

**Headless portable engine boundary / KEEP, narrow over time.**

Useful because it constructs and drives the renderer without owning OS audio I/O.

Inherited generic backend/config compatibility can contract while preserving the headless boundary itself.

### `orender_ffi`

**Embedding boundary / KEEP while useful.**

The C ABI is useful for headless validation and future native/platform shells. It is not treated as a separate product for its own sake.

### `reference_bridge`

**Known-scene laboratory instrument / KEEP.**

Provides deterministic known-channel / known-scene input. It is not normal consumer stereo ingestion.

### `bridge_api`

**Transitional lab/runtime boundary.**

Still load-bearing for the reference bridge and inherited rich-input paths. The useful known-scene contract should eventually be separated from obsolete generic decoder-plugin assumptions.

### `host_audio`

**Platform-shell boundary / KEEP conceptually, replace internals as needed.**

This is the correct architectural location for device/capture/output work because `renderer` and `orender_engine` should not own OS audio policy.

### `audio_output`

**Platform implementation / TRANSITIONAL.**

Current inherited hosts include Linux/PipeWire and CPAL routes. Windows currently hard-wires CPAL's ASIO feature, which is not the desired normal consumer default.

Target:

```text
portable output contract
→ Windows adapter
→ macOS adapter
→ Linux adapter
→ Android adapter
→ iOS adapter
```

ASIO may remain an optional specialist Windows route.

### `audio_input`

**Platform implementation / REPLACE THEN CUT.**

Despite its generic name, the current public model is inherited transport-specific machinery centered on:

```text
Bridge / PipeWire / PipeWireBridge
PipeWire / ASIO backend labels
fixed 7.1 mapping
IEC61937 bridge clocks/pacing
```

Do not expand this enum surface to every future operating system.

Replace it with a small neutral PCM/time/reset/source-metadata contract, migrate host callers, then delete the obsolete transport topology.

### `spdif`

**REPLACE THEN CUT.**

Owns inherited IEC61937/S/PDIF bitstream parsing used by the legacy Linux bridge path. Ordinary PCM music ingestion does not need it.

### `runtime_control`

**KEEP / TRANSITIONAL SURFACE.**

Timed state publication remains useful. Studio/OSC/backend-era assumptions can shrink.

### `sys`

**KEEP SMALL.**

Shared operational/platform helpers only. Do not let product/scene semantics accumulate here.

### `example_backend`, `script_backend`

**REMOVED.**

Their registrations, dependency edges, workspace membership and crate trees are already gone.

---

## Build and test

The workspace currently requires Rust **`1.88.0`**.

From this directory:

```sh
cargo fmt --all
cargo test -p dsp_fixtures
cargo test -p renderer
```

The authoritative repository workflow is:

```text
../.github/workflows/windows-renderer.yml
```

It currently separates:

```text
portable renderer core
Windows renderer core
Windows x64 headless renderer-engine artifact
```

The headless artifact packages engine/FFI/reference-test surfaces without requiring the transitional host-audio layer or separately licensed Steinberg ASIO SDK.

Windows is therefore the first native integration gate **around** the portable renderer, not the renderer's identity.

---

## Realtime time law

Audible state belongs to one logical sample timeline.

```text
WASAPI callback
CoreAudio callback
PipeWire callback
AAudio/Oboe callback
Core Audio callback
file-render chunk
plugin host block
        ↓
    same timeline
```

Changing caller block size must not change:

- gain trajectory;
- object motion;
- HRTF trajectory;
- room transitions;
- scene transitions;
- inferred auditory organization.

The speaker path already preserves more of this trajectory information than the current binaural handoff.

A deterministic known-defect reproducer now lives in:

```text
dsp_fixtures::binaural_block_size
```

It holds source position/HRTF/PCM constant and changes only callback partition while exercising the 20 ms metadata-gain slew.

After the hot-path fix it must become a mandatory positive equivalence gate.

---

## Binaural path

At a high level:

```text
source / scene state
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
- optional SOFA support;
- direction interpolation;
- old/new filter output crossfade for movement;
- asynchronous HRTF rebuild away from the audio thread;
- request-tagged rebuild completion so stale HRTFs cannot win late;
- analytic ITD separated from measured-HRTF direct-arrival timing;
- per-ear ITD on early image-source reflections;
- frequency-dependent interaural coherence in the late field;
- FDN modulation driven by processed sample count rather than callback count;
- true zero-predelay behavior.

See [`BINAURAL.md`](BINAURAL.md).

---

## Current binaural defects / next renderer gates

### Gain is still callback-quantized

`ChannelState::slew_gain` already produces a sample ramp, but the binaural handoff currently reduces it to one block-end scalar before `BinauralRenderer` consumes the block.

Fix requirement:

```text
same PCM + same gain timeline
40 / 240 / 960 sample caller partitions
→ same binaural output within calibrated numerical tolerance
```

Do not create a second independent gain state machine inside `BinauralRenderer`; `ChannelState` remains the authority.

### Position/motion is still callback-quantized

Object position is similarly advanced to a callback endpoint before the binaural renderer sees it. HRTF crossfades smooth the resulting staircase but do not restore the authored sample trajectory.

Motion gets a separate regression fixture after gain so the two defects remain attributable.

### Extent is not yet preserved in headphones

Scene/object size exists in inherited state and speaker/VBAP machinery. The binaural branch still collapses too much of it to a point.

### `BroadSource` still needs a real renderer

Reuse the existing size/extent state rather than inventing a duplicate width ontology.

### `DiffuseField` still needs a first-class musical field basis

The FDN is **room** energy, not a renderer for diffuse musical content. A spherical/Ambisonic or experimentally equivalent field representation remains a major candidate.

---

## Stereo scene inference

`renderer::stereo_inference` currently exposes inspectable evidence including:

- L/R pan/asymmetry;
- phase alignment/coherence proxy;
- directness / diffuseness;
- true complex mid and side magnitude;
- time-constant persistence;
- trajectory agreement and stability.

`renderer::scene_inference` adds conservative presentation evidence:

- frontal/foundation anchor support;
- lateral object-candidate support;
- broad-source evidence;
- diffuse-field evidence;
- spatial specificity;
- reassignment safety.

These modules are **not yet a complete realtime stereo → persistent scene path**.

Rear placement remains an Omniphony presentation choice constrained by hearing/music state, not a claim that stereo contained hidden authored rear coordinates.

---

## Known scene versus inferred scene

Keep the two validation problems independent.

### Known scene → binaural

Use reference bridge/layout/authored fixtures to ask:

> If the renderer receives correct geometry/state, does it produce a faithful and convincing headphone scene?

### Stereo → heard scene / presentation opportunity

Hold rendering understood and ask:

> Does analysis discover stable, musically useful organization without hallucinating hierarchy or source truth?

Do not debug inference and renderer physics simultaneously when one can be isolated.

---

## Fidelity contract

Every perceptual improvement should be accompanied by evidence about what it cost.

Current reusable measurements include:

- peak residual/null;
- RMS residual;
- peak/RMS level;
- crest factor;
- DC offset;
- matched level delta;
- FFT/frequency-response analysis;
- interaural lag/ITD.

Additional gates include:

- transient preservation;
- clipping/headroom;
- callback/chunk-size invariance;
- movement continuity;
- bass timing and weight;
- profile-switch continuity.

Human listening remains necessary for:

- externalization;
- front/back;
- elevation;
- depth;
- envelopment;
- source stability;
- fatigue;
- musical hierarchy;
- whether the enhancement sounds native to the song rather than imposed.

---

## Listener/headphone calibration

Keep distinct until evidence justifies combining them:

```text
listener HRTF
headphone response
driver ↔ ear interaction
room / presentation target
low-frequency integration
safety headroom
```

See [`../docs/HEADPHONE_CALIBRATION.md`](../docs/HEADPHONE_CALIBRATION.md).

---

## Contraction boundary

Already physically removed:

- Omniphony Studio;
- inherited product packaging/helper shells outside the current architecture;
- mpv product documentation;
- old Studio/WebGL archaeology;
- obsolete suite workflows/release jobs;
- demonstration backend crate;
- Lua/script backend crate.

Current deeper order:

```text
narrow generic contributor/plugin backend support
→ define neutral platform PCM input contract
→ migrate host callers
→ remove legacy PipeWire/IEC61937/SPDIF ingest topology
→ separate known-scene reference input from generic runtime plugin assumptions
→ add thin native platform shells only around the same tested core
```

The internal renderer-algorithm registry itself currently remains useful for comparing built-in VBAP/barycenter/distance/hybrid laws. Do not delete useful laboratory variation merely because the old external plugin UX disappeared.

See [`../docs/CONTRACTION_LEDGER.md`](../docs/CONTRACTION_LEDGER.md).

---

## North-star rule

The engine is not trying to maximize a spatial-effect score.

The target is:

```text
more dimension
+ more externalization
+ more stable auditory world

while preserving

clarity
transients
bass precision
timbre
dynamics
groove
hierarchy
recording character
```

After acclimation, bypass should make ordinary headphone playback feel lower-dimensional.

Bypass must **not** restore music that Omniphony damaged.
