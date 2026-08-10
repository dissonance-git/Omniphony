# Fork contraction ledger

This ledger records which inherited Omniphony surfaces are retained, removed, transitional, or waiting on a safer replacement.

The archive is Git history and upstream. This file exists so future cleanup does not repeatedly rediscover the same dependency boundaries or accidentally delete the machinery that makes audible regressions observable.

The root `README.md` owns product direction.

Current product identity:

```text
native Windows headphone product
built from the existing Omniphony renderer
```

Portability is a boundary guardrail, not an active multi-platform roadmap.

---

## Status vocabulary

- **KEEP** — directly useful to the current product, renderer, or validation.
- **WINDOWS HOST** — belongs to the native Windows shell rather than renderer semantics.
- **LAB / REFERENCE** — not normal listener UX, but valuable for controlled truth or A/B work.
- **TRANSITIONAL** — currently load-bearing, but carries inherited semantics that should shrink or move.
- **REPLACE THEN CUT** — target owner is known; do not delete until callers move.
- **REMOVED** — physically absent; recover from upstream/history only if evidence changes.
- **DEFERRED** — potentially useful later, but not part of the current Windows milestone.

The governing distinction is:

```text
Windows transport / device plumbing
≠
Omniphony renderer / scene / calibration semantics
```

---

# 1. Repository-level surfaces

| Surface | Status | Reason |
| --- | --- | --- |
| `README.md` | **KEEP** | Canonical private master project plan and re-entry surface. |
| `NOTICE.md` | **KEEP** | Permanent ancestry/licensing/attribution. |
| `CONTRIBUTING.md` | **KEEP** | Contributor/build behavior where still applicable. |
| `docs/FORK_POLICY.md` | **KEEP** | Defines selective upstream relationship and perceptual ancestry. |
| `docs/PLATFORM_PORTABILITY.md` | **KEEP** | Host/core boundary guardrail only; not an active porting roadmap. |
| `docs/headphone-rendering-research.md` | **KEEP** | Practical renderer experiments and Windows listening route. |
| `docs/SCENE_RENDERER_CONTRACT.md` | **KEEP** | Renderer/scene semantics and known defects. |
| `docs/HEADPHONE_CALIBRATION.md` | **KEEP / LATER** | Valuable calibration architecture, not first-listening prerequisite. |
| `docs/MUSIC_PRESENTATION_CONTRACT.md` | **KEEP / LATER** | Optional adaptive-presentation constraints, subordinate to baseline product. |
| `docs/REALTIME_CONTROL_CONTRACT.md` | **KEEP** | Sample-time/control ownership. |
| `omniphony-studio/` | **REMOVED** | Upstream control/visualization product is not this fork's listener shell. |
| `packaging/` | **REMOVED** | Upstream suite packaging did not serve the current Windows fork. |
| root `scripts/` | **REMOVED** | JACK/service helper surface belonged to the upstream suite. |
| `docs/superpowers/` | **REMOVED** | Historical implementation plans, not current contracts. |
| mpv product docs | **REMOVED** | mpv distribution is not the normal ingestion architecture. |
| old Studio/WebGL/Three.js docs | **REMOVED** | Deleted product/debug surface. |
| old PipeWire product plans | **REMOVED** | Historical host direction, not a reason to preserve old ingest topology. |
| old refactor diaries / duplicate translations | **REMOVED** | Git history already preserves them. |

---

# 2. Workflows

| Workflow | Status | Reason |
| --- | --- | --- |
| old `.github/workflows/ci.yml` | **REMOVED** | Built deleted Studio/inherited suite surfaces. |
| old `release.yml` | **REMOVED** | Upstream Studio release product. |
| old `integration-build.yml` | **REMOVED** | Rolling Studio integration release. |
| old `liborender-release.yml` | **REMOVED** | Separate library-release surface not currently shipped. |
| `.github/workflows/windows-renderer.yml` | **KEEP** | Current clean Windows/core build-and-test oracle. |

Current workflow lanes remain useful because they separate renderer truth from specialist host dependencies:

```text
portable/headless renderer core
Windows renderer core
Windows x64 headless renderer-engine artifact
```

The post-`73488c25` Windows Actions run was visually verified green by the repository owner on 2026-08-10. Treat the old host-path failure as closed unless a new run regresses.

The headless artifact deliberately avoids making the Steinberg ASIO SDK a requirement for validating the renderer core.

Future workflow work should first support the native Windows listening lane. Do not add macOS/Linux/mobile CI merely to satisfy an abstract portability plan.

---

# 3. Current dependency shape

