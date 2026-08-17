# Headphone rendering research and validation map

> **Scope:** research obligations for improving Omniphony's headphone presentation without replacing the protected musical baseline.
>
> The root `README.md` owns product identity. This document turns research into testable renderer obligations.

The practical principle is:

> **Research earns mechanisms only when they solve an isolated perceptual or fidelity problem without damaging the recording underneath them.**

Omniphony is not a blank-sheet binaural engine. The inherited renderer and the retained Current listening path are protected starting points.

---

## 1. Current architecture under study

```text
finished stereo master
        │
        ├→ protected direct master
        ├→ coherent foundation
        └→ stereo evidence
              ↓
       canonical 8.1.4.4 scene
              ↓
       22-direction Current shell
              ↓
       cascaded binaural render
              ↓
       directional early field
              ↓
       bounded late closure
              ↓
master + foundation + support
              ↓
          headphones
```

Research should target a named link in this chain. It should not silently redefine the whole chain.

---

## 2. Perceptual obligations are not one axis

The literature is much easier to apply when the problem is decomposed.

### Externalization

Question:

> Does the sound appear outside the head at a plausible distance?

Externalization is not identical to localization accuracy. A source can point in a plausible direction and still feel trapped near or inside the head.

### Localization

Question:

> Is direction stable and discriminable, including front/back and elevation?

### Timbre

Question:

> Did spatial processing preserve spectral identity rather than buying space with coloration?

### Envelopment

Question:

> Does the environment surround the listener without smearing direct material?

### Motion consistency

Question:

> Does the scene behave like a world rather than a pattern glued to the head or callback blocks?

### Musical fidelity

Question:

> Did the rendering preserve center authority, transients, bass timing, dynamics and hierarchy?

A candidate must state which obligation it is trying to improve.

---

## 3. Externalization evidence

### Front and rear are hard cases

Binaural research repeatedly identifies frontal and rear externalization as difficult, particularly with non-individualized HRTFs.

This supports the Current development choice to treat **front scale / center directness** as its own frontier instead of assuming that adding more surround energy will solve it.

### Room cues are useful but must be binaurally meaningful

Catic, Santurette & Dau (2015) found that the interaction between direct and reverberant interaural cues strongly affects externalization, with frontal sources requiring more binaural reflection information than lateral sources in their tests.

DOI: `10.1121/1.4928132`

Leclère, Lavandier & Perrin (2019) found a close relationship between externalization and binaural cues, especially interaural coherence, and reported that reverberation helped when it introduced useful interaural differences.

DOI: `10.1121/1.5128325`

Engineering consequence:

```text
do not optimize room energy alone
optimize direct + reflection cue relationship
```

This supports Omniphony's directional early-field architecture over a generic "more reverb" strategy.

---

## 4. Head motion evidence

Brimijoin, Boyd & Akeroyd (2013) reported substantially stronger externalization when virtual sources behaved as world-stable rather than moving with the listener's head.

DOI: `10.1371/journal.pone.0083068`

Hendrickx et al. (2017) likewise reported substantial externalization improvements from head-tracked motion, especially for frontal and rear sources using non-individualized HRTFs.

DOI: `10.1121/1.4978612`

Algazi & Duda (2008) also frame dynamic head cues as an important part of practical binaural reproduction.

DOI: `10.1109/ISM.2008.38`

Engineering consequence:

> **Head tracking is a high-value future externalization lever, but only after Current motion itself is sample-time stable and the static presentation is already trustworthy.**

Do not add head tracking as decoration around a callback-quantized or unstable scene.

---

## 5. Timbre, localization and diffuse behavior

Zaunschirm, Schörkhuber & Höldrich (2018) showed that HRIR time-alignment plus a diffuse-field constraint could improve coloration, localization accuracy and externalization together in binaural Ambisonic rendering.

DOI: `10.1121/1.5040489`

The important lesson for Omniphony is not to copy that renderer blindly. It is that:

```text
HRTF timing
+
diffuse-field behavior
+
spectral coloration
```

must be tested together.

A spatial candidate that improves apparent direction while creating moving spectral coloration is not a success.

---

## 6. Object-aware externalization

Landschoot & Jot (2023) review externalization methods for object-based binaural rendering and focus on the persistent difficulty of frontal objects appearing near or inside the head even when several common mitigating cues are present.

