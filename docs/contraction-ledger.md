# Fork contraction ledger

This ledger records which inherited Omniphony surfaces are retained, removed, transitional, or waiting on a safer replacement.

Git history and upstream are the archive. The working tree should contain what the Windows headphone product, its protected renderer, and its validation machinery still need.

The root `README.md` owns product direction.

Current product identity:

```text
Omniphony for Headphones
Windows-first headphone product
built from the existing Omniphony renderer
```

---

## Status vocabulary

- **KEEP** — directly useful to the current product, renderer, or validation.
- **WINDOWS HOST** — belongs to native Windows transport/device work rather than renderer semantics.
- **LAB / REFERENCE** — not normal listener UX, but valuable for controlled truth or A/B work.
- **TRANSITIONAL** — still load-bearing but carries inherited assumptions that should shrink.
- **REPLACE THEN CUT** — target owner is known; migrate callers first.
- **REMOVED** — absent from the working tree; recover from history/upstream only if evidence changes.
- **DEFERRED** — potentially useful later but not part of the current milestone.

Governing distinction:

```text
Windows transport / device plumbing
≠
Omniphony renderer / scene / calibration semantics
```

---

# 1. Repository-level surfaces

| Surface | Status | Reason |
| --- | --- | --- |
| `README.md` | **KEEP** | Canonical master plan and re-entry surface. |
| `NOTICE.md` | **KEEP** | Permanent ancestry/licensing/attribution. |
| `CONTRIBUTING.md` | **KEEP** | Private development rules for AI/tooling/contributors. |
| `docs/windows-audio-route.md` | **KEEP** | Live Windows single-path transport decision and ladder. |
| `docs/windows-integration-research.md` | **KEEP / PARKED** | Windows API, endpoint, Spatial Sound, APO/driver research. |
| `docs/influence-ledger.md` | **KEEP / PARKED** | Durable external research memory. |
| `docs/headphone-rendering-research.md` | **KEEP** | Practical renderer experiments/listening path. |
| `docs/scene-renderer-contract.md` | **KEEP** | Renderer/scene semantics and known gaps. |
| `docs/realtime-control-contract.md` | **KEEP** | Sample-time/control ownership. |
| `docs/music-presentation-contract.md` | **KEEP / LATER** | Optional adaptive-presentation safety laws. |
| `docs/headphone-calibration.md` | **KEEP / LATER** | Calibration architecture. |
| old standalone portability policy | **REMOVED** | Live host/core law is already in README/realtime contracts. |
| old standalone fork policy | **REMOVED** | Live upstream/deletion laws are already in README/CONTRIBUTING/this ledger. |
| old `spatial-dsp` migration diary | **REMOVED** | Useful mechanisms are implemented or retained in README/contracts/tests; history preserves archaeology. |
| `omniphony-studio/` | **REMOVED** | Upstream visualization/control product is not this fork's listener shell. |
| old suite packaging/release surfaces | **REMOVED** | Did not serve the Windows headphone product. |
| root script/demo backend residue | **REMOVED** | No current product/test owner. |
| old JACK/mpv/PipeWire product docs | **REMOVED** | Historical host direction, not current product architecture. |

---

# 2. Workflows

| Workflow | Status | Reason |
| --- | --- | --- |
| `.github/workflows/windows-renderer.yml` | **KEEP** | Current portable/core/Windows build, test, P0 package and packaged-reference oracle. |
| old Studio/suite release workflows | **REMOVED** | Built deleted inherited product surfaces. |

The P0 Windows run completed successfully on 2026-08-10. Its only reported compiler warning was an `unused_mut` in a test-local closure in `orender_engine`; that is cosmetic and can be removed with the next code-touching commit.

The current workflow deliberately validates the ordinary Windows host without requiring the optional Steinberg ASIO SDK.

---

# 3. Central renderer and validation crates

## `renderer` — KEEP

Central perceptual/product substrate.

Protect:

- binaural HRTF/ITD behavior;
- measured/parametric/SOFA HRTF support;
- interpolation/motion continuity machinery;
- object/scene state;
- early directional reflections;
- late room-field machinery;
- stereo evidence currently in use;
- bass/foundation safeguards;
- known-layout geometry useful for validation.

Do not rewrite this crate for architectural aesthetics.

## `dsp_fixtures` — KEEP

Independent deterministic measurement/regression layer for null/fidelity checks, callback invariance where applicable, known HRTF/scene truth, and cross-path comparison.

## `orender_engine` — KEEP

Headless engine boundary. Valuable for deterministic validation and for comparing host routes without device plumbing.

## `orender_ffi` — KEEP WHILE USEFUL / NARROW

Embedding/native-host boundary, not a product for its own sake.

## `reference_bridge` — LAB / REFERENCE, KEEP

Known-channel/known-scene file input isolates renderer quality from stereo inference and Windows capture/device timing. P0 now depends on this truth lane.

## `bridge_api` — TRANSITIONAL LAB/RUNTIME BOUNDARY

