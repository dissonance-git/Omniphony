# Music presentation contract

This document defines **future adaptive-presentation rules** for Omniphony.

It is subordinate to the root `README.md`.

The native Windows product and the protected upstream Omniphony percept do **not** depend on completing this contract, on libaural, or on solving general artificial hearing first.

Current hierarchy:

```text
existing Omniphony sound
→ native Windows listening product
→ measured/listened renderer improvements
→ bounded stereo scene behavior
→ optional richer adaptive presentation
→ libaural-informed policy when it proves useful
```

The purpose of this document is to keep future intelligence conservative and musically safe, not to make it a prerequisite for playback.

---

## 1. Product floor comes first

Omniphony already has a useful binaural renderer.

Any adaptive layer sits **above** that floor and must be bypassable for attribution.

```text
ordinary audio
        ↓
protected Omniphony rendering path
        ↓
useful product

OPTIONAL
        ↑
adaptive scene/presentation decisions
```

If adaptive processing makes the protected path sound worse, the adaptive processing loses.

---

## 2. Hearing evidence is not a placement command

Whether the evidence comes from local DSP, a future learned model, or `libaural`:

```text
hearing evidence ≠ spatial command
```

Forbidden shortcuts include:

```text
"guitar"              → rear-left
"busy source"         → wider
"high novelty"        → farther from centre
"semantic foreground" → louder
"diffuse spectrum"    → reverb
```

Evidence constrains presentation. It does not authorize a canned effect by itself.

---

## 3. libaural is optional evidence infrastructure

`libaural` is a separate artificial-hearing research/framework project.

It may later provide a bounded heard-state projection that improves Omniphony's choices.

The product must not become structurally dependent on the full libaural research stack.

Correct future relationship:

```text
local Omniphony evidence / scene state
        │
        ├→ sufficient for baseline product
        │
        └→ may be enriched by
           bounded libaural state
```

Candidate optional fields could include:

```text
sample/time identity
persistent entity / field id
compact / broad / diffuse character
audibility / masking
foreground / background relation
transient ownership
musical-role summaries
position / extent / motion evidence
stability / persistence
uncertainty / competing hypotheses
```

The exact schema is not frozen.

A smaller local mechanism may remain preferable when it is cheaper, more stable, easier to attribute, and sounds as good or better.

---

## 4. Musical importance is not activity

Do not equate musical importance with:

```text
raw activity
note count
pitch range
spectral change
energy
novelty score
```

A quiet repeating figure may be structurally essential.

A dense texture may be background.

A source should not receive more spatial prominence merely because it is numerically busy.

---

## 5. Tempo is not groove

Preserving BPM is not enough if spatial processing smears within-beat timing or coupled rhythmic relationships.

Protect:

- attack timing;
- bass/drum lock;
- microtiming;
- transient ownership;
- rhythmic precision.

This is part of the same product law as preserving the incumbent chain's strong physical bass/groove floor.

---

## 6. Uncertainty controls aggression

A useful policy shape is:

```text
high confidence
→ more permission for specific reversible presentation

medium confidence
→ broader / safer / slower changes

low confidence
→ preserve authoritative mix relationships
```

Confidence does not make an interpretation true.

It controls how much of the listener's recording Omniphony is allowed to bet on that interpretation.

Do not turn uncertain source ownership into precise spatial fiction.

---

## 7. The mastered recording remains authoritative

Unless a validated presentation rule earns otherwise, preserve:

- center of gravity;
- bass foundation;
- groove;
- transient ownership;
- vocal/instrument focus;
- dynamics;
- tonal hierarchy;
- important stereo relationships;
- recording character.

A transformation that increases dimensionality while weakening the song fails.

---

## 8. Presentation entities

The renderer vocabulary remains useful:

```text
FrontalAnchor
DirectObject
BroadSource
DiffuseField
RoomField
```

These are presentation entities, not claims that a stereo master contained authored object metadata.

Keep:

```text
DirectObject
≠ BroadSource
≠ DiffuseField
≠ RoomField
```

and:

```text
rear direct object
≠ rear reflection
≠ diffuse rear field
```

A stereo master normally does not identify literal rear-source coordinates. Rear placement can still be a valid immersive presentation choice when evidence and listening justify it.

