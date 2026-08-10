# Music presentation contract

This document defines the **music-preserving spatial-presentation rules** for Omniphony.

It is subordinate to the root `README.md`.

Ordinary stereo music is the main everyday use case. Native surround/object audio is a richer source when available, not a different product identity.

The portable product and the protected upstream Omniphony percept do **not** depend on libaural, a semantic music model, or a giant adaptive runtime.

Current hierarchy:

```text
existing Omniphony sound
→ clean platform route
→ trustworthy stereo music baseline
→ measured/listened renderer improvements
→ bounded stereo presentation
→ optional richer mechanisms only when earned
```

The purpose of this contract is to make immersive processing feel **pre-authored and transparent**, not busy or reactive.

---

## 1. Product floor comes first

Omniphony already has a useful binaural renderer.

Any additional presentation mechanism sits above that floor and must be attributable/bypassable.

```text
ordinary audio
        ↓
protected Omniphony rendering path
        ↓
useful product

OPTIONAL
        ↑
additional presentation mechanisms
```

If an added mechanism makes the protected path sound worse, it loses.

---

## 2. Stereo is the primary creative problem

The everyday source is usually:

```text
L R
```

The finished physical output is also:

```text
binaural L R
```

The difficult middle problem is not “make stereo into fake 7.1.”

It is:

> **Give the mastered stereo recording a convincing full-sphere physical presentation while preserving the recording's identity, timing, hierarchy and tonal character.**

The desired result should sound as if the recording had already been prepared for this presentation before playback began.

---

## 3. Pre-authored-quality law

The presentation should feel:

```text
stable
finished
authored
coherent
```

not:

```text
live remixed
section-reactive
wandering
showy
algorithmically restless
```

No rule is good merely because it can detect a chorus, solo, instrument or section.

No source should visibly “move because the classifier changed its mind.”

A stateful processor is fine. An audible sense of ongoing reinterpretation is not.

---

## 4. Hearing evidence is not a placement command

Whether evidence comes from local DSP, a learned model, or libaural:

```text
hearing evidence ≠ spatial command
```

Forbidden shortcuts include:

```text
"guitar"              → rear-left
"busy source"         → wider
"high novelty"        → farther from centre
"semantic foreground" → louder
"diffuse spectrum"    → room reverb
```

Evidence may constrain presentation. It does not authorize a canned effect by itself.

---

## 5. The mastered recording remains authoritative

Unless a validated mechanism earns otherwise, preserve:

- center of gravity;
- bass foundation;
- groove;
- transient ownership;
- vocal/instrument focus;
- dynamics;
- tonal hierarchy;
- important stereo relationships;
- recording character;
- exact musical timing.

A transformation that increases dimensionality while weakening the song fails.

---

## 6. Musical importance is not activity

Do not equate importance with:

```text
raw activity
note count
pitch range
spectral change
energy
novelty
```

A quiet repeating figure may be structurally essential.
A dense texture may be background.
A source should not gain spatial prominence merely because it is numerically busy.

---

## 7. Tempo is not groove

Preserving BPM is not enough if spatial processing smears within-beat timing or coupled rhythmic relationships.

Protect:

- attack timing;
- bass/drum lock;
- microtiming;
- transient ownership;
- rhythmic precision.

This is part of the same product law as preserving physical bass/groove quality from the incumbent chain.

---

## 8. Bass has multiple jobs

Helix research has already shown that “more bass” is not a useful universal explanation.

Useful distinctions include:

```text
physical mass
melodic/independent line
groove anchor
timbral color
transitional role
interlock with drums/other parts
```

Omniphony should not collapse these into one global LFE boost or one spatial rule.

A bass line may need contour/agency preserved.
A sub/foundation may need body and stability preserved.
A groove bass may be judged primarily by timing integrity.

These are evaluation constraints first, runtime mechanisms only if later evidence supports them.

---

## 9. Direct / broad / diffuse / room remain distinct

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

A “bigger” sound is not automatically better if the mechanism converts direct musical material into room coloration.

The first live prototype's reported hallway/tinny character is exactly why this distinction matters, although that first listen was route-contaminated and is not yet a renderer verdict.

---

## 10. Source truth outranks interpretation

For real multichannel/spatial sources:

```text
5.1 / 7.1 / height / objects
→ preserve authored positions/relations
→ enhance with Omniphony rendering
```

Do not flatten real surround to stereo and then ask the stereo presentation layer to rediscover it.

Stereo presentation is a solution for missing spatial truth, not a reason to ignore existing truth.

---

## 11. Concurrent sources must not contaminate each other

A desktop may contain:

```text
stereo music
+
native-surround game
```

The music remains a stereo logical source.
The game remains a surround logical source.

Neither should change the other's interpretation merely because both are sounding.

This is a presentation-law issue as well as a host issue:

> **Channel layout and source semantics are stream-local.**

---

## 12. libaural is research/evidence infrastructure

`libaural` is a separate artificial-hearing research/framework project.

It may provide better mechanisms, invariants or bounded heard-state projections.