DOI: `10.1121/10.0018389`

This reinforces a useful product distinction:

```text
object direction
≠ object externalization
```

The scene may know where an object belongs while the binaural renderer still fails to make that position perceptually convincing.

---

## 7. Why Current keeps direct and room energy separate

Research and listening both argue against collapsing every spatial job into one wet field.

```text
DIRECT
identity / source direction / center authority

EARLY FIELD
externalization / geometry / first-order room cues

LATE FIELD
envelopment / closure / decay
```

Current therefore keeps the protected master direct, adds spatial support separately, and uses a bounded early/late environment around the support branch.

This architecture makes falsification easier. If front externalization is weak, the experiment can change early cues without simultaneously rewriting bass, direct center and late decay.

---

## 8. Why the 17-lane scene and 22-direction shell are separate

The canonical scene answers:

> **What semantic positions exist, and what is the provenance of the material assigned to them?**

The shell answers:

> **What rendering geometry should the binaural engine use to turn that state into a continuous headphone world?**

```text
8.1.4.4 scene
17 semantic lanes
AUTHORED / DERIVED / EMPTY
        ↓
22-direction shell
rendering lattice
        ↓
HRTF / ITD / room
```

Research on higher spatial resolution can inform render-lattice experiments without forcing a change in scene semantics.

---

## 9. Stereo inference remains bounded

Stereo evidence can estimate useful presentation state from:

- pan and level relation;
- complex M/S structure;
- phase/coherence;
- directness/diffuseness;
- temporal persistence;
- transient behavior;
- spectral region.

It cannot prove a hidden authored 3D mix.

Therefore:

```text
stereo evidence
→ permission / confidence
→ bounded DERIVED support
```

not:

```text
stereo evidence
→ recovered object truth
```

Future libaural analysis may improve evidence quality. It does not change this provenance law.

---

## 10. Current high-value experiments

### Front / center externalization

Test whether front scale can grow while the protected center becomes clearer and remains stable.

Measure/listen for:

- frontal externalization;
- front/back confusion;
- center solidity;
- spectral coloration;
- late-field masking;
- matched-loudness preference.

### Head-tracked world stability

Only after the static front is stable:

```text
same scene
head fixed vs head-tracked
→ externalization / localization / fatigue comparison
```

### HRTF personalization

Compare generic versus selected/personalized HRTFs without allowing loudness or EQ mismatch to dominate the result.

### Interaural coherence shaping

Use only when an isolated externalization or late-field problem justifies it. Do not decorrelate direct music globally.

### Source extent

Test `BroadSource` behavior independently from reverb and diffuse-field behavior.

---

## 11. Required objective gates

Each perceptual experiment should carry objective evidence where applicable:

- matched level;
- peak and RMS;
- crest factor;
- headroom/clipping;
- frequency response / spectral residual;
- ITD / interaural lag;
- ILD where relevant;
- interaural coherence where relevant;
- onset/transient timing;
- bass timing/coherence;
- block-size invariance;
- motion continuity;
- non-finite detection.

Objective metrics do not replace listening. They reveal what a preference result may have secretly purchased.

---

## 12. Listening protocol

A useful listening comparison should isolate one question whenever possible.

```text
same source
same loudness
same route
one candidate mechanism changed
        ↓
preference + failure description
```

Important subjective dimensions:

- externalization;
- front/back;
- elevation;
- width;
- depth;
- envelopment;
- source body;
- center stability;
- transient precision;
- bass authority;
- timbral naturalness;
- fatigue.

A result such as "slightly better" is valid evidence if the route and level are controlled, but it should not be inflated into a universal psychoacoustic law.

---

## 13. Research adoption rule

```text
paper / implementation suggests mechanism
        ↓
map it to a specific Omniphony obligation
        ↓
implement the smallest reversible candidate
        ↓
objective checks
        ↓
controlled listening
        ↓
retain, revise or reject
```

Negative results stay useful. They prevent the same attractive dead end from returning under a new name.

---

## 14. Protected baseline law

> **The existing Omniphony spatial character is the reference to improve, not debris to clear away for a more academic renderer.**

Research may strengthen the model. It does not automatically outrank a perceptually successful mechanism already earned by listening and validation.