Never describe a presentation choice as recovered source truth unless the source format actually supplied that truth.

---

## 9. Reversibility during uncertainty

Prefer changes that can be undone perceptually without a jarring scene collapse:

- preserve original image before unsupported precise placement;
- use broader extent before committing to exact rear location;
- move gradually rather than teleport;
- retain competing hypotheses when useful;
- crossfade state/profile changes;
- fall back to the protected baseline on model/control failure.

The system should fail conservative, not theatrical.

---

## 10. Sample-time application

Musical decisions may happen at phrase or section timescales, but audible application belongs to the audio timeline.

```text
musical / scene decision
        ↓
timed control event
        ↓
continuous sample-time trajectory
        ↓
renderer
```

A host callback is never a musical boundary merely because an API delivered a buffer there.

---

## 11. First adaptive prototype should be small

Do not start with an AI that tries to remix every property of every source.

A useful first adaptive policy could be limited to:

```text
protect / preserve
or
allow modest separation / extent
or
allow a specific stable placement
or
allow diffuse-field treatment
```

Inputs can initially be local Omniphony evidence and confidence.

Later, libaural may replace or enrich individual inputs only when tests show an advantage.

Every additional artistic degree of freedom must earn itself independently.

---

## 12. Validation order

A candidate presentation rule should graduate through increasingly expensive evidence:

```text
candidate rule
→ controlled fixture
→ negative control
→ fidelity metrics
→ protected-renderer comparison
→ real music
→ matched-loudness listening
→ incumbent-chain comparison
→ long-session regression
```

No amount of structural elegance replaces listening.

Objective preservation checks should include where applicable:

- clipping/headroom;
- RMS/level;
- crest factor;
- DC;
- null/residual when invariance is expected;
- transient timing;
- low-frequency timing/coherence;
- callback-size invariance;
- HRTF direction cues;
- scene/profile switch continuity.

Human listening asks whether the processing:

- improves externalization and dimensionality;
- retains center authority;
- keeps bass/groove locked;
- avoids phasey/diffuse damage to direct material;
- avoids fatigue;
- makes bypass feel flatter rather than cleaner.

---

## 13. Forbidden product laws

Do not encode these as general rules:

```text
busy = important
loud = foreground
quiet = background
semantic label = scene role
pitch register = persistent identity
separator stem = perceptual object
wide stereo = rear source
low coherence = room
novelty = permission to move
higher confidence = louder
larger model = better hearing
AI unavailable = product unavailable
```

Each may sometimes correlate with something useful.

None is sufficient by itself.

---

## 14. Current implementation frontier

Do not treat the list below as the immediate roadmap. The root README owns priority.

Current presentation-related capabilities include:

- conservative stereo evidence;
- persistence/stability tracking;
- bass/foundation safeguards;
- compact/broad/diffuse distinctions;
- direct HRTF rendering;
- room reflections/FDN;
- deterministic fidelity fixtures.

Future adaptive work may include:

- a small persistent realtime scene from ordinary stereo;
- better source extent handling;
- first-class broad/diffuse rendering where listening shows a need;
- bounded optional libaural evidence;
- music-aware policy with explicit preservation gates.

But the next product milestone remains the coexisting native Windows listening lane, not completion of this list.

---

## 15. Frozen presentation laws

1. **The protected Omniphony renderer is usable without adaptive hearing.**
2. **Hearing evidence is not a placement command.**
3. **libaural is optional evidence infrastructure, not the product owner.**
4. **Musical importance is not raw activity.**
5. **Tempo and groove/microtiming are distinct.**
6. **Uncertain source ownership must not become precise spatial fiction.**
7. **Confidence controls presentation aggression, not truth.**
8. **The mastered recording remains the authoritative fidelity floor.**
9. **Direct object, broad source, diffuse musical field and room field remain distinct.**
10. **Rear presentation choices must not be described as recovered metadata.**
11. **Audible trajectories live in sample time, not host callback time.**
12. **A new artistic degree of freedom must earn itself through objective checks and listening.**
13. **If adaptive presentation makes bypass sound cleaner, adaptive presentation loses.**

The target is not to make Omniphony seem intelligent.

The target is to make the music inhabit a stronger headphone world while the processing itself disappears.