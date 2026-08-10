# Fork contraction ledger

This ledger tracks which inherited Omniphony surfaces are retained, removed, transitional, or scheduled for dependency-aware replacement/removal.

The archive is upstream/Git history. This file exists so the fork does not repeatedly rediscover its own boundaries after context changes.

Status vocabulary:

- **KEEP** — directly useful to the portable product core or its calibration/validation;
- **PLATFORM** — useful, but owned by a thin OS/host adapter rather than the portable core;
- **TRANSITIONAL** — currently load-bearing, but carries inherited semantics that should shrink or move;
- **REPLACE THEN CUT** — the target owner/contract is known; remove only after callers migrate;
- **REMOVED** — physically absent from this fork; recover from upstream/history only if evidence changes.

The governing distinction is now:

```text
platform transport
≠
portable hearing / presentation / rendering core
```

Windows is the current implementation/listening laboratory, not the product identity.

See [`PLATFORM_PORTABILITY.md`](PLATFORM_PORTABILITY.md).

---

## Repository-level surfaces

| Surface | Status | Reason |
| --- | --- | --- |
| `omniphony-studio/` | **REMOVED** | Upstream visualization/control product; not the listener-facing shell. |
| `packaging/` | **REMOVED** | Upstream suite packaging rather than a portable-core contract. |
| root `scripts/` | **REMOVED** | JACK/service helper surface belonged to the upstream suite. |
| `docs/superpowers/` | **REMOVED** | Historical upstream implementation plans, not current contracts. |
| mpv product docs | **REMOVED** | mpv distribution is not normal ingestion architecture. |
| Studio/WebGL/Three.js investigation docs | **REMOVED** | Deleted product/debug surface. |
| old PipeWire-specific product plans | **REMOVED** | Historical host-product direction; Linux later gets a thin native adapter around the portable core. |
| upstream refactor diaries / duplicate translations | **REMOVED** | History already preserves them. |
| `README.md` | **KEEP** | Fork-native portable product identity. |
| `NOTICE.md` | **KEEP** | Permanent ancestry/licensing/attribution. |
| `CONTRIBUTING.md` | **KEEP** | Fork-native contribution and deletion law. |
| `docs/FORK_POLICY.md` | **KEEP** | Defines upstream as source/peer/ancestor. |
| `docs/PLATFORM_PORTABILITY.md` | **KEEP** | Defines OS-shell versus portable-core ownership. |
| `docs/SCENE_RENDERER_CONTRACT.md` | **KEEP** | Core scene/binaural semantics and known defects. |
| `docs/HEADPHONE_CALIBRATION.md` | **KEEP** | Listener/headphone calibration architecture. |
| `docs/REALTIME_CONTROL_CONTRACT.md` | **KEEP** | Sample-time/control-plane ownership. |

---

## Workflows

| Workflow | Status | Reason |
| --- | --- | --- |
| old `.github/workflows/ci.yml` | **REMOVED** | Tried to build deleted Studio plus inherited suite surfaces; became meaningless after contraction. |
| old `release.yml` | **REMOVED** | Upstream Studio release pipeline. |
| old `integration-build.yml` | **REMOVED** | Rolling Studio integration release. |
| old `liborender-release.yml` | **REMOVED** | Separate library-release product surface not currently shipped. |
| `windows-renderer.yml` | **KEEP FOR CURRENT VALIDATION** | Build/test oracle while Windows is the first integration platform. |

Current workflow lanes:

```text
portable renderer core
Windows renderer core
Windows x64 headless renderer-engine artifact
```

The headless artifact deliberately excludes the transitional host-audio/ASIO layer.

The workflow resolves Cargo dependencies without `--locked` because the repository does not currently track `omniphony-renderer/Cargo.lock`. Its Rust floor is 1.88, matching the currently resolved dependency graph.

Future platform CI should add native host lanes around the same portable-core tests rather than cloning the renderer architecture per OS.

---

# Renderer workspace dependency shape

Current high-level ownership:

```text
                         PORTABLE / LAB CORE

renderer
   ↑
orender_engine ───────────── runtime_control / sys (partly transitional)
   ↑
orender_ffi

bridge_api ← reference_bridge
             known-scene lab instrument

                         PLATFORM SHELL

host_audio
 ├→ audio_input
 └→ audio_output

legacy Linux ingest seam:
audio_input → PipeWire bridge → SPDIF / IEC61937
```

The names of inherited crates do not determine their final ownership. The actual dependencies and portable contract do.

---

## `renderer`

**KEEP.**

Core product/research substrate:

- stereo evidence;
- scene inference;
- binaural HRTF/ITD;
- object state;
- early reflections;
- late room field;
- VBAP/layout geometry retained where useful for known-scene truth.

