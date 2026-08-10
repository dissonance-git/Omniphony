# Omniphony fork policy

This repository is an independent development fork of `mgth/Omniphony`.

The original project remains the permanent ancestry/attribution source and an important source of renderer mechanisms and fixes. It does not define this fork's roadmap.

The root `README.md` is the canonical product plan.

The key fork rule is:

> **Independence from upstream product structure does not mean independence from the upstream sound that made the fork worth building.**

```text
mgth/Omniphony
technical ancestor + perceptual foundation
        │
        │ selective fixes / mechanisms
        ▼
dissonance-git/Omniphony
native Windows headphone product
        │
        │ proven portable/general fixes
        ▼
possible upstream contribution
```

---

## 1. Direction of travel

The current product goal is narrow:

> ordinary Windows audio should reach headphones through Omniphony with a stable, externalized, convincing 360° presentation that preserves or improves the already-good upstream character and eventually replaces the listener's HeSuVi-based chain.

The fork is not obligated to preserve upstream Studio, packaging, speaker-authoring or distribution surfaces.

It **is** obligated to preserve useful renderer behavior until a replacement proves better.

---

## 2. Upstream perceptual ancestry

The hosted upstream headphone demo is a perceptual ancestor for this fork.

The local approximation lives at:

```text
omniphony-renderer/assets/binaural-baselines/upstream-demo-reference.yaml
```

Do not interpret "independent fork" as permission to silently redefine the sound floor.

A fork-specific renderer change graduates only after controlled comparison against the protected reference and relevant real listening.

---

## 3. Pulling from upstream

Use this flow:

```text
inspect upstream change
→ identify exact mechanism
→ check whether it is already present here
→ ask whether it serves this Windows headphone product
→ import only the smallest useful missing part
→ validate locally
```

High-value upstream candidates include:

- binaural/HRTF correctness fixes;
- realtime-safety fixes;
- renderer math fixes;
- deterministic DSP-test improvements;
- Windows audio fixes relevant to the native route;
- performance improvements that preserve semantics;
- scene/geometry machinery useful for calibration or controlled rendering;
- optional spatial mechanisms that can be isolated and A/B tested.

Low-priority current surfaces include:

- Omniphony Studio product work;
- mpv distribution integration;
- Linux packaging/product assumptions;
- generic speaker-authoring UX;
- macOS/mobile packaging;
- generic plugin/backend demonstrations with no current owner.

Do not merge broad upstream changes merely to keep histories visually similar.

---

## 4. Recent active-branch sweep

The August 2026 sweep established that several apparently interesting upstream active branches had already effectively landed in this fork:

- `feat/spectral-3d-phantom`: important implementation file already byte-identical;
- `feat/diffuse-mirror-axes`: important implementation file already byte-identical;
- `feat/workflow-runtime-isolation`: core runtime-isolation implementation already byte-identical;
- `ci/skip-unchanged-integration-build`: contains useful workflow ideas, but its Studio/integration-release product is not currently needed;
- `feat/release-0.4.2` / `release`: no hidden ahead-of-main Windows DSP payload found during the sweep;
- macOS signing work: not part of the current milestone.

Do not repeatedly rediscover or re-merge these branches unless upstream changes.

---

## 5. Sending fixes upstream

Do not use upstream as this fork's experiment branch.

A candidate upstream fix should first:

1. solve a real problem here;
2. have reproducible evidence where practical;
3. survive renderer/fidelity validation;
4. be separable from fork-specific product assumptions;
5. improve upstream Omniphony on its own terms.

Then the general portion may be suitable to send upstream.

```text
upstream gives mature machinery
→ fork tests in a concrete Windows product
→ general improvement is proven
→ portable fix may flow back
```

---

## 6. Deletion policy

Prefer deletion over indefinite local archiving when a subsystem has no current owner, but do not contract for aesthetics.

Keep code if at least one is true:

- required by native Windows listening;
- part of the retained realtime renderer;
- required for ordinary stereo inference currently in use;
- required for deterministic known-scene/file-render validation;
- required for HRTF/headphone calibration;
- required to reproduce an audible regression or preserve a protected baseline;
- an isolated experiment answering a current concrete question.

Otherwise:

```text
remove from fork
→ preserve ancestry in Git/upstream/ledger
→ recover later only if evidence changes
```

Deletion that reduces observability or removes a useful A/B control is negative progress.

---

## 7. Current product architecture

Current target:

```text
Windows system/player stereo PCM
        ↓
Windows host input
        ↓
existing Omniphony renderer foundation
+ bounded local scene/evidence as needed
        ↓
validated optional improvements
        ↓
binaural stereo output
        ↓
Windows output device
```

`libaural` is optional future evidence infrastructure, not a mandatory architecture stage.

Cross-platform shells are deferred. Keep the engine boundary clean enough that future ports remain possible, but do not spend current project time implementing them.

---

## 8. Attribution

Independence does not erase ancestry.

Preserve:

- original Git history;
- GPL licensing;
- `NOTICE.md`;
- explicit credit to `mgth/Omniphony` and upstream authorship.

Removing obsolete upstream product surfaces is scope contraction, not removal of authorship credit.