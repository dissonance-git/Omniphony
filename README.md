# Omniphony for Headphones

> **Private master project plan and canonical re-entry surface.**
>
> If conversational context disappears, recover the project from this README,
> recent `main` history, and the supporting documents under `docs/` before
> inventing a new direction.

Omniphony for Headphones is a fork of [`mgth/Omniphony`](https://github.com/mgth/Omniphony) being turned into a **platform-agnostic, always-on headphone spatial processor**, with Windows as the first product host.

The central architectural law is:

> **Use the inherited Omniphony renderer as the spatial core as much as possible. Preserve source truth, infer safe presentation evidence, feed the renderer better material, and add our own mechanism only when the inherited renderer genuinely does not own that job.**

The perceptual north star is:

> **The headphones should perceptually disappear and the listener should stand inside the largest coherent sphere the recording can support without losing clarity, punch, energy or identity.**

The target is not merely wider stereo and not merely a HeSuVi clone. It is closer to an open-source, modern Sony-360-like headphone world that works with ordinary stereo, native surround, games, movies and richer spatial sources without requiring specially authored music.

---

# 0. Current sound checkpoint

This section outranks older experiment descriptions elsewhere in the repository.

## 0.1 What failed first

Generic full-wet stereo virtual-speaker rendering damaged finished music:

```text
tinny / cheap-phone tone
bass and body loss
smaller useful stereo image
audible hallway / room coloration
less definition than bypass
```

That established the first hard rule:

> **A finished stereo master is authoritative. Do not delete it merely to spatialize it.**

## 0.2 Protected-master architecture succeeded

The current music path keeps the mastered stereo waveform explicitly present and adds separate derived support around it.

The reliable fidelity floor became:

```text
raw clarity retained
center remains solid
stereo identity remains intact
transients remain sharp
music still sounds like the same finished recording
```

P0.6 then established the first major perceptual win:

```text
real height exists
bubble is substantially larger
sound is beginning to beat HeSuVi
clarity remains unusually strong
```

The remaining job is no longer “make it spatial.” It is:

> **Push width, depth, height, rear impact, physical body and bypass collapse to their clean perceptual frontier without allowing ON to give anything important back to OFF.**

## 0.3 Current listener weighting

Physical listening says bubble scale is not primarily a side-width problem.

```text
FRONT DISTANCE   primary
REAR DISTANCE    primary and impactful, but must not dominate
HEIGHT DISTANCE  primary and currently the most promising frontier
SIDE WIDTH       very important for balance and continuity
SIDE DISTANCE    useful, but secondary to front/back/height reach
```

The target is therefore a **wide, depth-led sphere**.

Sides stay broad. Front/back/height supply most of the felt radial distance.

## 0.4 New low-end invariant

A later P0.7 candidate revealed that **OFF still had more low-end power and drum energy than ON**.

That is now a hard failure, not a preference:

> **Omniphony ON must match or exceed OFF in bass pressure, kick weight, drum impact and physical musical energy.**

The bubble is not allowed to be purchased with a weaker rhythm section.

---

# 1. P0.7: perceptual-frontier build

P0.7 is an **edge-seeking native-renderer experiment**.

Development law:

> **Increase acoustic volume until the next increment causes the first protected perceptual invariant to fail. The last clean state before that failure is the current frontier.**

This is the expansion-side dual of a Helix-style compression frontier: keep moving until the next operation destroys something load-bearing.

Failure signals include:

```text
clarity loss
source-image blur
center instability
transient softening
bass timing loss
bass/drum energy below OFF
high-frequency gain breathing / pumping
inside-head collapse
front/rear confusion
obvious reverb / hallway coloration
fatigue
microdetail loss
```

The protected master is the safety floor. The spatial scene can therefore be pushed aggressively and then pruned by listening.

---

# 2. Current native Omniphony frontier

Current `stereo-field-prototype.yaml`:

```text
HRTF                 SAF/KEMAR
analytic ITD          active
unit_scale_m          6.0
reflection room       13.5 m wide × 21 m deep × 10 m high
reflection level      0.40
late field level      0.04
RT60                  0.18 s
predelay              26 ms
air absorption        support only
support auto-gain     OFF
```

The upstream demo remains useful as a known working reference at roughly:

```text
unit_scale_m          3.0
reflection level      0.4
late field level      0.2
RT60                  0.3 s
```

The current frontier deliberately does **not** make everything wetter.

```text
MORE
metric distance
front distance
rear distance
vertical distance
first-order reflection propagation
room dimensions

LESS
late-room energy
long-tail blur
```

The main experiment is increasingly:

> **Can height and first-order distance cues grow the world faster than they grow coloration?**

---

# 3. Current support geometry

Logical 7.1.4 roles:

```text
L/R         front / front-side extent
C           silent
LFE         silent
Ls/Rs       maximum-width lateral continuity
Lb/Rb       impactful rear depth
Tfl/Tfr     dominant upper-front canopy
Tbl/Tbr     deep upper-rear closure
```

Current geometry intention:

```text
                    FAR / VERY HIGH
              .----------------------.
           .-'                        '-.
         .'         upper canopy         '.
        /                                  \
       /              FAR FRONT             \
      |                                      |
 WIDE |                 YOU                  | WIDE
 SIDE |                                      | SIDE
      |                                      |
       \               FAR REAR             /
        '.                                .'
           '-.                        .-'
              '----------------------'
```

Rear impact was intentionally restored after one candidate became too polite behind the listener.

Rear depth is desirable. **Rear gravity is not.**

Height is now pushed principally with **native Cartesian/metric geometry**, not by simply making treble louder.

---

# 4. Protected music foundation

The Noire X EQ is headphone/listener correction, not the spatial effect. It must remain conceptually separate.

Physical comparison with HeSuVi and later ON/OFF testing established that Omniphony needs its own coherent musical foundation role.

Current `music_foundation.rs` tuning:

```text
~85 Hz low shelf       +2.30 dB   pressure / mass
~240 Hz broad peak     +1.20 dB   kick / body
~800 Hz broad peak     +0.50 dB   density
~4.5 kHz high shelf    -0.35 dB   slight presence relaxation
```

Properties:

```text
no compressor
no limiter
no saturation
no dynamics-dependent loudness
no fake LFE
no HRTF-rendered bass foundation
same topology left/right
master remains explicitly present
foundation is additive only
```

This layer exists because physical body is a **music-presentation job**, not a room-rendering job.

The goal is not “more bass.” It is:

```text
pressure
kick weight
bass authority
drum body
low-mid density
physicality
```

without boom, timing loss or spatialized sub-bass.

---

# 5. Frequency-evidence music path

Active architecture:

```text
FINISHED STEREO MASTER
        │
        ├─────────────────────────────→ protected direct master
        │
        ├→ coherent foundation processor
        │      └→ additive body delta
        │
        └→ 1024-sample FFT ANALYSIS ONLY
              │
              ├→ L/R magnitude
              ├→ L/R phase
              ├→ true complex M/S
              ├→ pan / coherence
              ├→ directness / diffuseness
              └→ persistence / stability
                        │
                        ▼
                 scene inference
                        │
          ┌─────────────┼─────────────┐
          │             │             │
        BROAD        LATERAL       DIFFUSE
          │             │             │
          └──── causal multiband extraction
                        │
                        ▼
                derived 7.1.4 field
                        │
                        ▼
               OMNIPHONY CORE
          HRTF / ITD / distance / room
                        │
                        ▼
                 binaural support
                        │
protected master + foundation + support
                        │
                        ▼
                fixed linear headroom
                        │
                        ▼
                    headphones
```

The FFT is analysis-only. It does not STFT-resynthesize the master.

Current support bands:

```text
<320 Hz          protected master/foundation only
320–1200 Hz      restrained spatial body
1200–5000 Hz     full spatial support
5000 Hz–Nyquist  strong height permission, statically trimmed and slow-moving
```

Raising the support floor from ~220 Hz to **320 Hz** is deliberate. Physical listening showed that letting too much kick/snare/bass-body information enter the room/HRTF field could reduce perceived punch even though the dry master remained present.

The 320–1200 Hz field remains for continuity, but at reduced support strength.

---

# 6. High-frequency stability law

A frontier candidate exposed a new defect:

> **some high-pitched material could swell in level like a volume slider and then fall back.**

This is not the same as a fixed bright frequency response.

Two mechanisms were identified.

## 6.1 Fast top-band scene motion

The >5 kHz band had the strongest height prior while opening/closing at roughly the same speed as body bands. That could make changes in spatial confidence sound like changes in treble gain.

Current repair:

```text
high-band scene motion    substantially slower
high-band height prior    still strong
high-band support scale   fixed at 0.72
```

A previous experimental per-sample energy normalizer was removed because it would itself have been a gain-modulation mechanism.

Law:

> **Spatial analysis may change where high-frequency evidence is presented. It must not become a fast treble-volume envelope.**

## 6.2 Native renderer auto-gain was the wrong tool for a support branch

The inherited Omniphony binaural auto-gain is appropriate when Omniphony owns the **final output bus**. When binaural output exceeds full scale, it lowers renderer master gain.

In the protected-master architecture, however, Omniphony renders only the **additive support branch**.

That means a spatially hot event could reduce the support branch while leaving the direct master unchanged, which can sound like the spatial/high-frequency world suddenly sliding downward.

Therefore:

```text
support renderer auto_gain = OFF
```

Any future final-bus peak protection belongs after master + foundation + support have been combined, not inside one additive branch.

---

# 7. Clean summing and output level

The old Windows combiner used:

```text
base + clamp(spatial support to instantaneous remaining +/-1.0 headroom)
```

That was nonlinear and could shave only portions of the spatial waveform.

It was removed.

Current combiner:

```text
protected master
+ coherent foundation delta
+ rendered spatial support
→ one fixed linear gain
```

No sample-dependent support clamp, soft-knee or limiter is used in the host combiner.

The first clean-summing experiment used:

```text
0.45 linear
≈ -6.94 dB
```

Physical listening correctly rejected that as too quiet.

Current output gain:

```text
0.72 linear
≈ -2.85 dB
```

This restores roughly **4.1 dB** while keeping useful fixed headroom.

Future peak handling must not recreate audible pumping or waveform shaving.

---

# 8. Cosmic Cove Galaxy

`Cosmic Cove Galaxy` remains a known tiny clipping/grain stress case.

Observed:

```text
local playback: detectable
YouTube playback: detectable
Omniphony OFF: absent or much less apparent
almost every other song tested: not noticed
```

The obvious host-side nonlinear support clamp has already been removed and the tiny texture remains.

Current policy:

> **Do not let one isolated song hold the spatial frontier hostage.**

Until the same artifact appears clearly on another track or objective capture ties it to a repeatable Omniphony failure, Cosmic Cove is a **known stress oddity, not an active release blocker**.

If the artifact recurs elsewhere, reopen it immediately.

---

# 9. Architectural invariant

This is not a ground-up replacement for Omniphony.

```text
SOURCE
  │
  ├─ stereo master
  ├─ channel beds
  ├─ objects
  └─ HOA / richer future formats
  │
  ▼
SOURCE PRESENTATION
preserve truth + infer only what is safe
  │
  ▼
UPSTREAM-DERIVED OMNIPHONY CORE
HRTF / HRIR
ITD
geometry
distance
reflections
room field
head pose
object / bed handling
SOFA-capable HRTF path
  │
  ▼
BINAURAL STEREO
  │
  ▼
ordinary headphones
```

Engineering law:

> **If Omniphony already has a mechanism for a spatial job, use and improve that mechanism before writing a second implementation beside it.**

Native-first examples:

```text
distance          → Omniphony
directional HRTF  → Omniphony
ITD               → Omniphony
reflections       → Omniphony
late room         → Omniphony
head tracking     → Omniphony
SOFA HRTFs        → Omniphony
```

But mechanism ownership must match topology. A feature designed for a final output bus is not automatically correct on an additive support branch.

Our strongest custom ownership remains:

```text
source preservation
stereo evidence
confidence / permission laws
presentation mapping
foundation/body correction
final host summing/routing
validation
```

---

# 10. Hard fidelity law

> **Dimension may not be purchased by damaging the music.**

At matched practical listening level, bypass should ideally collapse:

```text
acoustic volume
front/back distance
height
radial depth
source extent
ambient continuity
listener envelopment
```

Bypass must **not** restore:

```text
clarity
transient punch
bass pressure
kick weight
drum impact
bass timing
overall musical energy
timbral identity
vocal solidity
rhythmic precision
microdetail
dynamics
stereo definition
stable tonal level
comfort
```

The strongest current formulation is:

> **OFF may collapse the world. It may not bring the rhythm section back to life.**

Desired reaction:

> **“The world collapsed.”**

Not:

> **“The music came back.”**

---

# 11. Influence ledger: current interpretation

`docs/influence-ledger.md` remains durable research memory.

## Promote through Omniphony itself

**Upstream Omniphony** remains highest priority: distance, HRTF/ITD, first-order reflections, short room field, air-distance cues, head pose and SOFA-capable HRTFs.

**3D Tune-In** and **Google Open Binaural Renderer** reinforce role separation:

```text
DIRECT       protected stereo master
AMBIENT      derived broad/lateral/diffuse support
REVERBERANT  native Omniphony room field
```

**Cavern** is especially relevant because physical listening says radial distance is one of the strongest bubble mechanisms.

**Foobar-for-Home-Theater** contributed the useful height law: preserve authoritative material, synthesize missing dimension, favor top-front over top-back, and do not let rear accumulation become the definition of immersion.

## Promote as analysis / validation

**FreeSurround / Real3D** remain useful for amplitude+phase evidence, not as authored multichannel truth.

**Trifield** reinforces center authority.

**LCC** reinforces preservation of useful source ITD/ILD.

**Halo / Penteo** become more important as source-safety / reversibility references as the effect becomes dramatic.

FIR/convolution research remains useful for ringing, phase, group delay and transition validation.

## Viable but not free

**Source extent** remains desirable, but the current binaural branch does not simply consume generic VBAP object-spread machinery. Implement extent inside the inherited binaural core only when it solves a demonstrated problem.

**Ambisonics / IEM / Steam Audio / Resonance Audio** remain research references for continuity and diffuse fields. Do not graft another binaural renderer over Omniphony.

**SOFA / HRTF personalization** becomes increasingly important as the bubble grows larger and cleaner because HRTF mismatch becomes an increasingly obvious ceiling.

## Still rejected as defaults

```text
wholesale full-wet stereo HRTF replacement
strong generic crossfeed
fake LFE from stereo
high-frequency = height shortcut
low-frequency = floor shortcut
long obvious reverb wash
rear gain as the definition of spatiality
external binaural engine grafts
semantic source wandering / live remix behavior
```

---

# 12. Universal source-truth law

Inference decreases as source truth increases.

```text
STEREO
→ preserve finished master
→ add validated support

5.1 / 7.1
→ preserve real directional channels

5.1.2 / 7.1.4
→ preserve authored elevation

OBJECT AUDIO
→ render supplied positions directly

AMBISONICS / HOA
→ preserve supplied field

ALREADY-BINAURAL
→ avoid destructive double virtualization
```

More truth means less inference.

---

# 13. Concurrent-stream law

Channel layout belongs to a logical stream, not Omniphony globally.

```text
foobar       stereo
browser      stereo
Overwatch    native surround / spatial
voice chat   mono / stereo
future app   objects / richer metadata
```

Correct future model:

```text
Stream A { layout = stereo }
Stream B { layout = 7.1 }
Stream C { layout = mono }
Object D { spatial metadata }
        ↓
shared Omniphony output timeline
        ↓
binaural stereo
```

A game must not reinterpret a playing song, and a song must not flatten a game's authored surround.

---

# 14. Windows host

Windows is the first host, not the product identity.

Current prototype:

```text
Windows application audio
→ self-excluding process loopback
→ portable stereo presentation
→ Omniphony renderer
→ physical FiiO/headphones
```

Clean development setup:

```text
Hi-Fi Cable speaker config = Stereo / 2.0
foobar upmix = OFF
HeSuVi = OFF
ASIO Bridge forwarding = OFF
Omniphony = only physical path to headphones
```

Mature experience:

```text
install
→ Omniphony ON
→ play anything normally
```

The host still assumes **48 kHz float**. 48 kHz itself is not the current grain hypothesis, but mature routing should negotiate/report the actual endpoint/mix format and avoid unnecessary conversion.

---

# 15. HeSuVi relationship

HeSuVi remains an incumbent/perceptual oracle until Omniphony repeatedly earns replacement.

Useful incumbent functions:

```text
large bubble
rear presence
subjective density
low-end pressure / body
strong practical level
```

Do not copy its topology. Reproduce or surpass the useful percept.

Current target is now clearer:

> **HeSuVi's physical energy + substantially larger and cleaner Omniphony geometry + protected stereo definition.**

Migration law remains:

> **Disable before uninstall.**

---

# 16. Validation lanes

## Fidelity

Track independently:

```text
clarity
center authority
transients
bass timing
bass pressure/body
kick weight
drum impact
overall energy
stable high-frequency level
stereo definition
microdetail
timbral identity
dynamics
fatigue
```

## Spatial world

Track independently:

```text
front distance
rear distance
rear impact
front/rear balance
height distance
height continuity
side width
side continuity
radial depth
source extent
ambient continuity
bypass collapse
```

## Frontier test

```text
current clean frontier
→ increase native scale / geometry
→ matched listening
→ did acoustic volume increase?
→ did ANY protected invariant regress?

NO regression
→ frontier advances

YES regression
→ crossed boundary
→ prune the responsible mechanism
```

The target is the **largest coherent presentation before blur**, not a conventionally moderate preset.

---

# 17. Milestones

## W0
Upstream spatial reference established.

## P0
Native protected listening established.

## P0.1
Arbitrary live Windows audio established.

## P0.2
Clean stereo architecture established; full-wet stereo rejected.

## P0.3
Protected-master fidelity floor established.

## P0.4
Frequency-evidence field established.

## P0.5
Full-strength 7.1.4 support shell established; clean/spatial but rear-heavy and belt-like.

## P0.6
**Listening win:** real height, larger bubble, clarity retained, starting to exceed HeSuVi.

## P0.7
**Active frontier experiment:**

```text
6.0 m native metric scale
13.5 × 21 × 10 m first-order reflection room
0.40 reflection level
0.04 / 0.18 s tiny late field
wide sides
far front
restored rear impact
very large upper canopy
protected <320 Hz foundation
restrained 320–1200 Hz support
slow/static top-band support behavior
support auto-gain OFF
0.72 fixed final linear gain
stronger coherent pressure/body layer
```

## P1
Excellent everyday stereo music:

```text
huge coherent 360° sphere
far front
real rear depth without rear dominance
convincing overhead volume
wide continuous sides
near / mid / far layering
source extent without smear
ambient continuity
bass/drums at least as powerful as bypass
raw-master clarity intact
bypass collapses the world, not restores the song
```

## P2
Owned Windows routing.

## P3
Native surround / richer spatial sources.

## P4
Deeper stereo presentation: binaural-native source extent, diffuse continuity, lower hemisphere, source-safe collapse metrics, optional HRTF fitting.

## P5
Personalization: headphone correction, HRTF selection/import, listener fitting, head tracking, advanced controls.

---

# 18. Product anti-goals

- Do not replace inherited Omniphony merely because another renderer is fashionable.
- Do not discard the finished stereo master.
- Do not make rear gain synonymous with immersion.
- Do not make sides narrow merely because radial distance is front/back/height-led.
- Do not create fake LFE from stereo.
- Do not spatialize the bass foundation merely to make it impressive.
- Do not let ON have less bass/drum authority than OFF.
- Do not turn treble into height by register alone.
- Do not allow high-band spatial confidence to become an audible volume envelope.
- Do not use support-branch auto-gain as final-bus protection.
- Do not turn bass into floor placement by register alone.
- Do not use long reverb as a shortcut to a large bubble.
- Do not solve Cosmic Cove by globally shrinking the sphere unless the defect generalizes.
- Do not reintroduce sample-dependent support clipping.
- Do not copy `noire_x.txt` into the music enhancer; headphone correction and presentation are separate.
- Do not let ON become less clear than OFF.
- Do not erase useful source ITD/ILD.
- Do not insert Ambisonics unless it solves an actual continuity problem.
- Do not graft Steam Audio, Resonance Audio or Cavern over Omniphony.
- Do not let semantic analysis make sources audibly wander.
- Do not let Windows APIs contaminate the portable core.
- Do not uninstall HeSuVi until Omniphony repeatedly earns it.
- Do not let a spectacular five-second effect outrank long-session fidelity.

---

# 19. Current commit checkpoint

Recent load-bearing changes:

```text
90948f1  remove sample-wise host clipping; linear summing
0ad14f3  portable coherent foundation/body processor
047f188  wire foundation into live Windows path
a71eb55  push native bubble to 5.25 m frontier
e3ba365  restore practical output level to 0.72 linear
0f339cc  push native height/rear frontier to 6.0 m / 13.5×21×10 m
4a07a56  strengthen protected foundation/body
6870115  raise spatial floor to 320 Hz; protect punch; static high-band trim
f72a717  disable native auto-gain on additive support renderer
```

Next packaged listening should answer:

```text
1. Does ON now equal or exceed OFF in bass pressure?
2. Is kick/drum impact at least as strong as OFF?
3. Is overall practical level acceptable again?
4. Is the high-frequency "volume slider" behavior gone?
5. Is rear impact restored without rear gravity?
6. Is the canopy clearly higher/farther?
7. Are sides still extremely wide?
8. Is front still far and externalized?
9. Did the stronger foundation become boomy or slow?
10. Did any new blur appear?
```

If #1-8 improve and #9-10 remain **no**, the frontier advances again, with height still the leading candidate for the next expansion.

---

# 20. Re-entry checkpoint

If context is lost, recover these facts first:

```text
1. upstream-derived Omniphony is the spatial heart
2. use native HRTF/ITD/distance/reflection/room mechanisms before substitutes
3. finished stereo remains explicitly protected
4. stereo evidence decides what may enter the support field
5. Noire X EQ is headphone correction, not the spatial effect
6. P0.6 produced real height and began beating HeSuVi while staying clear
7. P0.7 is an edge-seeking spatial frontier search
8. front, rear and height distance create most of the desired bubble scale
9. sides must remain wide and continuous
10. rear depth is desirable; rear gravity is not
11. current frontier uses 6.0 m scale and 13.5×21×10 m first-order room geometry
12. late room is intentionally tiny as geometry grows
13. height is the most promising remaining expansion axis
14. host sample-wise support clipping has been removed
15. output gain is 0.72 linear, not the rejected 0.45 experiment
16. support renderer auto-gain is OFF because this is an additive branch
17. high-band support uses slow controls and fixed scale, not fast/sample-wise gain normalization
18. spatial support begins at 320 Hz; lower foundation stays direct
19. ON must equal/exceed OFF in bass/drum energy
20. Cosmic Cove is an isolated stress oddity until the artifact generalizes
21. 48 kHz itself is not the current grain hypothesis
22. source truth always outranks inference
23. richer formats receive less inference than ordinary stereo
24. concurrent streams keep independent layouts
25. Windows is the first host, not the core
26. bypass should collapse the world, not restore the music
27. keep pushing useful native Omniphony capability until the first protected invariant breaks
28. the last clean state before blur is the current perceptual frontier
```

Supporting documents:

- `docs/frequency-evidence-music-path.md`
- `docs/music-presentation-contract.md`
- `docs/headphone-rendering-research.md`
- `docs/influence-ledger.md`
- `docs/windows-integration-research.md`

This README is the authoritative current project state when older lower-level documents disagree.
