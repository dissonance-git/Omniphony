# Omniphony development rules

This is a private development repository.

Treat this file as working guidance for ChatGPT, Codex, local tooling, or anyone making changes later. It is not public contributor onboarding.

Read the root `README.md` first. The README owns product intent, architecture, current priority, baseline hierarchy and roadmap.

For ownership/deletion boundaries, read `docs/contraction-ledger.md`.

---

## 1. Product invariant

The job is:

> **Turn the already-good upstream Omniphony renderer into a platform-agnostic universal headphone spatial processor, prove it on Windows first, and make ordinary stereo music feel radically more dimensional without making the recording feel remixed.**

Windows is the first host, not the core architecture.

Ordinary stereo music is the main use case.

Native surround/object audio is richer source truth when available.

A new mechanism that makes the protected baseline sound worse is a regression even if the implementation is more sophisticated.

---

## 2. Priority order

Unless the README is explicitly changed:

```text
1. preserve/reproduce the upstream Omniphony perceptual floor
2. keep the portable core free of platform-specific assumptions
3. prove one physical Windows audio path
4. make ON/OFF route-clean
5. establish a fair ordinary-stereo baseline
6. compare against the real incumbent
7. preserve native surround/rich source truth
8. prove stereo + surround coexistence
9. improve stereo presentation only from clean listening evidence
10. use libaural/Helix mechanisms only when they earn themselves
11. replace temporary Windows scaffolding with an owned native route
12. port hosts to other platforms only after the product is worth porting
```

Do not reverse this order because a research direction is interesting.

---

## 3. Core / host boundary

Portable core owns:

```text
logical input streams
per-stream channel/spatial layout
presentation state
scene state
binaural rendering
stereo output
```

Platform hosts own:

```text
device/session discovery
capture/interception
endpoint ownership
platform format translation
clock/recovery behavior
platform UI/service integration
```

Do not leak:

```text
WASAPI
ASIO
VB-Audio
Windows device names
Windows sessions
```

into portable renderer semantics.

---

## 4. Concurrent stream law

Channel layout is **stream-local**, never global.

Valid simultaneous state:

```text
stereo music
+
7.1 game
+
mono/stereo voice
→ one Omniphony world
→ binaural stereo output
```

Starting a surround application must not reinterpret an unrelated playing stereo stream.

A platform prototype may temporarily receive an already-mixed bed. Do not promote that limitation into the core model.

---

## 5. Current incumbent and migration law

Current reference chain:

```text
foobar DSP
→ 5.1-side upmix
→ VB-Audio / Hi-Fi Cable
→ HeSuVi / DTS Virtual:X
→ ASIO Bridge / FiiO ASIO
→ FiiO
→ Dan Clark Noire X
```

Do not require uninstalling this chain during migration.

Use:

```text
keep installed
→ disable one stage
→ replace it with Omniphony
→ verify
→ remove only after obsolete
```

For a trustworthy Omniphony listen, old forwarding must not simultaneously reach the FiiO.

---

## 6. First live result and current frontier

The native Windows app now plays arbitrary foobar/Windows audio through Omniphony to the physical headphones.

That proves the live transport path exists.

The first arbitrary-audio listen was reported as tinny, hallway-like and less bubble-like, with small echo after OFF.

Do **not** tune the renderer around that result yet.

Only HeSuVi had been disabled; the old forwarding chain remained configured. A duplicate delayed path is a strong current hypothesis, and the prototype also had queued-wet bypass leakage.

Current frontier:

```text
1. old ASIO/physical forwarding disabled, not uninstalled
2. Omniphony is the only path to FiiO
3. ON/OFF destroys stale queues in the prototype
4. clean stereo test
5. only then judge timbre / hallway / bubble / externalization
6. native surround test
7. stereo + surround simultaneous test
```

Detailed route state lives in `docs/windows-audio-route.md`.

---

## 7. Prototype app structure

Current product skeleton:

```text
Omniphony.exe
→ hidden omniphony_worker.exe
→ Windows host plumbing
→ protected Omniphony renderer
→ FiiO
```

This GUI/worker/core ownership split is worth keeping.

The temporary Hi-Fi Cable / process-loopback route may be replaced without replacing the product shell or portable renderer model.

---

## 8. Bypass law

OFF is a real routing state.

It must not leave:

```text
queued wet audio
stale room-selected output
secondary physical forwarding
duplicate dry paths
renderer leakage
```

The current prototype may accept a brief restart gap to guarantee old queues are destroyed.

A later polished implementation should use sample-aligned wet/dry state near physical output.

A clean comparison is more valuable than an instant ambiguous switch.

---

## 9. Protected renderer reference

Keep stable:

```text
omniphony-renderer/assets/binaural-baselines/upstream-demo-reference.yaml
```

It approximates the published upstream demo contract:

```text
stock-style Omniphony
+ SAF/KEMAR
+ early reflections
+ no fork-added late reverb
```

Do not overwrite the control to make an experiment look better.

