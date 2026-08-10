# Fork contraction ledger

This ledger tracks which inherited Omniphony surfaces are retained, removed, transitional, or scheduled for dependency-aware removal.

The archive is upstream/Git history. This file exists so the fork does not repeatedly rediscover its own boundaries after context changes.

Status vocabulary:

- **KEEP** — directly useful to the product or its calibration/validation;
- **TRANSITIONAL** — currently load-bearing, but carries broader inherited semantics;
- **CUT NEXT** — no target-product owner; removal edge is understood;
- **REMOVED** — physically absent from this fork; recover from upstream/history only if evidence changes.

---

## Repository-level surfaces

| Surface | Status | Reason |
| --- | --- | --- |
| `omniphony-studio/` | **REMOVED** | Upstream visualization/control product; not the listener-facing Windows product shell. |
| `packaging/` | **REMOVED** | Arch/Linux/Studio packaging belongs upstream. |
| root `scripts/` | **REMOVED** | JACK/service helper surface belonged to upstream suite. |
| `docs/superpowers/` | **REMOVED** | Historical upstream implementation plans, not current product contracts. |
| mpv product docs | **REMOVED** | mpv distribution is not target ingestion architecture. |
| Studio/WebGL/Three.js investigation docs | **REMOVED** | Deleted product/debug surface. |
| Linux/PipeWire investigation plans | **REMOVED** | Not current Windows product direction. |
| upstream refactor diaries / duplicate French docs | **REMOVED** | History already preserves them. |
| `README.md` | **KEEP** | Fork-native product identity. |
| `NOTICE.md` | **KEEP** | Permanent ancestry/licensing/attribution. |
| `CONTRIBUTING.md` | **KEEP** | Fork-native contribution and deletion law. |
| `docs/FORK_POLICY.md` | **KEEP** | Defines upstream as source/peer/ancestor. |
| `docs/SCENE_RENDERER_CONTRACT.md` | **KEEP** | Core stereo-scene/binaural semantics and known defects. |
| `docs/HEADPHONE_CALIBRATION.md` | **KEEP** | Listener/headphone calibration architecture. |

---

## Workflows

| Workflow | Status | Reason |
| --- | --- | --- |
| old `.github/workflows/ci.yml` | **REMOVED** | Tried to build deleted Studio, macOS and Linux/PipeWire suite; guaranteed failure after contraction. |
| old `release.yml` | **REMOVED** | Cross-platform Studio release pipeline. |
| old `integration-build.yml` | **REMOVED** | Rolling Studio integration release. |
| old `liborender-release.yml` | **REMOVED** | Separate cross-platform library-release product not being shipped. |
| `windows-renderer.yml` | **KEEP** | Single fork CI/listening-artifact pipeline. |

Current workflow lanes:

```text
portable renderer core
Windows renderer core
Windows x64 listening artifact
```

The workflow intentionally resolves Cargo dependencies without `--locked` because this repository does not track `omniphony-renderer/Cargo.lock`.

---

## Renderer workspace crates

### `renderer`

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

### `dsp_fixtures`

**KEEP.**

Independent measurement and deterministic regression layer.

### `audio_output`

**TRANSITIONAL / KEEP CORE.**

Realtime output timing is load-bearing. Linux/PipeWire and cross-platform portions may contract around Windows once dependency edges are explicit.

### `audio_input`

**TRANSITIONAL.**

Current crate is largely inherited Linux/PipeWire/fixed-channel control machinery. The target owner is future Windows stereo system/player capture.

Do not simply delete it before the Windows route exists, because shared control/host code currently references it.

Desired end state:

```text
Windows stereo PCM capture
→ small bounded input contract
→ realtime scene analysis
```

not

```text
PipeWire/IEC61937/fixed-7.1 source router
```

### `host_audio`

**TRANSITIONAL / KEEP CORE.**

Useful host boundary, but should eventually expose only product-required Windows capture/output behavior plus deterministic test hooks.

### `orender_engine`

**TRANSITIONAL / KEEP CORE.**

Useful headless renderer construction, but still owns substantial inherited generic config/backend compatibility.

This is a major future simplification site.

### `orender_ffi`

**TRANSITIONAL.**

Useful embedding/testing boundary today. It is no longer a separately released cross-platform product.

Keep while it simplifies testing/Windows integration. Remove or narrow if the final system route does not need it.

### `reference_bridge`

**KEEP AS TEST INSTRUMENT.**

