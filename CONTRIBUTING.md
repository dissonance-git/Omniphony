# Omniphony development rules

This is a private development repository.

Treat this file as working guidance for ChatGPT, Codex, local tooling, or anyone making changes later. It is not written as public contributor onboarding.

Read the root `README.md` first. The README owns project intent, current priority, baseline hierarchy and roadmap.

Read `docs/FORK_POLICY.md` before broad upstream/refactor work.

---

## 1. Product invariant

The current job is:

> **Turn the already-good upstream Omniphony renderer into a native Windows daily-listening replacement for the current HeSuVi pipeline, while preserving the sound that made the fork worth building.**

Ordinary music is the primary target.

Spatial spectacle does not outrank fidelity.

A new mechanism that makes the protected baseline sound worse is a regression even if the implementation is more sophisticated.

---

## 2. Priority order

Unless the root README is explicitly changed, work in this order:

```text
1. preserve / reproduce the upstream Omniphony perceptual floor
2. build the coexisting native Windows listening lane
3. make A/B against the real incumbent easy
4. fix renderer weaknesses exposed by actual listening/tests
5. improve ordinary-stereo scene behavior
6. add adaptive/music-aware presentation only when useful
7. use libaural as optional richer evidence when it earns itself
8. consider other platforms only after Windows is worth porting
```

Do not reverse this ordering because a research direction is interesting.

---

## 3. Current real incumbent

Development decisions should remember what the product must eventually replace:

```text
foobar DSP
→ 5.1-side upmix
→ VB-Audio multichannel / ASIO Bridge
→ HeSuVi / DTS Virtual:X
→ FiiO ASIO
→ FiiO K7
→ Dan Clark Noire X
```

The current chain remains usable during development.

No cold-turkey migration.

The incumbent is evidence about useful perception, not an implementation template to clone stage-for-stage.

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

Do not overwrite the control to make an experiment look better.

Use separate config/flags for experimental DSP.

---

## 5. Highest-value current work

Current W1 work is Windows transport around the existing renderer.

The repository already contains:

```text
windows_host
→ WASAPI-first output-device probe
→ self-excluding process-loopback diagnostic probe

realtime_ffi
→ small interleaved-f32 PCM C ABI
→ first implementation is bit-exact identity
→ CI/package coverage
```

The loopback route is diagnostic only because it copies rather than intercepts the system mix.

Current next-value areas:

- stable normal Windows output through the identity seam;
- connect the protected renderer behind `realtime_ffi`;
- prove native output matches controlled/offline renderer behavior;
- prototype a true single-path route, especially endpoint APO versus minimal virtual endpoint;
- preserve ASIO as a useful specialist/current-hardware route;
- build fast incumbent ↔ Omniphony A/B.

Renderer/scene work should move ahead of this only for a concrete regression or blocker.

---

## 6. Current repository shape

Important workspace surfaces include:

```text
omniphony-renderer/
  renderer/           binaural/spatial DSP + current stereo evidence
  dsp_fixtures/       deterministic measurements and regressions
  windows_host/       thin Windows-native transport probe/frontier
  realtime_ffi/       narrow PCM ABI for native host integration
  host_audio/         host/engine integration boundary
  audio_output/       inherited output/timing infrastructure, transitional
  audio_input/        inherited input/transport infrastructure, transitional
  orender_engine/     headless engine boundary
  orender_ffi/        existing embedding boundary
  reference_bridge/   deterministic known-scene/file laboratory input
  bridge_api/         retained reference/runtime seam, transitional
  runtime_control/    timed state/control infrastructure
  sys/                platform/lifecycle support
  spdif/              legacy encoded transport, replace-then-cut

layouts/              known-scene calibration geometry
docs/                 technical contracts subordinate to README
.github/workflows/     current validation / Windows artifacts
```

Removed surfaces such as Studio, `example_backend`, `script_backend`, old suite packaging and obsolete release workflows are historical. Do not document them as if they still exist.

See `docs/CONTRACTION_LEDGER.md` for ownership/status.

---

## 7. Build/test truth

The renderer workspace declares Rust `1.88.0` as its current minimum.

Do not assume a checked-in `Cargo.lock` exists for the workspace. Follow the current CI/workspace behavior rather than copying stale `--locked` commands from old docs.

At minimum, changes should run the relevant formatter/compiler/tests for the surface changed.

Important workflow:

```text
.github/workflows/windows-renderer.yml
```

The August 2026 host-native backend-path repair has been visually verified green by the repository owner.

New CI failures are evidence. Do not make CI green by weakening a perceptual/fidelity gate without understanding the failure.

---

