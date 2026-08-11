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

> **The headphones should perceptually disappear and the listener should stand inside a huge, coherent sphere of the same finished recording.**

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

The reliable fidelity floor is:

```text
raw clarity retained
bass/body retained
center remains solid
stereo identity remains intact
transients remain sharp
music still sounds like the same finished recording
```

The support scene can therefore become aggressive spatially without requiring the master itself to be HRTF-rendered, reverberated or reconstructed.

## 0.3 P0.5 listening result

P0.5 expanded the frequency-evidence field into logical 7.1.4 at full derived-support strength.

Physical listening:

```text
clear as hell
clearly spatial
close to full HeSuVi replacement
but rear-heavy
bubble too small
field behaved more like a horizontal band inside/behind the head
front projection weak
height weak
```

A tiny ON-only grain was also discovered on **Cosmic Cove Galaxy**.

## 0.4 P0.6 listening result

P0.6 moved the field forward and upward while preserving the protected master.

Physical listening established a major step:

```text
real height now exists
bubble is larger
sound is starting to beat HeSuVi
clarity remains otherwise excellent
```

The remaining shape problem became more precise:

```text
still somewhat rear-heavy
more bubble size is desired
radial distance matters more than lateral wrap alone
```

The target weighting is now explicit:

```text
FRONT DISTANCE   primary bubble cue
REAR DISTANCE    primary bubble cue, but must not dominate
HEIGHT DISTANCE  primary bubble cue
SIDE WIDTH       very important for balance and continuity
SIDE DISTANCE    useful, but secondary to front/back/height reach
```

The intended world is therefore a **wide, depth-led sphere**, not a narrow front/back tunnel and not a side-heavy ring.

---

# 1. P0.7: perceptual-frontier build

P0.7 is now an **edge-seeking native-renderer experiment**.

The development rule is:

> **Increase acoustic volume until the next increment causes the first protected perceptual invariant to fail. The last clean state before that failure is the current frontier.**

The failure boundary is not merely clipping. Stop pushing when another increment causes any of:

```text
clarity loss
source-image blur
center instability
transient softening
bass timing loss
inside-head collapse
front/rear confusion
obvious reverb / hallway coloration
fatigue
microdetail loss
```

This is deliberately more aggressive than the early P0.2-P0.4 isolation phase. The protected master is now a stable safety floor, so the spatial scene can be pushed hard and then pruned by listening.

## 1.1 Current native Omniphony frontier

Current `stereo-field-prototype.yaml`:

```text
HRTF                 SAF/KEMAR
analytic ITD          active
unit_scale_m          5.25
reflection room       12 m wide × 18 m deep × 8 m high
reflection level      0.38
late field level      0.05
RT60                  0.20 s
predelay              24 ms
air absorption        support only
auto gain             active
support ceiling       -2 dBFS
```

The important trade is intentional:

```text
MORE
metric distance
front/rear/height reach
first-order reflection propagation
physical acoustic volume

LESS
late-room energy
long-tail blur
```

The upstream demo remains a useful known-spatial reference at roughly:

```text
unit_scale_m          3.0
reflection level      0.4
late field level      0.2
RT60                  0.3 s
```

P0.7 is therefore not simply making everything wetter. It is pushing Omniphony's **distance and first-order geometry** much harder while making its late field smaller.

## 1.2 Current support geometry

The logical 7.1.4 scene is deliberately anisotropic in perceptual role while remaining broad in all directions:

```text
front floor      full front diagonal, far
rear floor       almost full rear diagonal, slightly shorter than front
sides            maximum width, radially closer than diagonals
front height     longest / strongest canopy axis
rear height      deep upper closure, slightly shorter than upper-front
```

Approximate intention:

```text
                   FAR / HIGH
             .------------------.
          .-'                    '-.
        .'      upper canopy        '.
       /                              \
      /            FAR FRONT           \
     |                                  |
WIDE |               YOU                | WIDE
SIDE |                                  | SIDE
     |                                  |
      \             FAR REAR           /
       '.                            .'
          '-.                    .-'
             '------------------'
```

Rear distance is desirable. **Rear gravity is not.**

Sides must remain wide. They simply are not the primary radial-scale mechanism.

---

# 2. Grain defect and clean summing