Do not contract for aesthetics. Remove internal surfaces only when replacements/irrelevance are proven.

Active defect/validation work includes:

```text
callback-size-invariant binaural gain
callback-size-invariant binaural motion
source extent in headphones
BroadSource
DiffuseField
```

`dsp_fixtures::binaural_block_size` contains the first isolated known-defect reproducer for the gain problem.

---

## `dsp_fixtures`

**KEEP.**

Independent measurement and deterministic regression layer.

This crate should become increasingly platform-neutral and serve as the common ruler for Windows, macOS, Linux and mobile core builds.

---

## `audio_output`

**PLATFORM / TRANSITIONAL.**

Realtime output timing is required, but output APIs belong to platform shells.

Current crate mixes several inherited hosts:

```text
Linux  → PipeWire
Windows → CPAL with ASIO hard-wired today
macOS   → CPAL/CoreAudio
```

The portable core must not depend on those choices.

Target:

```text
PortableOutputContract
        ↓
Windows adapter
macOS adapter
Linux adapter
Android adapter
iOS adapter
```

ASIO may remain an optional Windows specialist adapter. It must not remain the mandatory Windows compile path.

---

## `audio_input`

**REPLACE THEN CUT / PLATFORM.**

Despite its generic name, the current public contract is inherited transport-specific machinery:

```text
InputMode = Bridge / Pipewire / PipewireBridge
InputBackend = Pipewire / Asio
fixed 7.1 mapping state
IEC61937 bridge clocks/pacing
```

Its Linux bridge directly owns `SpdifParser` and feeds `RInputTransport::Iec61937` packets into the decode bridge.

That is not the portable Omniphony input contract.

Do **not** add every future OS into this legacy enum surface.

First establish a small neutral boundary such as:

```text
AudioStreamFormat
  sample_rate
  channels / channel meaning
  sample type
  timeline position

AudioInputBlock
  PCM
  timestamp / sample position
  discontinuity / reset state
  optional trusted source metadata
```

Then implement thin platform adapters around it.

The product path is ordinary PCM music first. Richer authored scene/object transports may remain separate optional inputs/testing paths when genuinely useful.

---

## `host_audio`

**KEEP AS PLATFORM BOUNDARY / TRANSITIONAL IMPLEMENTATION.**

This crate already has the right *conceptual location*: audio device/input/output work sits above `orender_engine` and is explicitly not an engine dependency.

That separation should survive.

What changes is the implementation beneath it:

```text
current inherited audio_input/audio_output surfaces
        ↓
small platform-neutral host contract
        ↓
thin native adapters
```

Do not move OS device code back into `renderer` or `orender_engine` during cleanup.

---

## `orender_engine`

**KEEP / CONTRACT LATER.**

The headless engine is a useful portable boundary and CI target.

It still owns inherited generic configuration/backend compatibility that can shrink as callers disappear, but its no-audio-I/O shape is strategically valuable.

---

## `orender_ffi`

**KEEP WHILE USEFUL / NARROW.**

The C ABI is useful for:

- headless engine validation;
- native host integration;
- possible future language/platform shells.

It is no longer treated as a separate release product for its own sake.

Retain only capabilities that serve the portable engine boundary.

---

## `reference_bridge`

**KEEP AS LAB INSTRUMENT.**

Known-channel / known-scene file input isolates rendering from stereo inference and platform capture.

It is not normal listener ingestion.

Its package metadata points to this fork while source ancestry remains preserved by Git history/NOTICE.

---

## `bridge_api`

**TRANSITIONAL LAB/RUNTIME BOUNDARY.**

Still load-bearing for `reference_bridge` and inherited engine paths.

The useful concept is:

```text
known/rich source information
→ typed engine input
```

The inherited generic decoder-plugin product surface may shrink once the known-scene fixture contract is separated cleanly from runtime plugin assumptions.

Do not delete it merely because ordinary stereo does not need a decoder bridge; the reference path is valuable experimental truth.

---

## `spdif`

**REPLACE THEN CUT.**

This crate is specifically an IEC61937/S/PDIF parser for immersive bitstreams.

Current real owner:

```text
legacy Linux/PipeWire bridge ingest
```

Ordinary PCM music ingestion does not require it.

Remove only after `audio_input` callers have migrated to the neutral PCM input contract and any deliberate encoded-input fixture is given a separate explicit owner.

Do not let “we may support Linux later” preserve this architecture accidentally. Linux support means a Linux adapter around the portable core, not permanent inheritance of one old PipeWire/IEC61937 topology.

---

## `runtime_control`

**KEEP / TRANSITIONAL SURFACE.**

