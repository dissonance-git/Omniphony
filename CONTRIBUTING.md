# Contributing to Omniphony

Omniphony is a free and open-source spatial audio renderer for headphones. Contributions are welcome across DSP, realtime systems, Windows audio integration, spatial-scene handling, testing, documentation, and portability.

Read the root [`README.md`](README.md) before making changes. It defines the product architecture and source-authority model. More detailed contracts live under [`docs/`](docs/).

## Project goal

Omniphony aims to provide one open spatial renderer that can:

```text
stereo
→ preserve the finished master
→ infer only missing spatial structure
→ enhance through Omniphony

5.1 / 7.1 / height PCM
→ preserve authored channels and positions
→ infer less because more source geometry is known
→ enhance through the same renderer

8.1.4.4 static spatial scenes
→ preserve supplied fixed spatial roles
→ enhance through the same renderer

dynamic spatial objects
→ preserve identity and continuous XYZ motion
→ enhance through the same renderer
```

Every path ends in one final binaural render to an ordinary stereo headphone endpoint.

The richer the source truth, the less Omniphony should invent.

## Core invariants

### Preserve source authority

Keep these concepts distinct:

```text
source truth
≠ signal evidence
≠ presentation hypothesis
≠ placement choice
```

Static scene lanes use explicit authority states:

```text
AUTHORED   supplied by the source or host
DERIVED    inferred or created by Omniphony
EMPTY      no trustworthy signal assigned
```

Do not relabel inferred geometry as authored. Do not discard real surround, height, or object metadata and then attempt to reconstruct it from stereo.

### One renderer

Stereo, native surround, height beds, static objects, and dynamic objects are different ingress representations, not different Omniphony products.

They should converge on the same portable scene semantics and final renderer wherever possible.

### Protect fidelity

Spatial improvement may not be purchased by damaging the recording.

A change that improves apparent width, height, distance, or envelopment but degrades clarity, transient impact, tonal identity, center stability, bass coherence, dynamics, or fatigue is not automatically an improvement.

For stereo material, the finished master remains the musical authority.

### Keep the portable core portable

Portable renderer code owns concepts such as:

```text
source scene
channel/object geometry
source authority
presentation state
spatial rendering
binaural output
```

Platform hosts own concepts such as:

```text
device and session discovery
platform audio APIs
endpoint association
format translation
clock and recovery behavior
platform UI/service integration
```

Do not leak Windows endpoint identities, WASAPI-specific state, device names, or host lifecycle assumptions into portable renderer semantics.

### Realtime code must remain realtime-safe

Keep the realtime path bounded and deterministic for equivalent continuous input/state.

Do not perform these operations from realtime callbacks:

- filesystem or network I/O;
- device/session enumeration;
- model inference that is not explicitly designed for realtime use;
- large or unbounded allocations;
- SOFA parsing/import;
- UI work;
- blocking logging or unbounded mutex waits.

Prefer preallocation, bounded queues, explicit discontinuity/reset behavior, and worker-owned allocating DSP where needed.

## Spatial model

The canonical fixed Windows spatial vocabulary is 8.1.4.4:

```text
horizontal
FL FR C LFE SL SR BL BR BC

upper
TFL TFR TBL TBR

lower
BFL BFR BBL BBR
```

This is a semantic coordinate frame, not a claim that every source contains seventeen authored channels.

Dynamic XYZ objects remain continuous objects and should not be prematurely snapped to static anchors.

The denser 22-direction Omniphony shell is internal rendering/support geometry. It is not an authored input format and must not be exposed as though a source supplied 22 channels.

See [`docs/windows-spatial-input-contract.md`](docs/windows-spatial-input-contract.md) and [`docs/scene-renderer-contract.md`](docs/scene-renderer-contract.md).

## Windows architecture

Windows is the first product host.

The current production path uses a format-changing stream SFX that can accept authored multichannel PCM while the physical headphone endpoint remains stereo. A separate stereo endpoint EFX is retained as a transactional recovery floor.

Windows-specific work should preserve these properties:

- one installer;
- no required virtual cable;
- no required loopback host;
- no foreground audio application that must remain open;
- endpoint power cycling or temporary absence must not erase installation state;
- richer source input must be preserved before the final binaural reduction;
- already-binaural material must not be blindly virtualized a second time.