`Cosmic Cove Galaxy` remains the known stress case for a tiny clipping-like grain with Omniphony ON.

Evidence:

```text
heard from local playback
heard when the same material is streamed from YouTube
not heard with Omniphony OFF
rare or inaudible on almost every other tested song
```

This makes local file corruption, one codec and one player-specific decode poor explanations.

The former Windows mixer performed:

```text
base + clamp(spatial support to instantaneous remaining +/-1.0 headroom)
```

That was nonlinear and could shave only the spatial waveform on hostile peaks.

That operation has now been removed.

Current host summing is:

```text
protected master
+ coherent foundation delta
+ full rendered spatial support
→ fixed linear output gain
```

No sample-dependent clipping, limiting or soft-knee operation occurs in the host combiner.

Current fixed output gain:

```text
0.45 linear
≈ -6.94 dB
```

Listening level is recovered downstream at the DAC/headphone amp rather than by shaving DSP peaks.

`Cosmic Cove Galaxy` must be rechecked on the next packaged build before the grain is considered solved.

---

# 3. HeSuVi low-end/body lesson

The imported HeSuVi setup clarified an important distinction.

`noire_x.txt` is the **Dan Clark Noire X headphone/listener EQ**, not a HeSuVi spatial effect preset. It already contains substantial low-end/body support and relaxed upper presence.

Therefore the observation that Omniphony still sounds somewhat brighter and lighter through bass/mids than the old HeSuVi chain cannot be dismissed as a different headphone EQ.

The incumbent chain also created physical density through multichannel matrixing + HRIR summation. Correlated low-frequency energy can reinforce across the virtual bed and produce **pressure/body**, not merely boom.

Do not copy that topology blindly.

Instead P0.7 adds a small portable **coherent music-foundation delta** outside the spatial scene.

Current foundation tuning:

```text
~85 Hz low shelf       +1.50 dB   pressure / mass
~240 Hz broad peak     +0.75 dB   body
~800 Hz broad peak     +0.35 dB   density
~4.5 kHz high shelf    -0.45 dB   slight presence relaxation
```

Properties:

```text
no compressor
no limiter
no saturation
no dynamics-dependent gain
no fake LFE
no HRTF-rendered bass foundation
same topology left/right
master remains explicitly present
foundation is only an additive delta
```

Portable implementation:

`renderer/src/music_foundation.rs`

This is **not a second copy of `noire_x.txt`**. Headphone correction and music presentation remain separate jobs.

---

# 4. Architectural invariant

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
renderer clipping protection → Omniphony
```

Our strongest custom ownership is:

```text
source preservation
stereo evidence
confidence / permission laws
presentation mapping
foundation/body correction where renderer has no such music role
platform routing
validation
```

---

# 5. Stereo music architecture

The active source path is:

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
protected master + body delta + support
                        │
                        ▼
                fixed linear headroom
                        │
                        ▼
                    headphones
```

The FFT is analysis-only. It does not STFT-resynthesize the master.

Current spatial support bands:

```text
220–1200 Hz
1200–5000 Hz
5000 Hz–Nyquist
```

Band 0 below about 220 Hz remains out of the inferred spatial field.

Current logical roles:

```text
L/R         front / front-side broad extent
C           silent
LFE         silent
Ls/Rs       wide lateral continuity
Lb/Rb       rear depth
Tfl/Tfr     dominant upper-front canopy
Tbl/Tbr     upper-rear closure
```

Durable evidence laws:

- coherent frontal anchors suppress aggressive field promotion;
- low-frequency foundation is not fake LFE;
- high frequency does not automatically mean height;
- low frequency does not automatically mean floor;
- stereo analysis supplies presentation permission, not recovered authored coordinates;
- more source truth means less inference.

Portable ownership:

```text
renderer/src/stereo_inference.rs
renderer/src/scene_inference.rs
renderer/src/music_field.rs
renderer/src/music_foundation.rs
```

Windows owns transport, not hearing logic.

---

# 6. Hard fidelity law

> **Dimension may not be purchased by damaging the music.**

Bypass should ideally collapse:

```text
acoustic volume
front/back distance
height
radial depth
source extent
ambient continuity
listener envelopment
```

Bypass must not restore:

```text
clarity
transient punch
bass timing / weight
timbral identity
vocal solidity
rhythmic precision
microdetail
dynamics
stereo definition
comfort
```

