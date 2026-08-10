# Omniphony development rules

This is a private development repository.

Treat this file as working guidance for ChatGPT, Codex, local tooling, or anyone making changes later. It is not public contributor onboarding.

Read the root `README.md` first. The README owns product intent, current priority, baseline hierarchy, and roadmap.

For current ownership and safe deletion boundaries, read `docs/contraction-ledger.md`.

---

## 1. Product invariant

The current job is:

> **Turn the already-good upstream Omniphony renderer into a native Windows daily-listening replacement for the current HeSuVi pipeline, while preserving the sound that made the fork worth building.**

Ordinary music is the primary target. Spatial spectacle does not outrank fidelity.

A new mechanism that makes the protected baseline sound worse is a regression even if the implementation is more sophisticated.

---

## 2. Priority order

Unless the root README is explicitly changed:

```text
1. preserve / reproduce the upstream Omniphony perceptual floor
2. prove and harden the native Windows listening lane
3. make A/B against the real incumbent easy
4. add the simplest useful ordinary-stereo path
5. fix renderer weaknesses exposed by actual listening/tests
6. improve persistent stereo scene behavior
7. add adaptive/music-aware presentation only when useful
8. use libaural as optional richer evidence when it earns itself
9. consider other platforms only after Windows is worth porting
```

Do not reverse this order because a research direction is interesting.

---

## 3. Current real incumbent

```text
foobar DSP
→ 5.1-side upmix
→ VB-Audio multichannel / ASIO Bridge
→ HeSuVi / DTS Virtual:X
→ FiiO ASIO
→ FiiO K7
→ Dan Clark Noire X
```

The current chain remains usable during development. No cold-turkey migration.

ASIO is valuable here because it serves the current Hi-Fi Cable/HeSuVi route. It remains a useful specialist path, but the ordinary Windows product must not require specialist ASIO setup.

---

## 4. Protected renderer reference

Keep this control stable:

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

Do not overwrite the control to make an experiment look better. Use separate configs/flags for experimental DSP.

---

## 5. Current Windows frontier

The repository now contains a compiled internal P0 listening prototype.

```text
windows_host
→ WASAPI output-device discovery
→ self-excluding process-loopback diagnostic
→ --smoke-output
→ --reference-demo
→ --render-reference-only

realtime_ffi
→ small interleaved-f32 PCM C ABI
→ current implementation is bit-exact identity
```

The P0 Actions run completed successfully on 2026-08-10. The only reported warning was a cosmetic `unused_mut` in a test-local closure in `orender_engine/src/object_gen.rs`.

Loopback remains diagnostic because it copies rather than intercepts the system mix.

Current next-value areas:

- physically test the P0 WASAPI smoke/reference paths;
- move protected rendering behind the persistent realtime host seam;
- prove realtime/native output matches controlled reference semantics;
- add the simplest ordinary-stereo music path without experimental DSP;
- establish fast incumbent ↔ Omniphony A/B;
- prototype a true single-path system route;
- preserve ASIO as a useful specialist/reference route.

Detailed route decisions live in `docs/windows-audio-route.md` and parked Windows API research in `docs/windows-integration-research.md`.

---

## 6. Repository shape

Important surfaces:

```text
omniphony-renderer/
  renderer/           binaural/spatial DSP + current stereo evidence
  dsp_fixtures/       deterministic measurements and regressions
  windows_host/       Windows-native product/transport frontier
  realtime_ffi/       narrow PCM ABI for native host integration
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
docs/                 technical contracts subordinate to README
.github/workflows/     validation / Windows artifacts
```

Removed surfaces such as Studio, old script/example backends, old suite packaging/release machinery, and obsolete host/product docs are historical. Do not document them as if they still exist.

---

## 7. Build/test truth

The renderer workspace currently declares Rust `1.88.0` as its minimum.

Do not assume a checked-in `Cargo.lock` exists. Follow current CI/workspace behavior rather than stale commands.

Important workflow:

```text
.github/workflows/windows-renderer.yml
```

New CI failures are evidence. Do not make CI green by weakening a perceptual/fidelity gate without understanding the failure.

---

## 8. Commit law

Work directly on `main` unless the user explicitly asks for a branch/PR workflow.

Prefer bounded commits that leave a durable re-entry trail:

