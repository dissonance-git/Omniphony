# Omniphony development contract

## GitHub connector entrance

When entering this repository through GitHub, an LLM connector, or another remote agent surface, treat the root documents as the lobby rather than trying to infer the project from code search alone.

Use this bounded entrance sequence:

```text
current main HEAD
→ README.md
→ AGENTS.md at the same HEAD
→ recent commits
→ current listening frontier
→ smallest task-relevant code / tests / history
→ exact target file
```

1. Resolve `dissonance-git/Omniphony-Headphones` and record the current `main` commit before substantive work.
2. Read `README.md` first because it owns the current listening model, retained baseline, active candidate, and frontier. Read this `AGENTS.md` from the same repository state for implementation and listening law.
3. Inspect recent commits to see which candidate or repair is actively moving. Recent activity does not overrule the retained listening baseline or the current root documents.
4. Hydrate only the task-relevant region. Do not pull in all of libaural, VGM Tooling, or Helix unless the work genuinely crosses those boundaries.
5. Before any GitHub replacement write, re-fetch current `main` and the exact target file. If `main` changed since preflight, re-read this file, refresh the README/frontier when relevant, and reconstruct the edit from current target content.
6. Write against the exact current blob SHA. Preserve unrelated concurrent work. Never replace a file from a cached or reconstructed older copy.
7. After publication, fetch the resulting commit, inspect its changed paths, and confirm the commit remains in current `main` history. Report publication, compile/tests, CI, measurements, and physical listening as separate evidence states.

Fast routes:

- current listening model, retained baseline, active candidate, and frontier: `README.md`
- governing implementation/listening law: `AGENTS.md`
- perceptual promotion and rejection history: `docs/listening-history.md`
- portable renderer/DSP implementation: `omniphony-renderer/`
- Windows host/integration code: follow the current README repository guide and exact implementation path rather than guessing from historical layouts
- CI and build behavior: `.github/`
- historical or research context: `docs/`, only as required by the current task

For sound-changing work, the shortest valid path is normally `README.md` → relevant recent commits → exact renderer code/tests → physical listening history. Do not begin with a broad historical scan.

## Project instruction chain

This repository is the independent implementation home for `project:omniphony`, which is tracked by Helix and is a child project of libaural.

Before substantive work, read the current instruction chain in order:

1. `dissonance-git/Helix/AGENTS.md` for the common operating law;
2. `dissonance-git/libaural/AGENTS.md` for applicable parent-project research law;
3. this file for Omniphony-specific implementation and listening law.

A child inherits only the parent laws that apply to its work. This file may specialize them where realtime rendering, listening authority, Windows integration, fidelity, or product constraints genuinely differ. Direct user instruction or correction outranks the entire chain.

Do not copy Omniphony's implementation into Helix or libaural merely to make the connection visible, and do not copy parent machinery into this repository merely to inherit its design. Helix preserves project continuity and cross-project routes; libaural owns general artificial-hearing research; this repository remains canonical for Omniphony code, tests, builds, local implementation history, and releases.

## Research gate

Every substantive change that can alter what the listener hears must begin with both:

1. a literature pass over relevant peer-reviewed / standards / primary technical work; and
2. a GitHub implementation pass over mature open-source systems that solve the same or an adjacent problem.

Do not tune Omniphony from intuition alone when established perceptual research or implementation precedent is available.

Preferred loop:

```text
listening observation
→ literature pass
→ mature implementation pass
→ identify the smallest relevant mechanism
→ adapt it to Omniphony's topology
→ CI / measurement
→ physical listening
→ keep, revise, or revert
```

Research is an influence source, not permission to replace a working baseline with a parallel science project. Prefer mechanisms already owned by upstream Omniphony where possible.

Purely mechanical build, packaging, CI, formatting, or compile repairs do not require new audio research so long as they cannot alter runtime sound.

## Core independence

The renderer / inference / DSP core must remain portable and independent of Windows. Windows owns capture, playback, lifecycle, recovery, autostart, tray behavior, and endpoint integration only.

Do not move Windows concepts into the portable core to solve host problems.

## Source authority

The finished master is authoritative. For stereo music, keep the protected direct master explicitly present and use inferred spatial content only as bounded additive support.

More source truth means less inference:

```text
stereo → protected master + bounded inferred support
multichannel → preserve authored channels
object audio → preserve supplied positions
Ambisonics / HOA → preserve the field
already-binaural → avoid destructive double virtualization
```

## Fidelity laws

- Dimension may not be purchased by damaging the music.
- OFF may collapse the world; it may not bring the rhythm section back to life.
- Energy may be anchored; authored motion may not be frozen.
- Bass pressure, kick weight, transient ownership, center stability, dynamics, tonal identity, and stereo motion are protected invariants.
- Do not recover spatial scale by adding excessive late reverb, treble energy, or diffuse duplication.
- Prefer geometry, HRTF / ITD, distance, early-field structure, source extent, and physically motivated room cues.

## Current spatial direction

The active music architecture uses a protected stereo master plus coherent foundation and a derived spatial support field rendered through Omniphony's cascaded binaural path. The current frontier uses an ITU-R BS.2051 System-H-derived 22-direction full-sphere support shell, including a real lower hemisphere.

Dense spatial sampling is not permission to smear sources. The current anti-blur law is:

> **The environment may grow while direct musical structure becomes sharper.**

## Listening authority

Physical listening outranks theory. Measurements and papers guide candidate mechanisms; they do not redefine success after the fact.

When a build is clearly better, preserve it as a rollback point before pushing farther. If a new mechanism damages a winning invariant, revert the mechanism rather than lowering the invariant.

## libaural relationship

libaural is the parent research project for reusable auditory / machine-hearing mechanisms. Omniphony should import only small validated distinctions that improve the consumer renderer without replacing its working spatial core.

```text
Helix research machinery
        ↓
libaural auditory research
        ↓
small validated mechanisms
        ↓
Omniphony consumer renderer
```