## 8. Commit law

Work directly on `main` unless the user explicitly asks for a branch/PR workflow.

Prefer bounded commits that leave a durable re-entry trail.

A useful sequence is:

```text
one exact question
→ smallest coherent change
→ test/inspect
→ commit
→ next question
```

This matters because the GitHub connector/chat may fail or compact. Saved commits are the durable work surface.

Avoid giant refactors that simultaneously alter:

- renderer sound;
- host transport;
- scene inference;
- calibration;
- repository structure.

When something breaks, the diff should make the responsible layer obvious.

---

## 9. Audible DSP changes

Every audible change must answer two questions independently:

```text
What intended perceptual behavior improved?

What did the change cost in fidelity or musical identity?
```

Useful objective checks include:

- strict null/residual where identity is expected;
- peak/RMS;
- crest factor;
- DC;
- frequency response;
- interaural lag/ITD;
- transient timing;
- bass timing/coherence;
- callback-size invariance where applicable;
- clipping/headroom;
- state-switch continuity.

Human listening remains required for:

- externalization;
- front/back discrimination;
- elevation;
- side precision;
- radial depth;
- source extent;
- image stability;
- listener envelopment;
- room naturalness;
- direct-source solidity;
- bass/groove integrity;
- timbre;
- fatigue;
- preference.

At matched loudness, bypass should feel flatter, not cleaner.

---

## 10. Golden/reference changes

Do not bless unexplained drift.

If a deterministic golden/reference must intentionally change:

```text
identify reason
→ measure difference
→ verify perceptual/product intent
→ keep old control where needed for A/B
→ only then update golden
```

The upstream-demo-style perceptual control has stronger protection than an ordinary implementation golden.

---

## 11. Realtime law

The audio path should be:

- bounded;
- nonblocking where practical;
- allocation-free after initialization where practical;
- explicit about resets/discontinuities;
- deterministic for equivalent continuous input/state;
- independent of arbitrary callback partitioning for semantic behavior;
- measured rather than assumed fast.

Keep off the realtime callback:

- filesystem access;
- network access;
- ordinary model inference;
- large allocations;
- SOFA parsing/import;
- device enumeration;
- UI calls;
- blocking logs/mutex waits.

A normal Windows host route and ASIO route must not develop different renderer semantics.

---

## 12. Scene semantics

Keep these distinct:

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

Ordinary stereo does not reveal literal rear object metadata.

Rear placement may be a valid presentation decision when evidence and listening justify it.

Do not let scene-model completeness become a prerequisite for the protected renderer or W1 Windows product.

---

## 13. libaural boundary

`libaural` is a separate research/framework project and a possible future evidence provider.

It is not the product owner and is not mandatory for Omniphony to run.

Use it when a bounded capability demonstrably improves a presentation decision.

Do not:

```text
research result
→ architecture rewrite
```

Prefer:

```text
specific audible/product weakness
→ candidate libaural evidence/mechanism
→ isolated experiment
→ A/B
→ keep only if earned
```

---

## 14. Upstream Omniphony

Treat `mgth/Omniphony` as technical ancestor, perceptual foundation and continuing source of mechanisms/fixes.

Use:

```text
inspect exact upstream change
→ check whether already present
→ take smallest relevant missing part
→ validate locally
```

Do not merge broad upstream product work merely to keep history aligned.

Do not delete useful inherited renderer behavior merely because the final UX is narrower.

When this fork proves a general fix, isolate the portable/general part before considering an upstream contribution.

---

## 15. External research

External projects are mechanism sources, benchmarks and experiment inputs, not dependency wish lists.

Research should begin from a concrete question.

For a new influence, record mentally or durably as appropriate:

1. exact mechanism that may help;
2. exact weakness/question it addresses;
3. smallest experiment that could falsify it;
4. whether it belongs to renderer, Windows host, optional adaptive policy, calibration or test tooling;
5. licensing/data implications if incorporated.

Do not restart broad GitHub/literature mining without a missing capability to search for.

---

## 16. Contraction law

Deletion is valid when it removes unowned inherited surface without losing:

- current Windows capability;
- protected sound;
- renderer behavior;
- regression observability;
- known-scene truth;
- calibration truth;
- an active experimental control.

Delete in dependency-safe order and test between cuts.

Git/upstream is the archive. The working tree does not need to impersonate one.

---

## 17. Final working rule

When uncertain what to do next, use this loop:

```text
make the native Windows path easier to hear
→ compare it against the protected Omniphony control
→ compare it against the real incumbent
→ identify the next actual weakness
→ research/fix only that weakness
→ preserve everything that still wins
```

That is the development process.