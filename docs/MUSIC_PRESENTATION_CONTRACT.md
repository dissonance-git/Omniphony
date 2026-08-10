# Music presentation contract

Omniphony is not trying to turn every audible event into a spectacular spatial effect.

Its product metaphor is stricter:

> **A world-class mix / mastering / immersive-audio engineer adapts the presentation of every song in real time while preserving what makes the recording work.**

libaural owns hearing.

Omniphony owns presentation.

This document defines the boundary between them.

---

## 1. Hearing state is not a placement command

The core separation is:

```text
source audio
→ libaural hears
→ typed auditory / musical state
→ Omniphony presentation policy
→ scene / trajectories / fields
→ binaural renderer
```

Therefore:

```text
hearing evidence ≠ spatial command
```

Examples of forbidden shortcuts:

```text
"guitar"              → rear-left
"busy source"         → wider
"high novelty"        → farther from centre
"semantic foreground" → louder
"diffuse spectrum"    → reverb
```

A hearing observation can constrain a decision. It does not make the decision by itself.

---

## 2. The portable consumer projection

Omniphony should eventually consume a **bounded projection of one libaural heard world**, not build a second independent artificial ear.

The projection should be small enough for realtime presentation and rich enough to preserve the obligations Omniphony actually needs.

Candidate fields include:

```text
sample/time identity
entity / stream / field id
current auditory role
persistent history handle
compact / broad / diffuse character
audibility / masking
foreground / background relation
transient ownership
pitch / rhythmic / musical-role summaries relevant to presentation
position / extent / motion evidence when actually observed
stability / persistence
uncertainty / competing hypotheses
change events on the sample timeline
```

The exact schema is not frozen.

What is frozen is the principle:

> **A smaller renderer projection may forget relations only when its declared presentation obligations do not require them. It may never replace omitted relations with invented specificity.**

See libaural `docs/CONSUMER_PROJECTIONS.md` and `AUD-PROJ-001`.

---

## 3. Musical importance is not activity

Helix `MUSIC-041` tested obvious low-level proxies for an authored distinction such as a bass line carrying its own melody. More pitch motion, wider range, more transitions and related activity measures did not provide a general implementation of that distinction.

The Omniphony consequence is deliberately conservative:

```text
musical importance / independence
≠ raw activity
≠ note count
≠ pitch range
≠ spectral change
≠ energy
≠ novelty score
```

A source does not deserve more spatial prominence merely because it is busy.

A quiet repeating figure may be structurally essential.

A dense decorative texture may be musically backgrounded.

Presentation should increasingly depend on **role and relation**, with confidence, rather than a DSP excitement meter.

---

## 4. Tempo is not groove

libaural `AUD-RHYTHM-001` established a controlled representation obligation:

```text
tempo
≠ within-beat timing
≠ groove / microtiming
```

In the current fixture:

- a straight offbeat relation survived large tempo, timbre and gain changes;
- the same straight relation survived a 20 dB SNR perturbation;
- moving the offbeat from beat phase 0.50 to 0.62 remained visible while tempo stayed at 120 BPM.

This does not define human groove perception.

It does establish an Omniphony protection law:

> **Do not let spatial processing weaken or smear timing relations merely because BPM, transient energy or spectral content are preserved.**

Fidelity tests for the mature product should therefore include timing/groove relations, not only RMS, peak, spectrum and static stereo image.

---

## 5. Musical relation can survive large surface change

libaural `AUD-MUSIC-005` provides a controlled arrangement-relative baseline.

The same musical material remained strongly aligned through changes in:

- tempo;
- timbre;
- chord voicing;
- absolute key after transposition-aware comparison.

A deliberate two-beat musical alteration remained locally visible while the rest of the version stayed strongly related.

The presentation consequence is important:

```text
musical identity / relation
≠ exact waveform match
```

and:

```text
global relation
can coexist with
local meaningful change
```

Omniphony should eventually be able to preserve a song-level presentation logic while still reacting to local musical changes.

It should not rebuild the whole spatial world merely because a chorus changes orchestration, nor freeze the scene so aggressively that a meaningful local transformation is ignored.

---

## 6. Persistence is not permission to hallucinate ownership

