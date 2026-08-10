# Omniphony practical rendering plan

> **Scope:** renderer experiments and the path to a real Windows listening build.
>
> The root `README.md` owns product intent and roadmap priority. This document must not grow into a second master plan.

The central engineering fact is simple:

> **Upstream Omniphony already provides a strong perceptual starting point. Renderer research exists to preserve and selectively improve that sound while the fork becomes a native Windows replacement for the current HeSuVi pipeline.**

General artificial-hearing research belongs in `libaural`, but Omniphony is not blocked on libaural and does not need to rebuild itself around it.

---

## 1. Practical target

```text
ordinary Windows stereo
        ↓
Omniphony realtime path
        ↓
stable externalized 360° binaural world
        ↓
headphones
```

while preserving:

- clarity;
- timbre;
- bass timing and weight;
- transients;
- dynamics;
- vocal/instrument identity;
- stereo relationships that matter musically;
- musical hierarchy;
- long-session comfort.

The current foobar + VB-Audio + HeSuVi + FiiO ASIO route remains available until Omniphony clearly earns replacement.

No cold-turkey migration.

---

## 2. Protected starting sound

The upstream hosted headphone demo is the perceptual ancestor.

Local control:

```text
omniphony-renderer/assets/binaural-baselines/upstream-demo-reference.yaml
```

Published ingredients approximated by that control:

```text
stock renderer defaults
+ SAF/KEMAR HRTF
+ early reflections
+ late reverb disabled
```

This is more important than any richer fork room preset.

A candidate that adds spatial drama but loses the upstream sense of coherent acoustic volume does not graduate.

---

## 3. Current incumbent is a second oracle

The current daily chain proves that a large, enjoyable headphone bubble is already possible for the target listener.

Current high-level route:

```text
foobar DSP
→ 5.1-side upmix
→ VB-Audio multichannel transport
→ HeSuVi / DTS Virtual:X HRIR
→ FiiO ASIO
→ K7
→ Noire X
```

This chain is evidence, not a template.

Omniphony should reproduce or surpass the useful perceptual functions without permanently reproducing the multistage topology.

---

## 4. The 360° target

The goal is not:

```text
stereo
+ width
+ rear reverb
```

Useful spatial jobs remain distinct:

```text
DIRECT OBJECT
specific source-like identity

BROAD SOURCE
coherent source-like identity with meaningful extent

DIFFUSE FIELD
musical/ambient energy better represented as a distribution

ROOM FIELD
shared acoustic context, reflections and late energy
```

And:

```text
rear direct object
≠ rear reflection
≠ diffuse rear field
```

The renderer may use front, side, rear, elevation and radial/depth cues when they produce a more convincing, stable presentation.

Do not force a complete object ontology into the first Windows build. The distinctions are there to avoid conflating perceptual jobs, not to create an implementation prerequisite maze.

---

## 5. Fidelity laws

### Preserve music before maximizing effect

A wider/deeper scene fails if matched bypass restores:

- clearer transients;
- better bass timing;
- stronger center authority;
- more natural timbre;
- more intelligible mix relationships;
- less phasey coloration;
- less fatigue.

### Cue agreement

ITD, ILD, HRTF spectrum, early reflections, distance, diffuseness and motion should describe compatible geometry.

Conflicting cues often create blur, instability or inside-head localization.

### Original master remains authoritative

Source separation, scene estimates and semantic models can provide control evidence. They do not automatically replace the master waveform or authorize a remix.

### Better hardware should expose more scene, not more artifact

The K7 + Noire X reference is useful because it will expose false spaciousness and smeared detail quickly.

---

## 6. Inherited renderer path

The upstream renderer already contains valuable machinery.

Conceptual binaural route:

```text
source / object position
→ head-relative direction
→ azimuth / elevation / distance
→ per-ear timing / HRTF
→ early reflections
→ optional late room field
→ [L, R]
```

Retain mature behavior unless a test demonstrates a concrete limitation.

Important inherited/fork substrate includes:

- stateful binaural DSP;
- analytic ITD;
- measured/parametric/SOFA HRTF support;
- moving-filter crossfades;
- object position/size state;
- early image-source reflections;
- late FDN room field;
- known-scene layout/VBAP machinery;
- deterministic fixtures;
- headless engine/FFI boundaries.

---

## 7. Renderer candidates, not mandatory roadmap items

These are candidate improvement lanes. They should be pulled forward only when listening or correctness work exposes the corresponding weakness.

### Sample-time position / HRTF motion

The inherited path still has block-start position publication that can quantize movement.

Desired direction:

```text
scene trajectory
→ authoritative sample-time position segment
→ HRTF motion follows that segment
```

Useful when motion or callback-size changes expose audible instability.

### Directional early reflections

Preferred geometry:

```text
image source
→ reflection direction
→ delay / attenuation
→ reflection-specific binaural cues
→ ears
```

Acceptance:

- better externalization/source body where intended;
- no echo/doubling;
- no obvious comb coloration;
- transients remain intact;
- stable localization;
- bounded CPU;
- upstream reference remains available.

### Source extent / BroadSource

The scene already carries size/extent information in places. The binaural path should not collapse every meaningful source to a point if listening demonstrates a benefit from real extent.