Raw Windows Spatial Audio static/dynamic object ingress is a richer host boundary than conventional PCM and must be treated as such. Do not claim object interception without evidence that the original object identities, PCM, and positions actually reach Omniphony before another headphone renderer consumes them.

## Good contribution areas

Useful contributions include:

- HRTF/ITD and binaural rendering improvements;
- source-authority and scene-model correctness;
- multichannel and object ingress;
- height/front-back localization;
- distance and externalization;
- source extent and diffuseness;
- realtime safety and latency;
- endpoint/device continuity;
- deterministic fixtures and regression tests;
- already-binaural detection/bypass;
- head tracking or HRTF personalization that fits the same scene model;
- platform-host work that keeps the portable renderer clean;
- documentation that clarifies public contracts without turning into a development diary.

Large architectural changes should explain which product invariant they improve and why a smaller change is insufficient.

## Testing

The renderer workspace currently requires Rust 1.88.0 or newer.

From `omniphony-renderer/`, useful focused commands include:

```sh
cargo test -p renderer
cargo test -p renderer --test source_shell_spread_energy
cargo test -p orender_engine --lib --tests
cargo test -p orender_engine --test source_shared_wet_extent
cargo test -p source_ffi --lib --tests
cargo test -p source_ffi --test runtime_spatial_mode
cargo test -p realtime_ffi
```

Windows host changes should also pass the relevant APO build, COM/lifecycle, manifest, realtime ABI, installer, and endpoint/client-format checks in CI.

CI failures are evidence. Do not make a gate green by weakening the requirement unless the requirement itself has been shown to be wrong.

## Audible DSP changes

For an audible change, state:

```text
What intended percept improved?
What source types are affected?
What measurable behavior changed?
What fidelity cost, if any, was observed?
```

Useful objective checks include:

- null/residual tests where identity is expected;
- peak and RMS behavior;
- crest factor;
- DC;
- frequency response;
- interaural delay/ITD;
- transient timing;
- bass timing/coherence;
- clipping/headroom;
- block-size/callback invariance;
- state-switch continuity;
- non-finite handling;
- source identity and channel/object provenance.

Human listening remains required for perceptual questions such as:

- externalization;
- front/back discrimination;
- elevation and below-listener localization;
- radial depth;
- source extent;
- image stability;
- envelopment;
- room naturalness;
- direct-source solidity;
- bass and groove integrity;
- timbre;
- fatigue;
- preference.

Listening evidence should be loudness-aware and route-clean. Do not draw conclusions while duplicate physical paths or multiple headphone virtualizers are active unintentionally.

## Documentation and evidence

Public-facing documents should describe stable product behavior, architecture, contracts, and supported capabilities.

Machine-specific debugging transcripts, personal hardware settings, one-off game configurations, temporary hypotheses, and dated experiment narratives belong in focused evidence/research material rather than the root README or contributor guide.

Keep these evidence states distinct:

```text
source builds
≠ unit/regression tests pass
≠ host API negotiation succeeds
≠ endpoint association succeeds
≠ a real application supplies the expected source representation
≠ physical listening confirms the intended percept
```

Do not promote a capability beyond the strongest evidence actually obtained.

## Upstream and third-party work

Omniphony is derived from the original [`mgth/Omniphony`](https://github.com/mgth/Omniphony) project. Preserve upstream attribution and licensing. See [`NOTICE.md`](NOTICE.md).

External projects, papers, datasets, and proprietary spatial renderers may be useful references or comparison targets, but they are not automatic dependency choices. Check licensing and redistribution implications before adding code, data, HRTFs, models, or other assets.

## Submitting changes

Prefer focused changes that answer one clear question and include the smallest coherent implementation plus the tests or evidence needed to support it.

A good contribution should make it easy to answer:

```text
What changed?
Why does it belong in Omniphony?
Which source representations are affected?
Which invariant protects against regression?
How was it validated?
```

Avoid combining unrelated renderer tuning, host plumbing, repository restructuring, and calibration changes into one patch unless they truly cannot be separated.