```text
one exact question
→ smallest coherent change
→ test/inspect
→ commit
→ next question
```

Avoid giant refactors that simultaneously alter renderer sound, host transport, scene inference, calibration, and repository structure.

Repository-only formatting/rename cleanup may be batched when it is deliberately non-audible and cross-links are updated atomically.

---

## 9. Audible DSP changes

Every audible change must answer independently:

```text
What intended perceptual behavior improved?

What did the change cost in fidelity or musical identity?
```

Useful objective checks include strict null/residual where identity is expected, peak/RMS, crest factor, DC, frequency response, interaural lag/ITD, transient timing, bass timing/coherence, callback-size invariance where applicable, clipping/headroom, and state-switch continuity.

Human listening remains required for externalization, front/back discrimination, elevation, side precision, radial depth, source extent, image stability, listener envelopment, room naturalness, direct-source solidity, bass/groove integrity, timbre, fatigue, and preference.

At matched loudness, bypass should feel flatter, not cleaner.

---

## 10. Golden/reference changes

Do not bless unexplained drift.

```text
identify reason
→ measure difference
→ verify perceptual/product intent
→ keep old control where needed for A/B
→ only then update golden
```

The upstream-demo perceptual control has stronger protection than an ordinary implementation golden.

---

## 11. Realtime law

The audio path should be bounded, nonblocking where practical, allocation-free after initialization where practical, explicit about resets/discontinuities, deterministic for equivalent continuous input/state, independent of arbitrary callback partitioning for semantic behavior, and measured rather than assumed fast.

Keep off the realtime callback:

- filesystem/network access;
- ordinary model inference;
- large allocations;
- SOFA parsing/import;
- device enumeration;
- UI calls;
- blocking logs/mutex waits.

A normal Windows host route and ASIO route must not develop different renderer semantics.

See `docs/realtime-control-contract.md`.

---

## 12. Scene semantics

Keep distinct:

```text
signal evidence
≠ auditory/presentation entity
≠ scene hypothesis
≠ placement choice
```

and:

```text
DirectObject
≠ BroadSource
≠ DiffuseField
≠ RoomField
```

Ordinary stereo does not reveal literal rear-object metadata. Rear placement may be a valid presentation choice when evidence and listening justify it.

See `docs/scene-renderer-contract.md`.

---

## 13. libaural boundary

`libaural` is a separate research/framework project and a possible future evidence provider.

It is not the product owner and is not mandatory for Omniphony to run.

Prefer:

```text
specific audible/product weakness
→ candidate libaural evidence/mechanism
→ isolated experiment
→ A/B
→ keep only if earned
```

Never use a research result as automatic permission for an architecture rewrite.

---

## 14. Upstream Omniphony

Treat `mgth/Omniphony` as technical ancestor, perceptual foundation, and continuing source of mechanisms/fixes.

```text
inspect exact upstream change
→ check whether already present
→ take smallest relevant missing part
→ validate locally
```

Do not merge broad upstream product work merely to keep history aligned.

When this fork proves a general fix, isolate the portable/general part before considering an upstream contribution.

---

## 15. External research

External projects are mechanism sources, benchmarks, and experiment inputs, not dependency wish lists.

Useful findings must be parked in `docs/influence-ledger.md` or the appropriate focused research room so GitHub dives remain cumulative across context loss.

For a new influence, record:

1. exact mechanism that may help;
2. exact weakness/question it addresses;
3. smallest experiment that could falsify it;
4. whether it belongs to renderer, Windows host, adaptive policy, calibration, or test tooling;
5. licensing/data implications if incorporated.

Do not restart broad mining without a missing capability to search for.

---

## 16. Contraction law

Deletion is valid when it removes unowned inherited surface without losing current Windows capability, protected sound, renderer behavior, regression observability, known-scene truth, calibration truth, or an active experimental control.

Git/upstream is the archive. The working tree does not need to impersonate one.

See `docs/contraction-ledger.md`.

---

## 17. Final working rule

When uncertain what to do next:

```text
make the native Windows path easier to hear
→ compare it against the protected Omniphony control
→ compare it against the real incumbent
→ identify the next actual weakness
→ research/fix only that weakness
→ preserve everything that still wins
```

That is the development process.