Desired reaction:

> **“The world collapsed.”**

Not:

> **“The music came back.”**

The new foundation layer is therefore provisional until listening confirms that its extra pressure/body feels like a positive presentation improvement rather than tonal repair for damage elsewhere.

---

# 7. Influence ledger: current interpretation

`docs/influence-ledger.md` remains durable research memory.

## Promote through Omniphony itself

**Upstream Omniphony** remains highest priority: distance, HRTF/ITD, first-order reflections, short room field, air-distance cues, head pose and SOFA-capable HRTFs.

**3D Tune-In** and **Google Open Binaural Renderer** reinforce the current role split:

```text
DIRECT       protected stereo master
AMBIENT      derived broad/lateral/diffuse support
REVERBERANT  native Omniphony room field
```

**Cavern** is especially relevant now because physical listening says radial distance is one of the strongest bubble mechanisms.

**Foobar-for-Home-Theater** contributed the useful P0.6 topology: preserve authoritative material, synthesize only missing dimension, favor top-front over top-back, and avoid unnecessary rear concentration.

## Promote as analysis / validation

**FreeSurround / Real3D** remain useful for amplitude+phase evidence, not as authored multichannel truth.

**Trifield** reinforces center authority.

**LCC** reinforces preservation of useful source ITD/ILD.

**Halo / Penteo** are increasingly important as source-safety / collapse benchmarks as the presentation becomes more dramatic.

FIR/convolution research remains useful for ringing, phase, group-delay and transition validation.

## Viable but not free

**Source extent** is desirable, but the current binaural branch does not simply consume the generic VBAP object-spread machinery. Implement extent inside the inherited binaural core if listening proves the need.

**Ambisonics / IEM / Steam Audio / Resonance Audio** remain useful research references for field continuity. Do not graft another binaural engine over Omniphony.

**SOFA / HRTF personalization** becomes more important as the bubble gets larger and cleaner, because HRTF mismatch becomes a more obvious remaining ceiling.

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

# 8. Universal source-truth law

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
→ preserve the supplied field

ALREADY-BINAURAL
→ avoid destructive double virtualization
```

More truth means less inference.

---

# 9. Concurrent-stream law

Channel layout belongs to a logical stream, not Omniphony globally.

```text
foobar       stereo
browser       stereo
Overwatch     native surround / spatial
voice chat    mono / stereo
future app    objects / richer metadata
```

Correct future model:

```text
Stream A { layout = stereo }
Stream B { layout = 7.1 }
Stream C { layout = mono }
Object stream D { spatial metadata }
        ↓
shared Omniphony output timeline
        ↓
binaural stereo
```

A game must not reinterpret a playing song, and a song must not flatten a game's authored surround.

---

# 10. Windows host

Windows is the first host, not the product identity.

Current prototype:

```text
Windows application audio
→ self-excluding process loopback
→ portable stereo presentation
→ Omniphony renderer
→ physical FiiO/headphones
```

Current clean development setup:

```text
Hi-Fi Cable speaker config = Stereo / 2.0
foobar upmix = OFF
HeSuVi = OFF
ASIO Bridge forwarding = OFF
Omniphony = only physical path to headphones
```

The mature experience should become:

```text
install
→ Omniphony ON
→ play anything normally
```

The host still assumes **48 kHz float**. 48 kHz is a valid spatial-DSP rate and is not the current Cosmic Cove grain hypothesis, but mature routing should negotiate and report the actual endpoint/mix format and avoid unnecessary conversion.

---

# 11. HeSuVi relationship

HeSuVi remains an incumbent/perceptual oracle until Omniphony repeatedly earns replacement.

Useful incumbent functions:

```text
large bubble
rear presence
subjective density
low-end pressure / body
strong level
```

Do not copy its topology. Reproduce or surpass the useful percept.

Latest physical state:

> **P0.6 gained real height and began to sound better than HeSuVi. P0.7 is now pushing native Omniphony distance/reflection geometry toward the largest clean sphere while adding a small coherent foundation/body correction.**

Migration law remains:

> **Disable before uninstall.**

---

# 12. Validation lanes

## Fidelity

Track independently:

```text
clarity
center authority
transients
bass timing
bass pressure/body
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
front/rear balance
height distance
side width
side continuity
radial depth
source extent
ambient continuity
bypass collapse
```

## Frontier test

For every stronger candidate:

```text
current clean frontier
→ increase one or more native scale mechanisms
→ matched listening
→ did acoustic volume increase?
→ did ANY protected invariant regress?

