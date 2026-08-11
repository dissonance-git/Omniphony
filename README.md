# Omniphony for Headphones

> **Private master project plan and canonical re-entry surface.**
>
> If conversational context disappears, recover the project from this README, recent `main` history, and the supporting contracts under `docs/` before inventing a new direction.

Omniphony for Headphones is a fork of [`mgth/Omniphony`](https://github.com/mgth/Omniphony) being turned into a **platform-agnostic, always-on headphone spatial processor**, with Windows as the first product host.

The central product law is now explicit:

> **Use the inherited Omniphony renderer as the spatial core as much as possible. Our work should primarily preserve source truth, infer safe presentation evidence, feed the renderer better material, and add only mechanisms the inherited core genuinely lacks.**

The target is not merely wider stereo and not merely a HeSuVi clone.

> **The headphones should perceptually disappear and the listener should stand inside a coherent, very large sphere of the same recording.**

A useful shorthand remains:

> **An open-source, modern Sony-360-like headphone world for ordinary stereo, native surround, games, movies and richer spatial sources without requiring specially authored music.**

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

The current music path keeps the mastered stereo waveform explicitly present and adds a separate derived spatial-support field around it.

This solved the major fidelity failure. The reliable listening floor is:

```text
raw clarity retained
bass/body retained
center remains solid
stereo identity remains intact
transients remain sharp
music still sounds like the same finished recording
```

The support field can now become aggressive spatially without requiring the master itself to be HRTF-rendered, reverberated or reconstructed.

## 0.3 P0.5 listening result

P0.5 expanded the frequency-evidence field into logical 7.1.4 at full derived-support strength.

Physical listening:

```text
clear as hell
clearly spatial
close to full HeSuVi replacement
but still rear-heavy
bubble too small
field behaved more like a horizontal band inside/behind the head
front projection weak
height weak
```

A tiny ON-only grain was also discovered on **Cosmic Cove Galaxy**. It was absent with Omniphony OFF.

## 0.4 P0.6 listening result

P0.6 deliberately moved energy and geometry forward and upward while preserving the protected master.

Physical listening established a major step:

```text
height now clearly exists
bubble is larger
sound is starting to beat HeSuVi rather than merely approach it
clarity remains otherwise excellent
```

The remaining shape problem is now more precise:

```text
still somewhat rear-heavy
more bubble size is desired
radial distance matters more than lateral wrap alone
```

The listener's own perceptual weighting is now a product input:

```text
FRONT DISTANCE   primary bubble cue
REAR DISTANCE    primary bubble cue, but must not dominate
HEIGHT DISTANCE  primary bubble cue
SIDE WIDTH       important for balance and continuity
SIDE DISTANCE    useful, but not the main source of scale
```

The intended world is therefore a **wide, depth-led sphere**, not a narrow front/back tunnel and not a side-heavy ring.

## 0.5 P0.7 native-renderer expansion

P0.7 is the current successor experiment.

Instead of inventing parallel spatial systems, it deliberately turns on and pushes more of **Omniphony's inherited binaural renderer** around the already-proven stereo evidence shell:

```text
SAF/KEMAR HRTF
analytic ITD
native metric distance cues
native first-order reflections
native short FDN room field
native distance air absorption on support only
native binaural clipping / auto-gain protection
```

Current music-field configuration is intentionally more expansive than the earlier conservative prototype:

```text
unit_scale_m         3.25
reflection room      7.0 m wide × 10.0 m deep × 4.8 m high
reflection level     0.30
late field level     0.07
RT60                 0.24 s
predelay              20 ms
```

The inherited upstream demo remains a useful reference at roughly:

```text
unit_scale_m         3.0
reflection level     0.4
late field level     0.2
RT60                 0.3 s
```

The product preset is not copying those numbers blindly. It is using the same **native mechanisms** while protecting the finished stereo master outside the room path.

The current virtual support geometry is explicitly distance-led:

```text
front floor       farther than sides
rear floor        real depth, shorter than front
sides             fully wide, closer radially
front height      longest / strongest radial canopy
rear height       real upper depth, shorter than upper-front
```

That geometry was committed after physical listening established that the desired bubble is driven most strongly by front/back/height distance, with wide sides providing balance and continuity.

---

# 1. The architectural invariant

This is **not** a ground-up replacement for Omniphony.

The load-bearing spatial engine remains upstream-derived Omniphony.

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

For ordinary stereo music the master is split conceptually, not destructively:

```text
FINISHED STEREO MASTER
        │
        ├──────────────────────────────────────→ protected direct master
        │
        └→ analysis only
             ↓
          safe spatial evidence
             ↓
        derived support field
             ↓
        OMNIPHONY CORE
             ↓
        binaural support
             │
protected master + aligned support
             ↓
          headphones
```

Engineering rule:

> **If Omniphony already has a mechanism for a spatial job, use and improve that mechanism before writing a second implementation beside it.**

Examples:

```text
distance          → native Omniphony first
HRTF / ITD        → native Omniphony first
reflections       → native Omniphony first
room field        → native Omniphony first
head tracking     → native Omniphony first
SOFA HRTFs        → native Omniphony first
clipping guard    → native Omniphony first where applicable
```

Our strongest custom ownership should remain:

```text
source preservation
stereo evidence
confidence / permission laws
presentation mapping
platform routing
validation
```

---

# 2. Hard fidelity law

> **Dimension may not be purchased by damaging the music.**

At matched loudness, bypass should ideally collapse:

```text
acoustic volume
front/back distance
height
lower spatial volume
radial depth
source extent
ambient continuity
listener envelopment
```

Bypass must **not** restore:

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

The current P0.6/P0.7 direction matters because the sphere has become substantially larger while the protected master has remained unusually clean.

---

# 3. Frequency-evidence music path

The active portable stereo presentation analyzes the recording without STFT-resynthesizing the master.

```text
1024-sample FFT ANALYSIS ONLY
        │
        ├→ L/R magnitude
        ├→ L/R phase
        ├→ true complex M/S
        ├→ pan
        ├→ coherence
        ├→ directness / diffuseness
        └→ persistence / stability
                 │
                 ▼
          scene inference
                 │
       ┌─────────┼─────────┐
       │         │         │
     BROAD    LATERAL   DIFFUSE
       │         │         │
       └──── causal multiband extraction
                 │
                 ▼
          derived 7.1.4 field
```

Current logical support roles:

```text
L/R         front / front-side broad extent
C           silent
LFE         silent
Ls/Rs       wide lateral continuity
Lb/Rb       restrained rear depth
Tfl/Tfr     dominant upper-front canopy
Tbl/Tbr     upper-rear closure
```

Bands:

```text
220–1200 Hz
1200–5000 Hz
5000 Hz–Nyquist
```

Durable laws:

- FFT is analysis-only;
- audible extraction remains causal;
- low-frequency foundation stays in the master;
- C and LFE remain silent for inferred stereo support;
- coherent anchors suppress aggressive spatial promotion;
- high frequency is **not** automatically height;
- low frequency is **not** automatically below;
- stereo evidence is permission, not hidden authored metadata.

Portable ownership:

```text
renderer/src/stereo_inference.rs
renderer/src/scene_inference.rs
renderer/src/music_field.rs
```

Windows owns transport, not the hearing logic.

---

# 4. Current bubble geometry law

The earlier development shorthand overemphasized “continuous wrap.” That is incomplete for this listener.

The new geometry hierarchy is:

```text
1. FRONT RADIAL DISTANCE
2. HEIGHT / UPPER RADIAL DISTANCE
3. REAR RADIAL DISTANCE
4. SIDE WIDTH + CONTINUITY
```

This does **not** mean narrow sides.

Desired shape:

```text
                 HIGH / FAR
            .----------------.
         .-'                  '-.
       .'      upper canopy      '.
      /                            \
     /          FAR FRONT           \
    |                                |
WIDE|              YOU               |WIDE
SIDE|                                |SIDE
    |                                |
     \            FAR REAR          /
      '.                          .'
         '-.                  .-'
            '----------------'
```

The side field should remain broad and seamless. It simply should not be the primary perceptual mechanism for making the world feel several feet away from the skull.

Rear remains important. The goal is **rear distance without rear gravity**.

---

# 5. Grain defect: current fidelity blocker

A tiny grain remains audible on **Cosmic Cove Galaxy** with Omniphony ON.

Important evidence:

```text
present on local playback
present when the same material is played from YouTube
absent with Omniphony OFF
otherwise the presentation is very clean
```

Therefore do **not** blame:

```text
local file corruption
one local codec/container
one player-specific decode
```

The content is acting as a repeatable stress signal for the Omniphony ON path.

Current strongest suspect is the host's final protected-master combiner. It presently preserves the master by **hard-clamping only the added support sample-by-sample to whatever instantaneous headroom remains before ±1.0**.

That operation is nonlinear:

```text
base + clamp(support, remaining sample headroom)
```

On dense near-full-scale material it can shave only portions of the support waveform and create a tiny clipping-like texture while leaving the protected master itself clear.

This is a better fit for the observed defect than a codec explanation.

Next fidelity experiment:

```text
remove sample-wise support shaving
→ reserve linear summing headroom for master + support together
→ keep master/support ratio and geometry intact
→ compare exact Cosmic Cove passage
```

The fix must be **spatially neutral**. Do not solve the grain by shrinking front, rear, height or side support.

---

# 6. Influence ledger re-evaluation after P0.6/P0.7

`docs/influence-ledger.md` remains durable research memory, but several old statuses change now that the protected master is structurally separate from the rendered support scene.

## 6.1 Promote now through Omniphony itself

### Upstream Omniphony

Highest priority. The inherited renderer already owns the acoustical mechanisms we need most:

```text
distance
HRTF / ITD
first-order reflections
short room field
air-distance cues
head pose
SOFA-capable HRTFs
```

P0.7 explicitly restores more of those mechanisms instead of approximating them externally.

### 3D Tune-In Toolkit

Its direct/environment separation is now highly actionable because Omniphony for Headphones already has the right boundary:

```text
protected mastered DIRECT
≠
derived support / ENVIRONMENT
```

This makes native room and distance cues much safer than they were in the rejected full-wet stereo design.

### Google Open Binaural Renderer

The Direct / Ambient / Reverberant role distinction maps cleanly onto the current system:

```text
Direct        protected stereo master
Broad/Ambient derived support evidence
Reverberant   native Omniphony room field
```

Use the role asymmetry, not another renderer.

### Cavern

Direction + distance is now a particularly relevant benchmark because listening says radial scale is one of the strongest contributors to the desired bubble.

### ArtifexEt / Foobar-for-Home-Theater

Its strongest transferable law already helped P0.6:

```text
preserve the authoritative bed/master
+ synthesize only missing dimension
+ top-front stronger than top-back
+ sides stronger than unnecessary rear accumulation
```

The AVR-specific coefficients remain reference material, not headphone truth.

## 6.2 Promote as analysis / validation, not audible replacement DSP

### FreeSurround / Real3D

Still valuable for amplitude+phase spatial evidence and confidence.

Do not adopt the reconstructed fake speaker bed as authored truth.

### Trifield

Center authority remains a first-class invariant.

Do not turn the phantom center into a mandatory literal center object.

### LCC

Preserve useful source ITD/ILD relations. Do not transplant loudspeaker crosstalk cancellation directly to headphones.

### NUGEN Halo / Penteo

Their strongest current contribution is **reversibility and source-safety testing**:

```text
expand
→ collapse / downmix
→ original relationships should remain recognizably intact
```

As the bubble becomes dramatic, this becomes more important, not less.

### FIR / convolution literature

Use for temporal validation:

```text
pre-ringing
ringing
phase / group delay
transition artifacts
```

The persistent Cosmic Cove passage should become a permanent stress fixture if a deterministic capture can reproduce the grain.

## 6.3 Newly viable, but not free yet

### Source extent / Airwindows Wider lesson

The conceptual lesson is now stronger: compact anchors, broad sources and diffuse fields should not all be points.

However the current **binaural branch does not simply consume the renderer's generic VBAP object-size/spread knobs**, so source extent is not yet a configuration-only win. It requires a real binaural-core mechanism or an equivalent native representation.

Promote only when implemented inside the inherited renderer rather than as indiscriminate decorrelation around it.

### Ambisonics / IEM / Steam Audio / Resonance Audio

Ambisonics remains interesting for a genuinely continuous diffuse support field if 7.1.4 eventually exposes audible holes.

But:

```text
do not graft Steam Audio over Omniphony
do not graft Resonance Audio over Omniphony
do not insert HOA merely because it is elegant
```

First ask whether Omniphony's own binaural/direct/reflection machinery can solve the observed weakness.

### SOFA / HRTF personalization

Increasingly important later because a larger, cleaner bubble makes HRTF mismatch easier to hear. Keep the built-in SAF/KEMAR reference while adding listener-specific options rather than replacing the known baseline blindly.

## 6.4 Still parked or rejected as defaults

```text
wholesale full-wet stereo HRTF replacement
strong generic crossfeed
fake LFE from stereo
high-frequency = height shortcut
low-frequency = floor shortcut
obvious late-reverb wash
rear gain as the definition of spatiality
external binaural engine grafts over Omniphony
semantic source wandering / live remix behavior
```

---

# 7. Development policy: add native capability, then prune

The current experiment policy is deliberately more aggressive than the early isolation phase.

During P0.2–P0.4 the project needed tiny attributable experiments because the basic music architecture was unknown.

That foundation now exists.

For P0.7 and later sound development:

> **Turn on useful existing Omniphony mechanisms aggressively enough to expose their value, then remove or narrow only the mechanisms that listening proves cost clarity or coherence.**

The protected master is the safety floor.

This does **not** authorize random DSP accumulation. It means:

```text
prefer native Omniphony mechanism
→ push to useful audibility
→ compare exact material
→ keep / narrow / remove
```

not:

```text
invent parallel mechanism
→ stack another effect
→ hope the total becomes immersive
```

---

# 8. Sample rate and format

The current Windows prototype processes at **48 kHz float**.

48 kHz is an appropriate spatial-DSP rate and is **not currently suspected as the cause of the Cosmic Cove grain**.

The host is still too rigid, however:

```text
capture requests fixed 48 kHz stereo float
renderer runs at fixed 48 kHz
output selects a device format that supports 48 kHz
Windows shared mode may auto-convert around that route
```

Mature host behavior should negotiate and report the actual endpoint/mix format rather than silently assuming one fixed rate forever.

Transport roadmap:

```text
discover endpoint / engine format
→ choose one authoritative process rate
→ avoid unnecessary conversion where practical
→ use explicit high-quality resampling when conversion is required
→ keep sample-rate handling outside the portable hearing model
```

Codec is not the current grain hypothesis because the same ON-only texture is heard from different delivery paths while OFF remains clean.

---

# 9. Universal source-truth law

Inference decreases as source truth increases.

```text
STEREO
→ preserve finished master
→ add validated derived support

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

# 10. Concurrent-stream law

Channel layout belongs to a logical stream, not to Omniphony globally.

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

# 11. Windows host law

Windows is the first host, not the product identity.

Current development route:

```text
Windows application audio
→ self-excluding process loopback
→ portable stereo presentation
→ Omniphony renderer
→ physical FiiO/headphones
```

Current clean listening setup:

```text
Hi-Fi Cable speaker config = Stereo / 2.0
foobar upmix = OFF
HeSuVi = OFF
ASIO Bridge forwarding = OFF
Omniphony = only physical path to FiiO/headphones
```

The mature experience should become:

```text
install
→ Omniphony ON
→ play anything normally
```

No permanent manual cable ritual should survive into the product.

---

# 12. HeSuVi relationship

The incumbent remains a perceptual oracle until repeated listening clearly proves replacement.

Useful incumbent functions:

```text
large bubble
rear presence
subjective density
bass/body
strong level
```

Do not copy the topology. Reproduce or surpass the percept.

The latest listening state is materially stronger than the old README claimed:

> **P0.6 gained real height and began to sound better than HeSuVi. P0.7 enlarged the bubble further by restoring native Omniphony distance/environment mechanisms.**

That is not yet permission to uninstall the incumbent.

Migration law remains:

> **Disable before uninstall.**

---

# 13. Validation lanes

Keep objective and listening failures attributable.

## Renderer / DSP

```text
compile
→ unit tests
→ deterministic fixtures
→ clipping / residual checks
→ transient / phase checks
```

## Stereo fidelity

```text
OFF master
vs
ON master + support
```

Track:

```text
clarity
center
bass
transients
stereo definition
microdetail
timbral identity
```

## Spatial world

Track separately:

```text
front distance
rear distance
height distance
side width
side continuity
front/rear balance
radial depth
source extent
ambient continuity
bypass collapse
```

## Stress material

`Cosmic Cove Galaxy` is currently the most useful known audible stress case for the tiny grain defect.

Do not treat a fix as complete merely because ordinary tracks are clean.

## Source-safe collapse

Inspired by Halo/Penteo:

```text
stereo master
→ derived presentation
→ defined collapse / analysis projection
→ compare original relationships
```

Perfect bit identity is not required, but destructive changes should become measurable.

---

# 14. Milestones

## W0 - upstream spatial reference

Established.

## P0 - native protected listening

Established.

## P0.1 - arbitrary live Windows audio

Established.

## P0.2 - clean stereo architecture

Established. Full-wet stereo rejected.

## P0.3 - protected-master fidelity floor

Established.

## P0.4 - frequency-evidence field

Established as the durable stereo inference foundation.

## P0.5 - full-strength 7.1.4 support shell

Established as a major spatial step. Clean and clearly spatial, but too rear-heavy and too belt-like.

## P0.6 - anterior + vertical shell

**LISTENING WIN.**

Established:

```text
real height
larger bubble
clarity retained
starting to exceed HeSuVi
```

Remaining:

```text
rear still somewhat too heavy
radial distance can grow much more
tiny Cosmic Cove grain remains
```

## P0.7 - native Omniphony maximal / distance-led bubble

**ACTIVE EXPERIMENT.**

Uses more inherited Omniphony rather than adding parallel DSP:

```text
stronger metric distance
explicit front/rear/height radial hierarchy
wide sides
larger native reflection room
stronger but still bounded early reflections
short low native FDN
support-only air cues
native auto-gain
```

Success criterion:

> **A substantially larger front/back/height volume with wide sides, less rear gravity, and no loss of the P0.6 clarity floor.**

## P0.7.x - grain removal

Remove the tiny content-dependent ON-path grain **without shrinking the sphere**.

Primary candidate: replace sample-wise support shaving with clean linear summing headroom.

## P1 - excellent everyday stereo music

Target:

```text
huge coherent 360° sphere
far front
real rear depth without rear dominance
convincing overhead volume
wide continuous sides
some lower-shell plausibility
near / mid / far layering
source extent without smear
ambient continuity
strong energy and bass physicality
raw-master clarity intact
bypass collapses the world, not restores the song
```

## P2 - owned Windows routing

Replace cable/loopback scaffolding without changing the protected sound.

## P3 - native surround / richer sources

Preserve real channel/object/height truth and concurrent layouts.

## P4 - deeper stereo presentation

Likely candidates after the current distance pass:

```text
binaural-native source extent
better diffuse-field continuity
lower hemisphere
source-safe collapse metrics
SOFA / HRTF fitting
bounded Ambisonic support only if discrete-field holes remain
```

## P5 - personalization

```text
headphone correction
HRTF selection/import
listener fitting
head tracking
advanced controls
```

---

# 15. Product anti-goals

- Do not replace the inherited renderer because another spatial engine looks fashionable.
- Do not discard the finished stereo master.
- Do not make rear gain synonymous with immersion.
- Do not make sides narrow merely because radial distance is front/back/height-led.
- Do not require virtual 7.1 for normal headphone output.
- Do not create fake LFE from ordinary stereo by default.
- Do not turn treble into height by register alone.
- Do not turn bass into floor placement by register alone.
- Do not use obvious reverb as a substitute for geometry.
- Do not solve grain by reducing the whole support field.
- Do not let ON become less clear than OFF.
- Do not erase useful source ITD/ILD.
- Do not make a FreeSurround/Real3D inferred bed pretend to be authored truth.
- Do not insert Ambisonics unless it solves an actual field-continuity problem.
- Do not graft Steam Audio, Resonance Audio or Cavern over Omniphony.
- Do not let semantic analysis move sources around audibly in realtime.
- Do not let Windows APIs contaminate the portable core.
- Do not uninstall HeSuVi until Omniphony has repeatedly earned it.
- Do not accept duplicate physical routes or dirty OFF behavior.
- Do not let an exciting five-second spatial effect outrank long-session fidelity.

---

# 16. Re-entry checkpoint

If context is lost, recover these facts first:

```text
1. upstream-derived Omniphony is still the spatial heart
2. use its own distance/HRTF/reflection/room machinery before inventing substitutes
3. finished stereo remains explicitly protected
4. frequency evidence decides what may enter the support field
5. P0.6 produced real height and began beating HeSuVi while staying very clear
6. P0.7 is the active native-renderer distance-led expansion
7. bubble scale is driven mainly by front, rear and height distance
8. sides should remain wide for balance and continuity
9. rear depth is desirable; rear dominance is not
10. Cosmic Cove Galaxy exposes a tiny repeatable ON-only grain
11. the grain also appears via YouTube, so local file/codec is not the explanation
12. current strongest grain suspect is sample-wise support headroom clipping in the host combiner
13. fix grain spatially neutrally, not by shrinking the bubble
14. native early reflections, short room field and support-only air cues are now active candidates because the master is outside that path
15. 3D Tune-In and OBR role separation now map directly onto the architecture
16. FreeSurround/Real3D remain evidence sources, not rendering truth
17. Halo/Penteo now matter strongly as reversibility/source-safety benchmarks
18. source extent is desirable but not yet a free binaural configuration knob
19. Ambisonics remains optional and must earn itself
20. 48 kHz is a valid current process rate; mature Windows routing should negotiate formats explicitly
21. richer source truth always outranks stereo inference
22. simultaneous streams keep independent layouts
23. Windows is the first host, not the core
24. bypass should collapse the world, not restore the music
25. keep adding useful native Omniphony capability, then prune whatever listening proves harmful
```

Supporting documents:

- `docs/frequency-evidence-music-path.md`
- `docs/music-presentation-contract.md`
- `docs/headphone-rendering-research.md`
- `docs/influence-ledger.md`
- `docs/windows-integration-research.md`

This README is the authoritative current project state when older lower-level documents disagree.