Known-channel/known-scene truth isolates binaural rendering from stereo inference.

It is not normal product ingestion.

### `bridge_api`

**TRANSITIONAL.**

Still load-bearing for reference bridge and inherited engine paths. Generic decoder-plugin compatibility is not product identity.

Narrow after the known-scene fixture contract is separated from runtime plugin assumptions.

### `spdif`

**CUT AFTER INPUT REWRITE.**

IEC61937/S/PDIF is tied into the inherited bridge/PipeWire decode path. Ordinary Windows stereo music does not require it.

Known dependency edges include:

```text
spdif
→ audio_input Linux bridge ingestion
→ CLI decoder thread transport auto-detection
```

Remove as one transport-path contraction, not as a leaf deletion.

### `runtime_control`

**TRANSITIONAL / KEEP CORE.**

Engine state/control remains useful; Studio-shaped state and generic option/backend compatibility should contract as callers disappear.

### `sys`

**TRANSITIONAL / KEEP WINDOWS CORE.**

Platform integration remains needed. Linux/systemd/cross-platform code can shrink after the active Windows lifecycle is established.

### `example_backend`

**REMOVED.**

The upstream demonstration crate, engine registration and workspace/dependency edges were removed atomically in dependency-safe order. Its purpose was to demonstrate arbitrary contributor backend extensibility, not to serve the headphone product.

Recover from upstream/Git history only if a future external-backend SDK becomes a real product requirement.

### `script_backend`

**REMOVED.**

The Lua/user-programmable backend, engine registration and workspace/dependency edges were removed atomically. Arbitrary user backend scripting has no owner in the target listener product and unnecessarily pulled an embedded scripting runtime into the workspace.

Recover from upstream/Git history only if future evidence establishes a product need.

---

## Assets and layouts

### Known layouts

**KEEP.**

Speaker layouts are useful known-scene geometry even though the final product is headphone-first.

They allow:

```text
known source direction
→ binaural renderer
→ objective / listening validation
```

### Reference/demo known-scene assets

**KEEP WHEN DETERMINISTIC.**

An upstream demo is not automatically marketing debris if it provides known authored source geometry.

The bundled binaural demo configuration currently serves this purpose.

### Branding assets

**KEEP ONLY IF CURRENT.**

Branding has no renderer dependency. Replace/remove independently when the product identity changes.

---

## Deeper code cuts, ordered

Do not attempt all of these in one commit.

### Cut 1 — backend demos — COMPLETE

Completed in dependency-safe order:

```text
remove runtime registrations
→ remove engine dependencies
→ remove workspace members
→ delete example_backend/
→ delete script_backend/
```

Compiler/CI confirmation remains a separate gate; physical dependency edges and crate files are gone.

### Cut 2 — generic backend product surface

After Cut 1, determine which dynamic backend registry/config compatibility still serves known-scene tests or the actual renderer.

Retain useful internal algorithms; remove contributor/plugin UX whose only consumer was deleted Studio/backend demos.

### Cut 3 — PipeWire / IEC61937 / SPDIF input product

First establish the Windows stereo input contract, then remove:

- Linux virtual sink product code;
- encoded passthrough ingestion;
- S/PDIF transport parser where no fixture depends on it;
- PipeWire-specific control state;
- fixed-7.1 input assumptions that no longer serve calibration.

### Cut 4 — bridge-first normal ingestion

Separate:

```text
reference known-scene fixture interface
```

from

```text
generic runtime codec/bridge plugin system
```

Then remove the latter if Windows normal playback no longer needs it.

### Cut 5 — cross-platform lifecycle

Once Windows capture/output is mature, remove macOS/Linux platform lifecycle code that no longer protects any portable DSP test.

---

## What must not be accidentally deleted

The contraction is not an excuse to throw away difficult-but-useful machinery.

Protect:

- measured/parametric/SOFA HRTF support;
- HRTF interpolation and motion continuity;
- analytic ITD and direct-arrival timing contract;
- early directional room cues;
- late field/externalization work;
- object extent/size state until broad-source binaural rendering consumes it;
- deterministic fixture generation;
- fidelity measurements;
- known geometry;
- Windows realtime timing code;
- anything required to reproduce an audible regression.

---

## Acceptance rule for each contraction commit

A deletion is successful when:

```text
less repository surface
+
clearer ownership
+
no lost product/test capability
+
compiler/tests still meaningful
```

Deleting code that merely makes the tree smaller while destroying observability is not progress.
