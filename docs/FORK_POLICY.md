# Omniphony fork policy

This repository is now an **independent product/research fork** of `mgth/Omniphony`.

The original project remains a major technical source and the ancestry/attribution remains permanent, but upstream no longer defines this repository's product surface, folder structure, compatibility promises, or development priorities.

```text
mgth/Omniphony
source / peer / upstream history
        │
        │ selective mechanisms and fixes
        ▼
dissonance-git/Omniphony
Windows stereo-music → auditory scene → binaural product
        │
        │ general fixes proven here
        ▼
possible upstream contribution
```

## Direction of travel

The fork optimizes for one narrow listener-facing goal:

> ordinary Windows stereo music should become a stable, externalized, full-sphere headphone scene while preserving the recording's musical identity and fidelity.

Code that does not serve that goal, a required Windows integration path, or a valuable deterministic calibration/validation fixture does not have permanent residency merely because it exists upstream.

Upstream history is the archive. This tree should become the product.

---

## Pulling from upstream

Upstream work is treated like any other high-quality influence:

```text
inspect
→ identify exact mechanism
→ test relevance to our product
→ port/cherry-pick/reimplement the smallest useful part
→ validate locally
```

Do not merge broad upstream product changes merely to keep histories visually similar.

High-value upstream candidates include:

- binaural/HRTF correctness fixes;
- realtime-safety fixes;
- renderer math fixes;
- deterministic DSP test improvements;
- Windows audio fixes relevant to our route;
- performance improvements with compatible semantics;
- reusable scene/geometry machinery that supports our calibration or rendering needs.

Low-priority upstream surfaces include:

- Omniphony Studio UI/product work;
- mpv distribution integration;
- Linux packaging;
- general speaker-authoring UX;
- generic plugin/backend demonstrations;
- compatibility code for product paths this fork has intentionally removed.

---

## Sending fixes upstream

Do not use upstream as this fork's test branch.

A fix should first:

1. solve a real problem in this fork;
2. have a regression test or other reproducible evidence where practical;
3. survive our renderer/fidelity validation;
4. be separated from fork-specific product assumptions;
5. improve upstream Omniphony on its own terms.

Only then should we consider sending the general portion back.

This creates a reciprocal relationship without forcing the fork to remain structurally compatible:

```text
upstream gives us mature machinery
→ fork experiments aggressively
→ fork proves general improvements
→ portable fixes may flow back upstream
```

---

## Deletion policy

Because upstream preserves the broad suite, this fork should prefer deletion over indefinite local archiving when a subsystem has no current owner.

Keep code only if at least one is true:

- it is required by the Windows listening product;
- it is part of the realtime renderer path;
- it is required to infer the stereo auditory scene;
- it is a deterministic known-scene / file-render test fixture;
- it is required for HRTF/headphone calibration;
- it provides a clearly useful research comparison that cannot cheaply live outside runtime scope.

Otherwise:

```text
remove from fork
→ cite upstream/source in influence ledger if useful
→ recover later from Git history/upstream only if evidence changes
```

The fork should become easier to understand after every contraction pass.

---

## Product architecture target

```text
Windows system/player stereo PCM
        ↓
realtime multiresolution analysis
        ↓
libaural-informed persistent scene evidence
        ↓
anchors / direct objects / broad sources / diffuse fields
        ↓
Omniphony binaural renderer
        ↓
listener HRTF / room / headphone calibration
        ↓
stereo output device
```

The runtime should remain small. Rich calibration, HRTF conversion, corpus generation, symbolic-music fixtures and expensive analysis belong in offline/control tooling whenever possible.

---

## Attribution

Independence does not erase ancestry.

The original Git history, GPL licensing, `NOTICE.md`, and explicit credit to `mgth/Omniphony` must remain. Removing obsolete upstream product surfaces is repository contraction, not removal of authorship credit.