High-level ownership:

```text
                       RENDERER / LAB CORE

renderer
   ↑
orender_engine ───────── runtime_control / sys
   ↑
orender_ffi

bridge_api ← reference_bridge
             known-scene / file-render instrument

                       WINDOWS HOST FRONTIER

host_audio
 ├→ audio_input
 └→ audio_output

legacy inherited transport:
audio_input → PipeWire bridge → SPDIF / IEC61937
```

Crate names are historical. Ownership follows current product behavior, not names.

---

# 4. `renderer`

**KEEP.**

This is the central perceptual/product substrate.

Retain:

- binaural HRTF/ITD behavior;
- stereo evidence currently in use;
- object/scene state;
- early reflections;
- late room-field machinery;
- measured/parametric/SOFA HRTF support;
- moving-filter continuity;
- known-layout geometry useful for validation;
- object size/extent state until explicitly proven unnecessary.

Do not rewrite this crate for architectural aesthetics.

Potential defect/experiment lanes include:

```text
sample-time position/HRTF motion
source extent in headphones
BroadSource behavior
DiffuseField behavior
directional early-reflection consistency
bass/foundation preservation
```

These are candidates, not a mandatory prerequisite queue. The root README decides priority from actual listening/product needs.

---

# 5. `dsp_fixtures`

**KEEP.**

Independent measurement and deterministic regression layer.

Use it for:

- null/fidelity checks;
- callback-size invariance where applicable;
- known HRTF/scene truth;
- cross-path comparison;
- future host-neutral checks around the same engine.

A product that cannot attribute a regression is harder to improve safely.

---

# 6. `audio_output`

**WINDOWS HOST / TRANSITIONAL.**

Realtime output is required. The inherited implementation mixes several host assumptions.

Current important problem:

```text
Windows output is effectively tied to CPAL/ASIO in inherited code
```

Target:

```text
same Omniphony engine
        ↓
normal Windows output route
and/or
optional ASIO route
```

ASIO remains useful for the current FiiO setup and may remain a permanent specialist option.

It must not be the only normal Windows product route.

Do not move output-device semantics into `renderer` or `orender_engine` while fixing this.

---

# 7. `audio_input`

**REPLACE THEN CUT / WINDOWS HOST FRONTIER.**

Despite the generic name, the inherited public contract is heavily shaped by older transport assumptions:

```text
Bridge / PipeWire modes
PipeWire / ASIO enum concepts
fixed multichannel mapping state
IEC61937 bridge clocks/pacing
```

The next product need is much simpler:

```text
ordinary Windows PCM
→ timestamp/sample position
→ discontinuity/reset state
→ Omniphony engine
```

Do not add Windows product support by extending every legacy PipeWire/bridge enum forever.

First establish a small neutral-enough PCM/time/reset boundary, then implement the Windows adapter behind it.

The first Windows capture route may be loopback, player-specific, virtual-endpoint-based, or another practical host strategy. Choose from real product testing rather than from a desire to preserve the old input abstraction.

---

# 8. `host_audio`

**KEEP AS THE HOST/ENGINE BOUNDARY.**

This crate is conceptually in the right location: device/input/output work sits above the headless engine.

Desired evolution:

```text
current inherited input/output surfaces
        ↓
small host contract
        ↓
Windows implementation(s)
```

The first job is not a universal host API. The first job is a clean enough Windows lane that leaves renderer semantics untouched.

---

# 9. `orender_engine`

**KEEP.**

The headless engine is strategically valuable because it lets us:

- validate renderer behavior without device plumbing;
- keep CI independent of specialist audio SDKs;
- compare Windows host routes behind one engine;
- maintain deterministic offline/reference paths.

Generic backend compatibility may shrink as deleted consumers disappear, but the no-audio-I/O engine boundary should remain.

---

# 10. `orender_ffi`

**KEEP WHILE USEFUL / NARROW.**

Useful for:

- native host integration;
- headless validation;
- possible future external shells.

It is not a separate product for its own sake.

Retain only the ABI surface that serves current engine/host boundaries.

---

# 11. `reference_bridge`

**LAB / REFERENCE. KEEP.**

Known-channel / known-scene file input isolates renderer quality from:

- stereo inference;
- Windows capture;
- device timing.

This is exactly the kind of non-consumer machinery that must survive because it protects attribution.

Do not confuse "not normal listener ingestion" with "not valuable."

---

# 12. `bridge_api`

**TRANSITIONAL LAB/RUNTIME BOUNDARY.**

Still useful for `reference_bridge` and inherited engine paths.

Separate the valuable concept:

```text
known/rich source information
→ typed engine input
```

from historical generic plugin/decoder product assumptions.

Contract gradually. Do not delete the reference path merely because normal stereo does not need generic decoder plugins.

---

# 13. `spdif`

**REPLACE THEN CUT.**

This is an IEC61937/S/PDIF parser whose current meaningful owner is the inherited Linux/PipeWire bridge path.

Ordinary Windows PCM music does not require it.

Remove after:

- the first normal PCM input boundary exists;
- callers are migrated;
- any deliberately retained encoded-input test has a clear separate owner.

Do not preserve the old architecture on the theory that Linux may be supported someday.

---

# 14. `runtime_control`

**KEEP / TRANSITIONAL.**

Timed state/control is useful.

Preserve the realtime law:

```text
slow/control/model work
→ build/validate away from audio thread
→ timestamp / generation tag
→ publish bounded immutable state
→ audio thread follows sample-time behavior
```

Studio-shaped state, OSC assumptions and generic backend compatibility can shrink as actual consumers disappear.

---

# 15. `sys`

**KEEP SMALL.**

Own genuinely platform-specific lifecycle/IO support shared by host code.

Do not let it become a dumping ground for scene, presentation, or product policy.

Windows host work may legitimately grow this crate where the abstraction protects more than one host component.

---

# 16. Removed demo/script backends

## `example_backend`

**REMOVED.**

It existed to demonstrate arbitrary backend extensibility, not the headphone product.

## `script_backend`

**REMOVED.**

The Lua/user-programmable backend had no current product owner and pulled an unnecessary scripting runtime into the workspace.

Do not restore either without a new concrete need.

---

# 17. Assets and known layouts

## Binaural baseline assets

**KEEP. PROTECTED.**

Especially:

```text
upstream-demo-reference.yaml
baseline-room.yaml
dry-binaural.yaml
```

The upstream-demo reference is the perceptual ancestor.

The fork room preset is a comparison, not a replacement floor.

## Known speaker layouts

**LAB / REFERENCE. KEEP.**

They provide controlled geometry:

```text
known source direction
→ renderer
→ objective / listening validation
```

They do not imply that ordinary stereo should permanently pass through fake 7.1/7.1.4 transport.

## Demo/reference audio

**KEEP WHEN LICENSE-CLEAR AND DETERMINISTIC.**

A demo file can be valuable laboratory truth even if it has no product UX role.

---

# 18. Ordered deeper cuts

Do not combine many ownership changes into one giant deletion commit.

## Cut A · Generic backend product residue

Determine what remaining dynamic backend/config machinery is still needed for:

- renderer operation;
- reference bridge;
- FFI/headless engine.

Remove only unowned contributor/plugin UX.

## Cut B · Normal PCM host boundary

Before deleting legacy input code:

```text
define PCM / time / reset contract
→ add deterministic test source
→ add Windows host adapter
→ migrate host_audio callers
```

## Cut C · Legacy PipeWire / IEC61937 ingest

After Cut B callers migrate, remove the inherited chain that no longer serves the Windows product:

- encoded passthrough ingest;
- S/PDIF parser;
- PipeWire-bridge-specific mode/control state;
- fixed-layout assumptions not retained explicitly as lab fixtures.

## Cut D · Generic bridge/plugin residue

Separate the known-scene fixture interface from generic runtime plugin assumptions.

Then remove generic machinery with no product/test owner.

## Cut E · Other platforms

**DEFERRED.**

There is no current cut/build sequence for macOS, Linux, Android or iOS.

If the Windows product later earns a port, design that port from the then-current engine boundary and current platform APIs rather than restoring old historical host plans wholesale.

---

# 19. What must not be accidentally deleted

Protect:

- upstream-demo perceptual control;
- measured/parametric/SOFA HRTF support;
- HRTF interpolation and motion continuity;
- analytic ITD and direct-arrival timing contract;
- early directional room cues;
- late room/externalization machinery until experiments prove a better boundary;
- object extent/size state until explicitly evaluated;
- deterministic fixture generation;
- fidelity measurements;
- callback-size regression tests/reproducers;
- known-scene geometry;
- headless engine boundary;
- host/core separation;
- ASIO route while it remains useful for the real K7 setup;
- any artifact needed to reproduce an audible regression.

---

# 20. Acceptance rule for each contraction commit

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

A prettier tree that makes it harder to explain or reproduce the sound is negative progress.

The current contraction frontier is not "delete more." It is **make the Windows host boundary simple enough to build the first coexisting native listening lane without disturbing the renderer underneath it.**