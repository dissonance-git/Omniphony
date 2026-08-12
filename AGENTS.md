# Omniphony development contract

## Helix relationship

This repository is the independent implementation home for `project:omniphony-immersive-audio`, which is tracked by Helix for project continuity, research, evidence, relationships, negative results, and re-entry.

Before substantive work, read the current Helix operating law at `dissonance-git/Helix/AGENTS.md` and apply the parts relevant to evidence, provenance, correction, validation, re-entry, and concurrent repository safety. This file owns Omniphony's project-specific implementation laws and may specialize repository workflow where the project genuinely differs. Direct user instruction or correction outranks both.

Do not copy Omniphony's implementation into Helix merely to make the connection visible, and do not copy Helix machinery into this repository merely to inherit its design. Helix preserves the exact route to this project; this repository remains canonical for its code, tests, builds, local implementation history, and releases.

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

libaural is the next research layer for reusable auditory / machine-hearing mechanisms. Omniphony should import only small validated distinctions that improve the consumer renderer without replacing its working spatial core.

```text
Helix research machinery
        ↓
libaural auditory research
        ↓
small validated mechanisms
        ↓
Omniphony consumer renderer
```