Keep the typed rich-source/reference concept while generic plugin/decoder assumptions shrink.

---

# 4. Windows host frontier

## `windows_host` — WINDOWS HOST, KEEP

Current native product-shell/prototype lane.

It now owns:

```text
output-device discovery
self-excluding loopback diagnostic
native output smoke path
protected reference render/playback
packaged render-only validation
```

Loopback remains diagnostic because it copies rather than intercepts system playback.

## `realtime_ffi` — KEEP / CURRENT HOST SEAM

Narrow interleaved-f32 PCM ABI. First implementation is bit-exact identity.

It exists so host evolution does not redefine renderer semantics.

## `host_audio` — KEEP AS HOST/ENGINE BOUNDARY

Conceptually the right owner for device/input/output integration above the headless engine. Evolve it from actual Windows needs rather than toward a universal host framework.

## `audio_output` — WINDOWS HOST / TRANSITIONAL

Inherited output infrastructure remains useful, including specialist ASIO support.

Target relationship:

```text
normal Windows output
→ WASAPI/native path

specialist/reference output
→ ASIO where useful
```

ASIO must not be deleted merely because a normal route exists, and must not be mandatory for ordinary users.

## `audio_input` — REPLACE THEN CUT / WINDOWS HOST FRONTIER

Inherited input abstractions still carry older PipeWire/bridge/encoded-transport assumptions.

Target boundary is simpler:

```text
ordinary Windows PCM
→ sample/time identity
→ discontinuity/reset state
→ Omniphony engine
```

Migrate callers before deleting legacy input machinery.

## `spdif` — REPLACE THEN CUT

IEC61937/S/PDIF parsing has no owner in ordinary Windows music once the normal PCM boundary is established. Preserve only if a deliberate encoded-input reference use remains.

## `sys` — KEEP SMALL

Own genuinely platform-specific lifecycle/I/O support shared by host code. Do not let it become a dumping ground for scene or product policy.

---

# 5. Realtime/control infrastructure

## `runtime_control` — KEEP / TRANSITIONAL

Timed state/control remains valuable.

Preserve:

```text
slow/control/model work
→ build/validate away from audio thread
→ generation/time tag
→ publish bounded immutable state
→ audio thread follows sample-time behavior
```

Studio-shaped state, OSC assumptions, and generic compatibility can shrink as their remaining callers disappear.

---

# 6. Protected assets and controlled geometry

## Binaural baselines — KEEP, PROTECTED

Especially:

```text
upstream-demo-reference.yaml
baseline-room.yaml
dry-binaural.yaml
```

The upstream-demo reference is the perceptual ancestor. Fork room/dry configs are comparisons, not replacements by default.

## Known speaker layouts — LAB / REFERENCE, KEEP

They provide controlled geometry for known-source-direction renderer validation. Their existence does not mean ordinary stereo should permanently pass through a fake 7.1/7.1.4 bed.

## Demo/reference audio — KEEP WHEN LICENSE-CLEAR AND DETERMINISTIC

P0 demonstrates why non-consumer reference material is valuable: it isolates renderer/packaging truth before arbitrary stereo inference enters the experiment.

---

# 7. Ordered deeper cuts

Do not combine many ownership changes into one giant deletion merely because the final product is narrower.

### Cut A · Normal PCM host boundary

```text
define PCM / time / reset contract
→ add deterministic source
→ add Windows adapter
→ migrate callers
```

### Cut B · Legacy PipeWire / IEC61937 ingest

After normal Windows PCM callers migrate, remove inherited encoded/PipeWire bridge assumptions that no longer serve a current product/test owner.

### Cut C · Generic bridge/plugin residue

Separate the known-scene/reference interface from generic runtime plugin assumptions, then remove unowned generic machinery.

### Cut D · Other platforms

**DEFERRED.** If Windows earns a port later, design it from the then-current engine boundary and current platform APIs rather than restoring historical host plans wholesale.

---

# 8. What must not be accidentally deleted

Protect:

- upstream-demo perceptual control;
- measured/parametric/SOFA HRTF support;
- HRTF interpolation and motion continuity;
- analytic ITD/direct-arrival timing contract;
- early directional room cues;
- late room/externalization machinery until experiments prove a better boundary;
- object extent/size state until explicitly evaluated;
- deterministic fixtures and fidelity measurements;
- callback-size regression tests/reproducers;
- known-scene geometry;
- headless engine boundary;
- host/core separation;
- ASIO route while it remains useful;
- any artifact required to reproduce an audible regression.

---

# 9. Acceptance rule for each contraction

A contraction succeeds only when it produces:

```text
less unowned surface
+
clearer ownership
+
no lost protected sound
+
no lost A/B or regression observability
+
no lost current Windows capability
+
meaningful compiler/tests remain
```

A prettier tree that makes the sound harder to explain or reproduce is negative progress.

The current frontier is not "delete more." It is **make the Windows host simple and reliable enough to turn the protected renderer into an everyday listening product without disturbing the renderer underneath it.**