NO regression
→ frontier advances

YES regression
→ crossed boundary
→ retreat / prune the mechanism responsible
```

The target is the **largest coherent presentation before blur**, not a preset chosen for conventional moderation.

## Stress material

`Cosmic Cove Galaxy` is the current known grain stress case.

Do not mark the linear-summing repair successful until this passage is physically retested.

---

# 13. Milestones

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
5.25 m native metric scale
12 × 18 × 8 m native first-order reflection room
0.38 reflection level
smaller 0.05 / 0.20 s late field
wide sides
far front
strong rear depth without rear gravity
very large upper canopy
linear master + body + support summing
coherent pressure/body layer
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
strong low-end physicality
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

# 14. Product anti-goals

- Do not replace inherited Omniphony merely because another renderer is fashionable.
- Do not discard the finished stereo master.
- Do not make rear gain synonymous with immersion.
- Do not make sides narrow merely because radial distance is front/back/height-led.
- Do not create fake LFE from stereo.
- Do not spatialize the bass foundation merely to make it impressive.
- Do not turn treble into height by register alone.
- Do not turn bass into floor placement by register alone.
- Do not use long reverb as a shortcut to a large bubble.
- Do not solve grain by shrinking the sphere.
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

# 15. Current commit checkpoint

Recent load-bearing changes:

```text
90948f1  linear master + support headroom; remove sample-wise clipping
0ad14f3  portable coherent foundation/body processor
6fbbf78  expose foundation processor from renderer crate
047f188  wire foundation delta into live Windows music path
a71eb55  push native bubble to 5.25 m / 12×18×8 m frontier
```

The next packaged build should answer, in this order:

```text
1. Is Cosmic Cove Galaxy grain gone?
2. Is the sphere clearly larger than the previous P0.7 candidate?
3. Does front feel farther rather than simply wetter?
4. Does rear have real depth without pulling the world behind the head?
5. Is the canopy higher/farther?
6. Do sides remain very wide and continuous?
7. Does the new foundation layer add pressure/body rather than boom?
8. Are transients, center, bass timing and microdetail still intact?
9. Has any part begun to blur?
```

If #9 is **no**, push the frontier again.

If #9 is **yes**, the previous clean state is evidence for the current perceptual boundary; remove or reduce the mechanism that crossed it rather than globally shrinking the effect.

---

# 16. Re-entry checkpoint

If context is lost, recover these facts first:

```text
1. upstream-derived Omniphony is the spatial heart
2. use native distance/HRTF/reflection/room machinery before inventing substitutes
3. finished stereo remains explicitly protected
4. frequency evidence decides what may enter the spatial support field
5. a separate small coherent foundation delta may improve physical body
6. Noire X EQ is headphone correction, not the spatial effect
7. P0.6 produced real height and began beating HeSuVi while staying clear
8. P0.7 is an edge-seeking native-renderer frontier search
9. front, rear and height distance create most of the desired bubble scale
10. sides must remain wide and continuous
11. rear depth is desirable; rear gravity is not
12. current frontier uses 5.25 m scale and a 12×18×8 m first-order reflection room
13. late room is intentionally reduced as geometry grows
14. host sample-wise support clipping has been removed
15. Cosmic Cove Galaxy must verify that grain repair physically
16. 48 kHz itself is not the current grain hypothesis
17. source truth always outranks inference
18. richer formats receive less inference than ordinary stereo
19. concurrent streams keep independent layouts
20. Windows is the first host, not the core
21. bypass should collapse the world, not restore the music
22. keep pushing useful native Omniphony capability until the first protected invariant breaks
23. the last clean state before blur is the current perceptual frontier
```

Supporting documents:

- `docs/frequency-evidence-music-path.md`
- `docs/music-presentation-contract.md`
- `docs/headphone-rendering-research.md`
- `docs/influence-ledger.md`
- `docs/windows-integration-research.md`

This README is the authoritative current project state when older lower-level documents disagree.