libaural's polyphonic tests now contain a useful ladder:

```text
AUD-MUSIC-002
multiple simultaneous pitches are required

AUD-MUSIC-003
pitch order alone fails persistent voice ownership at a crossing

AUD-MUSIC-004
individual cues can collapse, conflict or become non-identifying

AUD-FRONTIER-001
one verified ambiguity benefits from retaining two bounded ownership hypotheses
```

Omniphony should not convert uncertain hearing into aggressive scene motion.

For example:

```text
high-confidence persistent compact source
→ precise/stable object presentation may be allowed

ambiguous ownership but stable broad relation
→ preserve or use broad/conservative presentation

non-identifiable source allocation
→ do not fabricate exact object decomposition
```

Uncertainty is therefore a presentation input.

---

## 7. Confidence controls aggression, not truth

A useful policy shape is:

```text
high confidence
→ greater permission for specific presentation changes

medium confidence
→ broad / reversible / conservative changes

low confidence
→ preserve authoritative mix relationships
```

This does **not** mean confidence decides what is true.

It controls how much Omniphony is allowed to bet the listener's recording on an interpretation.

A low-confidence state can still contain valuable observations.

A high-confidence semantic label still does not authorize a fixed spatial effect.

---

## 8. Preserve the authoritative musical floor

Unless evidence and validated policy justify otherwise, the original mastered recording remains the authority for:

- centre of gravity;
- bass foundation;
- groove;
- transient ownership;
- vocal/instrument focus;
- dynamics;
- tonal hierarchy;
- important stereo relationships.

Current scene inference already contains bass/foundation protection. The mature presentation layer should generalize this idea to musical structure rather than using frequency alone.

A transformation that increases dimensionality while weakening the song fails.

---

## 9. Presentation entities

The renderer vocabulary remains:

```text
FrontalAnchor
DirectObject
BroadSource
DiffuseField
RoomField
```

These are presentation entities, not claims that the stereo master contained authored object metadata.

### FrontalAnchor

A protected centre-of-gravity relationship.

Typical evidence may include strong mix authority, musical focus, persistence and high cost of destabilization.

### DirectObject

Compact persistent material for which a specific spatial presentation is justified.

### BroadSource

Coherent source-like material whose extent is meaningful.

### DiffuseField

Musical/ambient material that is better represented as a directional distribution than a point.

### RoomField

Presentation-environment energy such as reflections and late reverberation.

Critical distinction:

```text
DirectObject
≠ BroadSource
≠ DiffuseField
≠ RoomField
```

The FDN is a `RoomField`, not a substitute for musical diffuseness.

---

## 10. Rear presentation policy

A stereo master generally does not identify literal authored rear-source coordinates.

Therefore:

```text
stable stereo evidence
≠ recovered rear metadata
```

But Omniphony is a presentation system, not a forensic reconstruction system.

A rear direct object can be an artistically justified immersive **presentation choice** when the heard state and policy support it.

The contract is:

```text
rear direct object
≠ rear reflection
≠ diffuse rear field
```

and:

> **Never describe a presentation decision as recovered source truth unless the source format actually supplied that truth.**

---

## 11. Reversibility during uncertainty

When hearing state is unstable or still resolving, prefer changes that are easy to undo perceptually.

Examples:

- preserve original image rather than commit to an unsupported point location;
- use broad extent before precise rear placement;
- change separation gradually rather than teleport an object;
- retain scene-history hypotheses until later evidence resolves them;
- crossfade profile/scene changes rather than replace state discontinuously.

This is particularly important in realtime because the system hears the song while the listener hears the system hearing it.

---

## 12. Sample-time presentation

Musical policy decisions happen at musical timescales, but their audible application belongs to the audio timeline.

```text
phrase / role / scene decision
        ↓
timed control event
        ↓
continuous sample-time trajectory
        ↓
renderer
```

A callback is never a musical boundary simply because the host delivered a buffer there.

The binaural metadata-gain path now has a callback-invariance gate. Position/HRTF movement remains the active trajectory defect until the parent renderer publishes a canonical sample-time position segment.

---

## 13. Platform independence

Nothing in this policy should know whether the audio came from:

- WASAPI;
- CoreAudio;
- PipeWire;
- AAudio;
- an iOS audio session;
- a file decoder;
- a plugin host;
- another future transport.

Platform shells move samples and device state.

The portable presentation core reasons about heard music and sample-time trajectories.

---

## 14. Validation ladder

A future presentation rule should graduate through increasingly expensive evidence.

```text
candidate rule
→ controlled synthetic fixture
→ negative control
→ fidelity metrics
→ arrangement / perturbation test
→ known-scene render test where applicable
→ real music corpus / aligned versions
→ matched-loudness human listening
→ long-session regression
```

No amount of structural elegance replaces listening.

### Objective preservation gates

Current/future gates should include:

- peak / clipping;
- RMS / level;
- crest factor;
- DC;
- residual/null where an invariant output is expected;
- transient timing;
- low-frequency timing/coherence;
- callback-size invariance;
- HRTF direction cues;
- profile/scene switch continuity.

### Musical gates

As libaural improves, add controlled measurements for:

- beat-relative timing / groove;
- persistent layer ownership;
- local versus global musical change;
- section/phrase continuity;
- foreground/background hierarchy;
- masking-aware audibility.

### Human listening gates

Ultimately ask whether the processing:

- improves externalization and dimensionality;
- retains centre authority when appropriate;
- keeps bass/groove locked;
- avoids phasey/diffuse damage to direct material;
- avoids listener fatigue;
- makes bypass feel flatter rather than cleaner.

---

## 15. First policy prototype should be small

Do not begin with an AI that attempts to remix every property of every source.

A good first live policy can be intentionally bounded:

```text
INPUT
persistent libaural entity/field hypotheses
+ confidence
+ masking/audibility
+ broad musical-role/importance evidence
+ groove/foundation protection

OUTPUT
protect / preserve
or
allow modest separation / extent
or
allow specific spatial placement
or
allow diffuse-field treatment
```

The first job is to make **better choices than static upmix heuristics without damaging the recording**.

Every additional artistic degree of freedom must earn itself through listening and fidelity tests.

---

## 16. Forbidden shortcuts

Do not implement these as product laws:

```text
busy = important
loud = foreground
quiet = background
semantic label = scene role
pitch register = persistent identity
source separator stem = perceptual object
wide stereo = rear source
low coherence = room
novelty = permission to move
higher confidence = louder
larger model = better hearing
```

Each may sometimes correlate with something useful.

None is sufficient by itself.

---

## 17. Current implementation frontier

The renderer is ahead of the presentation policy in some areas and behind it in others.

Already useful:

- conservative stereo evidence;
- persistence/stability tracking;
- bass/foundation safeguards;
- explicit compact/broad/diffuse distinctions;
- direct HRTF path;
- RoomField reflections/FDN;
- measured HRTF validation;
- sample-time metadata gain;
- deterministic fidelity fixtures.

Still required before the product metaphor becomes real:

1. fix sample-time position/HRTF movement;
2. propagate object extent to headphones;
3. implement a genuine `BroadSource` renderer;
4. implement musical `DiffuseField` separately from `RoomField`;
5. wire ordinary stereo into persistent realtime heard-scene state;
6. establish a stable bounded libaural→Omniphony projection;
7. build and falsify the first music-aware presentation policy;
8. test on real music at matched loudness;
9. only then expand artistic freedom.

---

## Frozen presentation laws

1. **Hearing evidence is not a placement command.**
2. **Musical importance is not raw activity.**
3. **Tempo and groove/microtiming are distinct.**
4. **Global musical relation and local musical change may coexist.**
5. **Uncertain source ownership must not become precise spatial fiction.**
6. **Confidence controls presentation aggression, not truth.**
7. **The mastered recording remains the authoritative fidelity floor.**
8. **Direct object, broad source, diffuse musical field and room field remain distinct.**
9. **Rear presentation choices must not be described as recovered metadata.**
10. **Audible trajectories live in sample time, not host callback time.**
11. **The presentation policy belongs to the portable core, not the OS shell.**
12. **A new artistic degree of freedom must earn itself through evidence and listening.**

The target remains simple:

> **Make the song feel as though its presentation had been personally rebuilt for headphones by an extraordinary immersive engineer, while making Omniphony itself disappear.**