Timed engine state/control is useful.

Studio-shaped state, OSC-only assumptions and generic backend compatibility should contract as consumers disappear.

The key thing to preserve is not a particular control protocol but the realtime law:

```text
slow/control/model work
→ build/validate state off audio thread
→ timestamp / generation tag
→ atomically publish
→ audio thread follows sample-time trajectory
```

---

## `sys`

**KEEP SMALL / PORTABILITY SUPPORT.**

`sys` should own genuinely platform-specific lifecycle/IO abstractions shared by host code, not become a dumping ground for product semantics.

It currently contains cross-platform/platform-specific operational code. Retain pieces only where they protect a current adapter or diagnostic contract.

A recent CI compiler error in `sys::live_log` exposed and fixed a formatting-argument lifetime bug; this is a good example of why compiler validation must precede aggressive removal.

---

## `example_backend`

**REMOVED.**

The upstream demonstration crate, engine registration and workspace/dependency edges were removed in dependency-safe order.

Its purpose was to demonstrate arbitrary contributor backend extensibility, not to serve the headphone product.

---

## `script_backend`

**REMOVED.**

The Lua/user-programmable backend, engine registration and workspace/dependency edges were removed.

Arbitrary user backend scripting has no current product owner and unnecessarily pulled a scripting runtime into the workspace.

---

# Assets and layouts

## Known layouts

**KEEP.**

Speaker layouts remain useful known-scene geometry even though playback output is binaural.

```text
known source direction
→ renderer
→ objective / listening validation
```

They are instruments, not a requirement that ordinary stereo pass through a fake 7.1 transport.

## Reference/demo known-scene assets

**KEEP WHEN DETERMINISTIC / LICENSE-CLEAR.**

An upstream demo is not automatically marketing debris if it gives us controlled source geometry or known signal behavior.

## Branding assets

**KEEP ONLY IF CURRENT.**

No renderer dependency. Replace/remove independently.

---

# Ordered deeper cuts

Do not perform several layers in one giant commit.

## Cut 1 — backend demos — COMPLETE

```text
remove runtime registrations
→ remove engine dependencies
→ remove workspace members
→ delete example_backend
→ delete script_backend
```

Physical dependency edges/crates are gone. Compiler/CI verification is an independent ongoing gate.

## Cut 2 — generic backend product surface

Determine which dynamic backend registry/config compatibility still serves:

- the renderer itself;
- known-scene calibration;
- the portable FFI boundary.

Retain useful internal algorithms. Remove contributor/plugin UX whose only owner was deleted Studio/demo backends.

## Cut 3 — neutral platform input contract

Before deleting old transport code:

```text
define PCM/time/reset/source-metadata contract
→ add deterministic file/test implementation
→ add Windows validation adapter
→ migrate host_audio/engine callers
```

The contract itself must remain OS-neutral.

## Cut 4 — legacy PipeWire / IEC61937 / SPDIF ingest

After Cut 3 callers migrate, remove the inherited transport chain that no longer has an owner:

- encoded passthrough ingestion;
- S/PDIF parser;
- PipeWire-bridge-specific mode/control state;
- fixed-7.1 assumptions not retained as explicit calibration fixtures.

If a future Linux adapter needs PipeWire, build it around the neutral PCM contract rather than restoring the old coupled topology wholesale.

## Cut 5 — bridge-first normal ingestion

Separate:

```text
reference known-scene fixture interface
```

from

```text
generic runtime codec/plugin architecture
```

Then remove generic plugin machinery with no remaining product/test owner.

## Cut 6 — platform shells

Once Windows integration proves the core:

```text
portable core frozen by deterministic tests
→ add macOS shell
→ add Linux shell
→ add Android/iOS shells
```

Do not delete portability abstractions merely because the first live host is Windows. Delete only obsolete **implementations** and semantics.

---

# What must not be accidentally deleted

Protect:

- measured/parametric/SOFA HRTF support;
- HRTF interpolation and motion continuity;
- analytic ITD and direct-arrival timing contract;
- early directional room cues;
- late field/externalization work;
- object extent/size state until broad-source binaural rendering consumes it;
- deterministic fixture generation;
- fidelity measurements;
- callback-size-invariance tests/reproducers;
- known scene geometry;
- the headless portable engine boundary;
- host/core separation;
- anything required to reproduce an audible regression.

---

# Acceptance rule for each contraction commit

A contraction is successful only when:

```text
less repository surface
+
clearer ownership
+
stronger portability boundary
+
no lost product/test capability
+
compiler/tests remain meaningful
```

Deleting code that makes `tree` prettier while destroying observability, portability, or calibration truth is negative progress.
