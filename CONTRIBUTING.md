# Contributing to Omniphony

Omniphony is an independent Windows-first stereo-music → auditory-scene → binaural fork of [`mgth/Omniphony`](https://github.com/mgth/Omniphony).

The upstream project remains the technical ancestor and a major source of useful renderer work, but contributions here should serve this fork's narrower product rather than preserve the upstream suite or extension surface.

Read [`docs/FORK_POLICY.md`](docs/FORK_POLICY.md) before making architectural changes.

## Product invariant

The listener-facing goal is:

> ordinary stereo music should become a stable, externalized, full-sphere headphone presentation without losing clarity, timbre, bass timing, transient precision, dynamics, stereo relationships, or musical hierarchy.

Spatial spectacle is not a substitute for fidelity.

## Repository priorities

The highest-value contribution areas are:

- realtime stereo evidence and persistent scene inference;
- libaural scene-state integration;
- binaural HRTF/ITD correctness;
- sample-accurate object motion and gain;
- broad-source and diffuse-field rendering;
- externalization / room cues;
- listener and headphone calibration;
- Windows capture/output integration;
- realtime safety and bounded latency;
- deterministic DSP validation;
- objective fidelity measurement;
- controlled listening fixtures.

Do not add a large framework, plugin system, UI surface, codec path, or compatibility layer merely because upstream or another spatial-audio project has one.

## Current repository layout

```text
omniphony-renderer/
  renderer/           spatial + binaural DSP and scene evidence
  dsp_fixtures/       deterministic measurement / regression tools
  audio_output/       output and timing infrastructure
  audio_input/        inherited input/control code being reduced toward the Windows product
  host_audio/         host audio integration
  orender_engine/     headless renderer construction / glue
  orender_ffi/        embedding boundary
  reference_bridge/   deterministic known-scene/file calibration source
  bridge_api/         inherited/reference scene input boundary
  runtime_control/    control/state plumbing still used by the engine
  sys/                platform integration
  spdif/              inherited transport code; not part of the long-term stereo product
  example_backend/    inherited extension demo; scheduled for removal
  script_backend/     inherited scriptable backend; scheduled for removal

layouts/              known-scene calibration geometry
docs/                 current fork contracts, validation and research decisions
.github/workflows/     compiler/test/listening-artifact pipeline
```

Some inherited crates remain temporarily because removal must be dependency-aware. Their presence is not a promise of permanent product scope.

## Build and test

The workspace currently uses Rust `1.87.0`.

```sh
cd omniphony-renderer

cargo fmt --all -- --check
cargo test --locked -p dsp_fixtures
cargo test --locked -p renderer
```

The GitHub workflow also tests the core on Windows and builds a Windows x64 listening artifact.

The workflow lives at:

```text
.github/workflows/windows-renderer.yml
```

## DSP changes

Changes to anything audible should answer two independent questions:

```text
Did the intended perceptual/spatial behavior improve?

and

What did the transformation cost in fidelity?
```

Useful automatic measurements include:

- strict null / residual level;
- RMS and peak level;
- crest factor;
- DC offset;
- frequency response;
- interaural lag / ITD;
- block/chunk-size invariance;
- transient and dynamic behavior;
- clipping/headroom.

Human listening remains required for externalization, front/back discrimination, elevation, image stability, depth, envelopment, fatigue, hierarchy and preference.

### Intentional golden changes

If an existing deterministic golden is intentionally changed, regenerate it only after the difference is understood and record the relevant measurement rather than blessing unexplained drift.

```sh
cd omniphony-renderer
OMNIPHONY_BLESS_GOLDENS=1 cargo test -p renderer -- --nocapture
```

See [`omniphony-renderer/dsp_fixtures/README.md`](omniphony-renderer/dsp_fixtures/README.md).

## Realtime contract

Audio-thread work should be:

- allocation-free after initialization where practical;
- lock-free or bounded/non-blocking on the realtime path;
- independent of host callback partitioning;
- deterministic for equivalent continuous input;
- explicit about reset/discontinuity semantics;
- measured rather than assumed fast.

A host changing from 40 to 128 to 1024 samples per callback must not change the semantic scene, object trajectory, gain trajectory or room evolution.

Expensive calibration, SOFA conversion, corpus generation, symbolic analysis and model work belong off the audio thread.

## Scene semantics

Keep these concepts separate:

```text
signal evidence
≠ auditory object
≠ scene hypothesis
≠ presentation choice
```

And:

```text
DirectObject
≠ BroadSource
≠ DiffuseField
≠ RoomField
```

Do not infer literal rear recording metadata from ordinary stereo where none exists. Rear placement can be a presentation decision constrained by evidence and musical role.

## Upstream Omniphony

Treat `mgth/Omniphony` as a source/peer/ancestor.

When useful upstream work appears:

```text
inspect exact mechanism
→ take the smallest relevant change
→ validate it in this fork
→ adapt it to our product
```

Do not merge broad upstream changes solely to maintain structural parity.

When this fork discovers a general renderer fix:

```text
prove it here
→ isolate the general part from fork-specific policy
→ offer it upstream if it improves upstream on its own terms
```

This fork should improve itself before using upstream as the integration target.

## External projects

External repositories are mechanism sources, not dependency wish lists.

For a new influence, document:

1. the exact useful mechanism;
2. whether it belongs to libaural, Omniphony runtime, Windows integration, calibration or testing;
3. the smallest controlled experiment that can test it;
4. licensing/attribution implications if code or data would be incorporated.

The durable influence ledger lives in `dissonance-git/libaural`.

## Scope contraction

Deletion is a valid improvement.

Keep inherited code when it is required by:

- the Windows listening product;
- realtime renderer behavior;
- stereo scene inference;
- HRTF/headphone calibration;
- deterministic known-scene validation;
- a clearly load-bearing dependency.

Otherwise prefer removing it from this fork and recovering it later from upstream/Git history if evidence changes.

Do not locally archive entire deleted products just to feel safe. Git already is the archive.

## Pull requests and commits

- Target `main`.
- Keep changes focused enough that a regression can be localized.
- Add or strengthen a test when fixing a reproducible bug.
- Avoid unexplained DSP constants; document the measurement or hypothesis they encode.
- Keep comments and new docs in English.
- Run formatting and relevant tests before submitting.
- Do not add permanent compatibility machinery for a product surface this fork intentionally removed.

Contributions are licensed under the inherited `GPL-3.0-or-later` license.