Use separate configs/flags for experimental DSP.

---

## 10. Repository shape

Important surfaces:

```text
omniphony-renderer/
  renderer/           portable binaural/spatial DSP + stereo evidence
  dsp_fixtures/       deterministic measurements/regressions
  windows_host/       first platform-host/product shell frontier
  realtime_ffi/       narrow PCM ABI
  host_audio/         host/engine integration boundary
  audio_output/       inherited output/timing infrastructure, transitional
  audio_input/        inherited input/transport infrastructure, transitional
  orender_engine/     headless engine boundary
  orender_ffi/        embedding boundary
  reference_bridge/   deterministic known-scene/file laboratory input
  bridge_api/         retained reference/runtime seam, transitional
  runtime_control/    timed state/control infrastructure
  sys/                platform/lifecycle support
  spdif/              legacy encoded transport, replace-then-cut

layouts/              known-scene reference geometry
docs/                 contracts subordinate to README
.github/workflows/     validation / Windows artifacts
```

Do not document removed inherited surfaces as if they still exist.

---

## 11. Build/test truth

Renderer workspace minimum Rust:

```text
1.88.0
```

Important workflow:

```text
.github/workflows/windows-renderer.yml
```

CI failures are evidence.

Do not make CI green by weakening a perceptual/fidelity gate without understanding the failure.

---

## 12. Commit law

Work directly on `main` unless the user explicitly asks for branches/PRs.

Prefer bounded commits:

```text
one exact question
→ smallest coherent change
→ test/inspect
→ commit
→ next question
```

Avoid giant refactors that simultaneously alter renderer sound, host transport, stereo inference, calibration and repository structure.

---

## 13. Audible DSP changes

Every audible change must answer:

```text
What intended percept improved?
What did it cost in fidelity or musical identity?
```

Useful checks include:

- null/residual where identity is expected;
- peak/RMS;
- crest factor;
- DC;
- frequency response;
- interaural lag/ITD;
- transient timing;
- bass timing/coherence;
- callback invariance where applicable;
- clipping/headroom;
- state-switch continuity;
- bypass queue cleanliness.

Human listening remains required for:

- externalization;
- front/back discrimination;
- elevation/below;
- radial depth;
- source extent;
- image stability;
- envelopment;
- room naturalness;
- direct-source solidity;
- bass/groove integrity;
- timbre;
- fatigue;
- preference.

At matched loudness, bypass should feel flatter, not cleaner.

Do not score a candidate while two physical routes are audible.

---

## 14. Realtime law

The audio path should be bounded, nonblocking where practical, explicit about resets/discontinuities, deterministic for equivalent continuous input/state, and independent of arbitrary callback partitioning for semantic behavior.

Keep off realtime callbacks:

- filesystem/network access;
- ordinary model inference;
- large allocations;
- SOFA parsing/import;
- device/session enumeration;
- UI calls;
- blocking logs/mutex waits.

The same semantic core should survive different platform hosts.

See `docs/realtime-control-contract.md`.

---

## 15. Scene/source semantics

Keep distinct:

```text
source truth
≠ signal evidence
≠ presentation hypothesis
≠ placement choice
```

and:

```text
DirectObject
≠ BroadSource
≠ DiffuseField
≠ RoomField
```

Ordinary stereo does not reveal literal rear-object metadata.

Real surround/object metadata should not be thrown away and re-inferred.

See `docs/scene-renderer-contract.md`.

---

## 16. Music law

Ordinary stereo music is the main use case.

Desired result:

> **The song sounds as though it had already been mixed/mastered for this immersive headphone presentation before playback began.**

Not live remixing.
Not fader-riding behavior.
Not classifier-driven source wandering.

See `docs/music-presentation-contract.md`.

---

## 17. libaural boundary

`libaural` is separate research/framework infrastructure.

It is not the product owner and is not mandatory for playback.

Prefer:

```text
specific audible weakness
→ candidate hearing mechanism
→ isolated experiment
→ clean A/B
→ keep only if earned
```

Never use a research result as automatic permission for an architecture rewrite.

---

## 18. Upstream Omniphony

Treat `mgth/Omniphony` as technical ancestor, perceptual foundation and continuing mechanism source.

```text
inspect exact upstream change
→ check whether already present
→ take smallest relevant missing part
→ validate locally
```

Do not merge broad upstream product work merely to keep history aligned.

---

## 19. External research

External projects are mechanism sources, benchmarks and experiment inputs, not dependency wish lists.

Useful findings must be parked in `docs/influence-ledger.md` or the appropriate focused research document.

For a new influence, record:

1. exact mechanism;
2. exact weakness/question;
3. smallest falsifying experiment;
4. ownership layer;
5. licensing/data implications.

---

## 20. Final working rule

When uncertain what to do next:

```text
make the physical route clean
→ test ordinary stereo music
→ compare against protected Omniphony + real incumbent
→ identify actual weakness
→ research/fix only that weakness
→ preserve everything that still wins
```

That is the development process.