The product must not become structurally dependent on the full libaural research stack.

Correct relationship:

```text
Helix / libaural research
→ identify mechanism
→ compress / simplify
→ bounded implementation candidate
→ objective checks + listening
→ Omniphony only if earned
```

A smaller deterministic mechanism remains preferable when it is cheaper, more stable, easier to attribute, and sounds as good or better.

---

## 13. Helix music concepts are research coordinates

Useful concepts include:

```text
line + field
role elasticity
relational continuity
synchronization under heterogeneity
bass-function plurality
pressure topology
closure latency
temporal sovereignty
world formation
```

They are useful for:

- exact-moment listening tests;
- failure discovery;
- negative controls;
- asking what relationships processing must preserve;
- teaching libaural what to investigate.

They are **not automatic Omniphony modules**.

The product consumes distilled results.

---

## 14. Uncertainty controls aggression

A useful policy shape is:

```text
high confidence
→ more permission for specific reversible presentation

medium confidence
→ broader / safer / slower change

low confidence
→ preserve authoritative mix relationships
```

Confidence does not make an interpretation true.

Do not turn uncertain source ownership into precise spatial fiction.

---

## 15. Reversibility during uncertainty

Prefer changes that can be perceptually undone without a jarring scene collapse:

- preserve original image before unsupported precise placement;
- use broad extent before exact rear placement;
- move gradually rather than teleport;
- crossfade state/profile changes;
- fall back to the protected baseline on failure.

The system should fail conservative, not theatrical.

---

## 16. Sample-time application

Any audible state change belongs to the audio timeline.

```text
presentation decision
        ↓
timed control event
        ↓
continuous sample-time trajectory
        ↓
renderer
```

A host callback is never a musical boundary merely because an API delivered a buffer there.

---

## 17. Bypass is part of music evaluation

The desired matched-loudness result is:

```text
ON
→ same music, stronger world

OFF
→ world collapses, music remains intact
```

A bypass contaminated by:

```text
queued wet tail
duplicate physical path
phase/comb filtering
volume advantage
```

cannot be used to evaluate the music presentation.

The first live prototype therefore proved transport but did **not** yet produce a trustworthy quality A/B.

---

## 18. First useful stereo prototype should stay small

Do not begin with an AI that tries to remix every source property.

A useful first stereo presentation may be only:

```text
preserve strong direct stereo image
+
add validated spatial support / depth / externalization
+
protect center / bass / transient timing
```

Then add one degree of freedom at a time.

Every artistic degree of freedom must earn itself independently.

---

## 19. Validation order

A candidate rule should graduate through:

```text
candidate mechanism
→ controlled fixture
→ negative control
→ fidelity metrics
→ protected-renderer comparison
→ real music
→ matched-loudness clean-route listening
→ incumbent-chain comparison
→ long-session regression
```

Objective preservation checks may include:

- clipping/headroom;
- RMS/level;
- crest factor;
- DC;
- frequency response;
- transient timing;
- low-frequency timing/coherence;
- callback-size invariance;
- HRTF direction cues;
- switch continuity.

Human listening asks whether processing:

- improves externalization/dimensionality;
- retains center authority;
- keeps bass/groove locked;
- avoids phasey/direct-material damage;
- avoids excess room/hallway coloration;
- avoids fatigue;
- makes bypass feel flatter rather than cleaner.

---

## 20. Forbidden product laws

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
stereo = fake 7.1 by definition
surround app active = all sources are surround
```

Each may sometimes correlate with something useful.
None is sufficient by itself.

---

## 21. Current implementation frontier

Immediate order:

```text
1. prove one physical Windows path
2. clean OFF/bypass
3. test ordinary stereo music cleanly
4. compare base Omniphony against dry stereo and incumbent
5. test native surround
6. test stereo + surround simultaneously
7. only then tune the actual music presentation
```

Do not tune around a double/phase route.

---

## 22. Frozen presentation laws

1. **Ordinary stereo music is the main use case.**
2. **The song should feel pre-authored for the immersive presentation, not remixed live.**
3. **The protected Omniphony renderer is useful without adaptive hearing.**
4. **Hearing evidence is not a placement command.**
5. **libaural is research/evidence infrastructure, not the product owner.**
6. **Musical importance is not raw activity.**
7. **Tempo and groove/microtiming are distinct.**
8. **The mastered recording remains the authoritative fidelity floor.**
9. **Direct, broad, diffuse and room roles remain distinct.**
10. **Source truth outranks inferred presentation.**
11. **Channel layout is stream-local, not global.**
12. **Stereo and native surround must be able to coexist.**
13. **Uncertain source ownership must not become precise spatial fiction.**
14. **Audible trajectories live in sample time, not callback time.**
15. **Every new artistic degree of freedom must earn itself through clean-route listening.**
16. **If bypass sounds cleaner rather than merely flatter, the candidate loses.**

The target is not to make Omniphony seem intelligent.

The target is to make the music inhabit a stronger headphone world while the processing itself disappears.