Do not add spread by indiscriminately decorrelating direct material.

### DiffuseField

A late FDN is a room field, not automatically a model of diffuse musical material.

If a genuine diffuse musical representation is needed, test it separately from room reverb.

### Bass / foundation protection

Do not buy spatial dimension by destabilizing the low-frequency groove floor.

---

## 8. Stereo inference

Ordinary stereo is the primary practical source.

Current fork evidence machinery includes inspectable measures for things such as:

- M/S relation;
- pan/lateral evidence;
- phase coherence;
- channel asymmetry;
- directness/diffuseness;
- persistence;
- lateral stability;
- foundation/bass protection.

This evidence should remain conservative.

It does not prove a frequency region is an instrument or reveal hidden authored rear coordinates.

As the product matures, richer evidence may come from local algorithms or from a bounded optional libaural projection.

Use whichever mechanism produces better, more stable decisions with less cost.

---

## 9. HeSuVi relationship

HeSuVi is an **end-to-end incumbent and perceptual oracle**, not the future architecture.

```text
CURRENT
stereo
→ foobar DSP / upmix
→ virtual multichannel route
→ HeSuVi DTS virtualization
→ headphones

TARGET
stereo
→ Omniphony
→ headphones
```

The current chain proves that meaningful behind-head and room-scale perception is desirable.

Omniphony should make that world more direct and coherent, not recreate the external upmix→virtualizer detour internally.

---

## 10. Known-scene tests

Known geometry remains valuable because it isolates renderer quality from stereo inference.

```text
KNOWN RICH SCENE
       │
       ├→ protected/reference binaural render
       │
       └→ fork candidate
```

Use these tests for:

- HRTF direction correctness;
- front/back/elevation;
- reflection geometry;
- source extent;
- motion continuity;
- room-field changes;
- callback-size invariance where relevant.

Do not infer product superiority from known-scene tests alone. Ordinary music and the incumbent chain remain separate gates.

---

## 11. Development order

This order is intentionally practical.

### W0 · Reproducible renderer baseline — CURRENTLY ESTABLISHED

- Windows x64 CI;
- deterministic file rendering;
- protected upstream-demo reference;
- fork room/dry controls;
- host-native path tests;
- green Windows Actions after the August 2026 repair.

### W1 · First coexisting native Windows listening lane — NEXT

- inspect/normalize current `host_audio`, `audio_input`, `audio_output` ownership;
- retain existing ASIO usefulness;
- add ordinary Windows output not dependent on ASIO;
- add practical Windows input/capture for normal playback;
- expose development enable/bypass and device selection;
- prove stable 48 kHz realtime playback on K7/Noire X;
- do not disturb the existing HeSuVi route.

### W2 · A/B harness

- matched-loudness switching where practical;
- stable test tracks/passages;
- upstream reference controls;
- incumbent-chain comparison;
- latency/underrun diagnostics;
- written listening dimensions.

### R1 · Fix the next actual renderer weakness

Only after W1/W2 make it easy to hear the renderer in context.

Potential work:

- sample-time motion;
- source extent;
- directional reflections;
- front/back/elevation robustness;
- room/direct separation;
- bass integrity.

### S1 · Small persistent stereo scene

- wire current stereo evidence into a bounded realtime scene;
- keep center/foundation safe;
- permit direct/broad/diffuse distinctions only where useful;
- prefer reversible behavior under uncertainty.

### P1 · Optional adaptive music presentation

- add artistic degrees of freedom one at a time;
- use local evidence first where sufficient;
- introduce libaural inputs only when they demonstrate an advantage;
- always keep the protected baseline route for attribution.

### C1 · Headphone/listener calibration

Later:

- per-device profile;
- optional headphone correction;
- HRTF selection/import;
- headroom management;
- deeper personalization only after core listening is strong.

---

## 12. Research trigger rule

Do not do another broad influence sweep by default.

Research starts from a concrete weakness:

```text
listening/test reveals weakness
→ formulate exact missing capability
→ search literature / GitHub / existing systems
→ isolate candidate mechanism
→ implement smallest test
→ measure + listen
```

This keeps Steam Audio, Dolby, ImmersiveFlow, psychoacoustics, libaural and other work available without letting them steer the product merely by existing.

---

## 13. Listening scorecard

Score dimensions separately:

```text
front externalization
rear discrimination
side precision
elevation
radial distance
apparent source width
listener envelopment
source extent
source separation
source stability
room presence / scale
ambient continuity
transient clarity
vocal/direct clarity
timbral fidelity
bass stability / groove
microdetail
dynamics
fatigue
bypass-collapse strength
```

The desired bypass result is a collapse in perceived acoustic volume, not discovery that bypass is cleaner, punchier, tighter, or more coherent.

---

## 14. Current practical north star

```text
play ordinary Windows music
        ↓
Omniphony keeps the good upstream spatial character
        ↓
native realtime path removes external plumbing
        ↓
front stays anchored where it should
sources can occupy convincing side/rear/depth/height space
room surrounds rather than smears
bass stays physical and timed
transients stay sharp
headphones stop feeling like the source
```

Then the old chain can be retired because Omniphony **won**, not because the project plan declared it obsolete.