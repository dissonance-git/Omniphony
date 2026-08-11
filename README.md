# Omniphony for Headphones

> **Private master project plan and canonical re-entry surface.**
>
> If conversational context disappears, recover the project from this README, recent `main` history, and the supporting contracts under `docs/` before inventing a new direction.

Omniphony for Headphones is a fork of [`mgth/Omniphony`](https://github.com/mgth/Omniphony) being turned into a **platform-agnostic, always-on headphone spatial processor**, with Windows as the first product host.

The central product law is simple:

> **Upstream Omniphony already has a very good spatial/binaural core. Preserve and extend that percept. Do not replace it with research for research's sake.**

The job is to make that renderer usable as a universal listening layer for an operating system, then make it unusually good for the dominant real-world source: **ordinary mastered stereo music**.

The target is not merely wider stereo and not merely a HeSuVi clone.

> **The headphones should perceptually disappear and the listener should stand inside a coherent sphere of the same recording.**

Useful category shorthand:

> **An open-source, modern Sony-360-like headphone world that works with ordinary stereo, native surround, games, movies and richer spatial sources without requiring specially authored music.**

---

# 0. Current protected sound foundation

This is the most important re-entry checkpoint in the repository.

The early full-wet stereo experiments failed because replacing the mastered stereo image with two HRTF-rendered virtual speakers damaged the recording:

```text
tinny / cheap-phone tone
bass and body loss
smaller useful stereo image
audible room / hallway coloration
less definition than bypass
```

The next preserved-master experiments established the opposite extreme:

```text
ON ≈ OFF
raw music intact
no obvious tonal damage
but little useful added world
```

The current **frequency-evidence music architecture** is the first direction that has preserved the raw stereo sound while beginning to add useful spatial enhancement.

Longer listening outranks the first few minutes of excitement. The reliable result is now:

```text
raw stereo clarity remains
music still feels intact
no obvious tinny / hallway regression
ON is mildly enhanced versus OFF
frequency-evidence support is audible enough to be promising
```

Do **not** currently treat these earlier first-impression claims as proven:

```text
bigger than HeSuVi
stronger rear localization than HeSuVi
fully convincing 360° bubble
convincing height
strong bypass-collapse effect
```

Some of the first perceived jump was confounded by becoming reacquainted with raw stereo after turning the old HeSuVi chain off. Correction outranks coherence: the architecture is validated as a **fidelity-preserving foundation**, not yet as a finished spatial win over the incumbent.

This is the protected stereo sound floor:

> **Omniphony ON can preserve the clarity, bass/body, definition and identity of the mastered stereo signal while adding a mild spatial layer. Build outward from here.**

From this point forward, every sound change must preserve or improve:

```text
raw clarity
bass/body
center solidity
stereo identity
transient definition
groove timing
timbral naturalness
microdetail
comfort
```

while increasing:

```text
front externalization
side wrap
rear discrimination without rear dominance
height
below-listener plausibility
radial depth
source extent
ambient continuity
listener envelopment
separation without disassembly
energy / punch / density
bypass-collapse strength
```

A candidate may no longer buy a bigger sphere by regressing to the earlier tinny, hollow, phasey or reverberant sound.

The current practical stereo-development setup is:

```text
Hi-Fi Cable speaker configuration = Stereo / 2.0
foobar upmix                     = OFF
HeSuVi                           = OFF
ASIO Bridge forwarding           = OFF
Omniphony                        = only audible path to FiiO/headphones
```

Changing the temporary Hi-Fi Cable endpoint from Windows 7.1 to Stereo restored normal playback level. Treat that as a **prototype transport/gain finding**, not a permanent user requirement. The finished Windows host must own the routing automatically.

The active successor experiment above this protected floor expands the derived support field from sparse logical 7.1 to **overlapping logical 7.1.4**, adds conservative height participation, reduces rear-only concentration, and runs the derived support field at full host strength. It must earn promotion by listening.

---

# 1. The inheritance map

This project is **not a ground-up replacement for Omniphony**.

The load-bearing acoustical renderer remains upstream-derived Omniphony.

## 1.1 Inherited spatial heart

The fork continues to build on upstream machinery for:

```text
source / channel / object geometry
→ renderer state
→ HRTF / HRIR selection and interpolation
→ interaural timing
→ binaural convolution
→ head orientation
→ source distance cues
→ early-reflection machinery
→ late-room machinery
→ VBAP and known-layout rendering
→ object / bed semantics
→ SOFA-capable HRTF support
→ head-tracking capability
→ stereo binaural output
```

Important inherited components include:

- the `renderer` crate and `SpatialRenderer`;
- the independent binaural branch;
- HRTF/HRIR providers and convolution machinery;
- ITD and head-pose handling;
- channel/object metadata and ramps;
- VBAP and speaker layouts;
- early reflections and FDN room support;
- bridge-based decoding boundaries;
- the headless renderer/engine structure;
- upstream Studio's useful separation between rendering and supervision.

That is the **heart** of the product.

“Upstream core” means upstream-derived renderer architecture and machinery, not an untouched upstream binary hidden underneath the fork. The fork has already modified and extended parts of that code.

## 1.2 What this fork adds around it

The fork's major ownership is increasingly:

```text
portable source contracts
stereo-music presentation
source-truth preservation
frequency-dependent stereo evidence
concurrent-stream semantics
Windows system routing
future platform adapters
always-on product shell
clean bypass / transport
fidelity regression tests
music-specific preservation laws
research / evaluation machinery
selective renderer corrections only when earned
```

Desired architecture:

```text
                      PLATFORM HOST
          Windows now / macOS or Linux later
                           │
                           ▼
                 PORTABLE INPUT CONTRACTS
             stereo / beds / objects / HOA
                           │
                           ▼
              MUSIC / SOURCE PRESENTATION
          preserve truth, add only earned structure
                           │
                           ▼
              UPSTREAM-DERIVED OMNIPHONY CORE
        HRTF + ITD + geometry + binaural rendering
                           │
                           ▼
                    BINAURAL STEREO
                           │
                           ▼
                  ordinary 2.0 headphones
```

Engineering rule:

> **When a problem can be solved by feeding the inherited renderer better material or by preserving the source around it, prefer that over replacing the renderer.**

There is no meaningful code-line percentage for “how much is upstream.” Host, tests and research code can dwarf a compact DSP kernel. Acoustically, however, the upstream-derived Omniphony renderer remains the load-bearing spatial engine.

---

# 2. Perceptual north star

The strongest music aspiration is:

> **A song should sound as though an elite immersive mix/mastering engineer had already prepared that exact recording for this full-sphere headphone presentation before playback began.**

This is a quality target, not a mandate to imitate an engineer at runtime.

The result should feel:

```text
finished
authored
stable
coherent
inevitable
```

not:

```text
reactive
warped section-by-section
sources wandering
faders being ridden live
constantly reclassified
obviously controlled by an algorithm
```

Ordinary DSP state is expected. Convolution histories, delay lines, interpolation, smoothing, room decay and future head tracking are normal realtime mechanisms.

The distinction is:

```text
PRE-AUTHORED IMMERSIVE QUALITY
without requiring
LIVE SEMANTIC REMIXING
```

The final full-sphere presentation may contain:

```text
front
side
rear
above
below
near
far
compact sources
broad sources
diffuse musical fields
room / environmental fields
```

But the musical object always comes first.

---

# 3. Hard fidelity law

> **Dimension may not be purchased by damaging the music.**

At matched loudness, bypass should ideally collapse:

```text
perceived acoustic volume
externalization
front/back structure
height
below-listener structure
radial depth
source extent
ambient continuity
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
musical hierarchy
stereo definition
comfort
```

Desired bypass reaction:

> **“The world collapsed.”**

Not:

> **“The music came back.”**

Frozen invariant:

> **Omniphony ON may never be narrower, less defined, less energetic, or tonally cheaper than the authoritative stereo master merely to obtain spatial cues.**

This law outranks clever DSP, research elegance and feature count.

---

# 4. Main use case: ordinary stereo music

The everyday source is usually normal two-channel music.

The mature path should be:

```text
foobar / Spotify / browser / player
        ↓
finished stereo master
        ↓
Omniphony stereo presentation
        ↓
upstream-derived Omniphony binaural renderer
        ↓
ordinary 2-channel DAC / headphones
```

The listener should not configure the operating system or physical DAC as virtual 7.1 just to hear stereo music.

```text
physical headphone output = 2.0
```

The central creative problem is:

> **How can a finished stereo master gain convincing full-sphere physicality while remaining recognizably and tonally the same finished master?**

The current architecture is the first answer that has sounded promising enough to protect as a fidelity floor.

---

# 5. Stereo is authoritative, not raw material to discard

A finished stereo recording already contains valuable authored information:

```text
left/right image
center authority
bass relationships
phase relationships
interaural timing / level relationships
width
transients
ambience
reverberation
mix hierarchy
```

The default stereo presentation should preserve those properties and grow a spatial world around them.

Conceptual law:

```text
                         STEREO MASTER
                              │
                ┌─────────────┼─────────────┐
                │             │             │
                ▼             ▼             ▼
           FOUNDATION       DIRECT         FIELD
           bass / body      authored       ambience /
           groove floor     stereo image   spatial support
                │             │             │
                └─────────────┼─────────────┘
                              ▼
                     OMNIPHONY RENDERER
                              ▼
                         headphones
```

This is not necessarily three literal stems. It is a law about what remains authoritative.

## 5.1 Foundation protection

Low-frequency energy is dangerous to spatialize indiscriminately.

The portable scene evidence already encodes a conservative `80-220 Hz` bass-protection transition.

Product implication:

```text
bass / pressure / groove foundation
→ preserve first
→ spatially move only when evidence earns it
```

Do not fix structural bass loss afterward with a compensating shelf.

## 5.2 Direct versus field

Keep this distinction:

```text
DIRECT
identity / attack / center / definition / authored stereo image

FIELD
ambient / decorrelated / broad / environmental support
```

Environment is not a substitute for geometry, and geometry is not permission to replace the direct master.

---

# 6. Current protected stereo architecture

The old pure `(L-R)/2` side-field experiment is historical evidence now. It proved that a protected dry/master path could remain transparent, but the spatial branch was too starved to matter.

The active foundation is the **frequency-evidence music path**.

```text
FINISHED STEREO MASTER
        │
        ├────────────────────────────────────────→ protected direct path
        │
        └→ 1024-sample FFT ANALYSIS ONLY
              │
              ├→ L/R magnitude
              ├→ L/R phase
              ├→ pan / phase coherence
              ├→ directness / diffuseness
              ├→ true complex M/S
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
                 derived support field
                        │
                        ▼
              UPSTREAM OMNIPHONY
              HRTF / ITD / binaural
                        │
                        ▼
                  stereo support
                        │
        protected master + aligned support
                        │
                        ▼
                    headphones
```

The protected listening floor used differentiated broad/lateral/diffuse support while preserving the master. The active successor now distributes those same evidence classes into overlapping **logical 7.1.4** lanes:

```text
L/R         broad front/front-side extent
C           silent
LFE         silent
Ls/Rs       strongest lateral wrap
Lb/Rb       restrained rear continuation
Tfl/Tfr     front-height extent
Tbl/Tbr     rear-height / upper diffuse continuation
```

Physical output remains ordinary binaural stereo. The internal multichannel structure is only a differentiated support field for the inherited renderer.

Important properties:

- FFT is **analysis-only** and does not STFT-resynthesize the master;
- the audible support extraction remains causal;
- center/foundation evidence can suppress aggressive field promotion;
- low-frequency foundation remains in the master;
- no artificial LFE is created for stereo music;
- support is latency-aligned before combination;
- the master gets first claim on headroom;
- early reflections remain off in the current shell experiment;
- late reverb is off;
- air absorption is off;
- spatiality therefore comes primarily from evidence + geometry + HRTF/ITD rather than an audible room tail.

Portable ownership lives in:

```text
renderer/src/stereo_inference.rs
renderer/src/scene_inference.rs
renderer/src/music_field.rs
```

Windows owns transport, not the hearing logic.

Detailed checkpoint:

`docs/frequency-evidence-music-path.md`

---

# 7. Sound frontier from this foundation

This section is deliberately about **sound**, not product features.

The protected result is the foundation. The next work is to make the same intact recording inhabit a dramatically stronger acoustic world.

## 7.1 Increase effect strength without losing the master

The longer listening result says the current protected frequency-evidence presentation is only **mildly enhanced** versus OFF.

That means the next candidates must make spatial support unmistakable while retaining the same clarity floor.

The active 7.1.4 shell experiment therefore increases support energy and gives the derived field more places to exist rather than merely increasing rear gain.

Success means:

```text
ON is immediately distinguishable from OFF
but OFF does not sound cleaner, fuller or more correct
```

## 7.2 Front / side / rear balance

The sphere should become:

```text
front authority
+ front-side continuity
+ side wrap
+ rear depth
```

rather than dry stereo plus a detached rear layer.

Rear should become one region of a larger shell, not the default destination of everything that is not centered.

## 7.3 Stronger front externalization

The front image should project outward instead of remaining mostly at the head while support appears elsewhere.

Target:

```text
center stays solid
front wall moves away from the forehead
left/right front image gains acoustic distance
no vocal thinning
no center hole
```

## 7.4 Side wrap and continuous 360° bubble

Desired:

```text
front
→ front-side
→ side
→ rear-side
→ rear
```

as a coherent wrap around the listener.

The field should be continuous enough to inhabit while remaining structured enough to localize.

## 7.5 Height and below-listener structure

The full-sphere ambition includes vertical dimension.

Desired sound:

```text
upper field adds canopy / air / vertical extent
lower field adds grounding / lower spatial volume
neither becomes a crude EQ trick
```

Stereo contains little authored height truth, so inferred vertical treatment should primarily operate on already-spatial broad/diffuse evidence and remain conservative for direct anchors.

Do not turn treble into “height” merely because it is high frequency.
Do not move bass foundation downward merely because it is low frequency.

## 7.6 Radial depth

The target is not a flat ring around the head.

The presentation should develop:

```text
near
mid-distance
far
```

with enough radial layering that sources and fields can occupy different acoustic distances without destroying mix hierarchy.

Bypass should make the world shallower and smaller, not clearer.

## 7.7 Source extent

Not every musical source should collapse to a point.

The mature presentation should support a continuum:

```text
compact anchor
→ coherent broad source
→ diffuse field
```

without indiscriminate decorrelation.

## 7.8 Ambient continuity

The sphere should not have obvious virtual-speaker holes.

Desired field:

```text
continuous enough to inhabit
structured enough to localize
```

This is one place where more continuous geometry, diffuse-field representation or eventually an Ambisonic support field may earn a role.

## 7.9 Separation without disassembly

Omniphony should reveal more room between musical agents without making the mix sound pulled apart.

Desired:

```text
more legibility
more breathing room
more dimensional layering
same song
```

Not:

```text
stem-demo separation
holes in the mix
sources detached from their musical relationships
```

## 7.10 Energy, punch and density

Spatiality and energy are separate dimensions.

The incumbent HeSuVi/DTS + foobar chain demonstrates a sound that can feel unusually energetic through a combination of gain structure, multichannel summation, HRIR response and limiting.

Omniphony should reproduce the **beneficial percept**, not the accidental topology.

Target:

```text
strong level
physical bass/body
kick and transient punch
subjective density
presence
large spatial field
without clipping or crushed dynamics
```

The finished product should not feel bare or polite merely because its signal path is cleaner.

Do not let louder-is-better contaminate spatial A/B tests, but do not leave energy as an afterthought either.

## 7.11 Bass physicality

Bass is more than quantity.

Protect and eventually improve:

```text
mass
groove lock
kick/bass interlock
melodic bass contour
pressure
body
```

The low-frequency foundation should feel at least as physical and timed as the dry master and eventually competitive with the incumbent chain, without turning bass into fake omnidirectional LFE.

## 7.12 Transient sharpness and microdetail

More world must not mean softer attacks.

The final presentation should retain or improve:

```text
attack precision
microdetail
fast percussion clarity
rhythmic lock
```

## 7.13 Stable, pre-authored appropriateness

The presentation should become more appropriate to different recordings without sounding like a live remix.

Desired percept:

> **This recording somehow already belongs in this acoustic world.**

Not:

> **The algorithm noticed something and moved it.**

Confidence should control aggression. Uncertain evidence should become broad, reversible support rather than precise spatial fiction.

## 7.14 Long-session comfort

A spectacular five-second demo is insufficient.

Avoid:

```text
spectral glare
phase fatigue
constant spatial motion
excessive crossfeed
room buildup
transient softening
pressure imbalance
```

The target is a presentation that becomes difficult to turn off because it feels natural, not because it constantly advertises itself.

---

# 8. Direct, broad, diffuse and room remain distinct

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

A “bigger” sound fails if it merely converts direct musical material into room coloration.

External research reinforces:

```text
DIRECT
precise / short / identity-bearing

AMBIENT FIELD
broad / diffuse / spatial support

ROOM / REVERBERANT FIELD
longer temporal structure
```

For music, the default room contribution may be tiny or absent. Spatial structure should survive without hallway coloration.

Directional early reflections may be tested later only if they add externalization/body without echo, doubling, comb coloration or transient damage.

---

# 9. Stereo convolution and source-safe transfer laws

The foobar Stereo Convolver lineage suggests a useful conceptual model.

A general stereo convolver is a 2x2 transfer matrix:

```text
yL = L * HLL + R * HRL
yR = L * HLR + R * HRR
```

Useful interpretation:

```text
DIAGONAL TERMS
HLL / HRR
→ preserve source authority and direct identity

OFF-DIAGONAL TERMS
HLR / HRL
→ add controlled interaural / spatial support
```

Conceptually:

```text
H(z) = I + S(z)
```

where `I` is protected stereo identity and `S(z)` is only the spatial support that earns its existence.

This is a research model, not a frozen implementation formula.

The important law is:

> **The master can remain explicitly present instead of being deleted and synthesized again.**

Professional upmix references such as Halo Upmix and Penteo add another constraint:

```text
stereo master
→ expanded presentation
→ defined collapse / downmix
→ original relationships remain recognizably recoverable
```

Perfect bit-exact reversibility is not required for every perceptual enhancement, but the sphere may not be created by destroying the source structure.

---

# 10. Source-safe stereo evidence laws

The Trifield, LCC, FreeSurround/Real3D, MathAudio, professional upmix and stereo-width research passes produced durable laws.

## 10.1 Center authority must survive

Trifield's useful lesson is not “make a center speaker.” It is that the center image is its own stability problem.

```text
correlated / center-like energy
→ protect center authority

side / differential energy
→ candidate expansion evidence
```

Do not gain rear width by making vocals, snares or other center anchors hollow or vague.

## 10.2 Existing interaural cues are source evidence

LCC is not a headphone algorithm to copy literally. Its deeper lesson transfers:

> **Do not erase useful ITD/ILD relations already encoded in the source merely because synthesized binaural cues are available.**

## 10.3 Analysis is not rendering

FreeSurround and Real3D demonstrate that frequency-dependent amplitude and phase relations can provide candidate spatial evidence:

```text
L/R amplitude relation
+ L/R phase relation
→ candidate position / field confidence
```

But a synthetic speaker bed is only one rendering choice.

Do not confuse:

```text
spatial evidence extracted from stereo
```

with:

```text
authored multichannel truth
```

## 10.4 Audible room is not required for spatiality

If Omniphony needs an obvious reverb tail to feel 3D, the geometry/presentation problem is not solved yet.

## 10.5 Small transforms may outperform wholesale reconstruction

M/S width/depth processors and headphone crossfeed systems show that small frequency- and timing-aware transformations can materially reshape space without rebuilding the whole mix.

Use complexity only where listening earns it.

## 10.6 MathAudio lessons are bounded

Useful lessons:

- headphone correction and spatial presentation are different problems;
- natural interaural coupling can be useful;
- correction should be bounded rather than blindly inverted;
- phase, transient behavior and pre-ringing matter.

Do not inherit a blanket “FIR is bad” conclusion. Realtime binaural systems use convolution successfully when phase, latency and transitions are designed correctly.

---

# 11. Universal source-truth law

Inference decreases as source truth increases.

```text
STEREO
finished presentation but little explicit full-sphere truth
→ preserve master
→ add validated support

5.1 / 7.1
real directional channel truth
→ preserve and render it

5.1.2 / 7.1.4
real elevation information
→ preserve authored height

OBJECT AUDIO
real supplied positions
→ render objects directly

AMBISONICS / HOA
real field representation
→ preserve the field

ALREADY-BINAURAL / VIRTUALIZED AUDIO
existing headphone spatial cues
→ avoid destructive double virtualization
```

Source truth outranks reconstruction.

Richer sources should generally require **less inference**, not more.

The stereo evidence system may create broad/lateral/diffuse support. It may not claim to have recovered hidden authored rear or height coordinates.

---

# 12. Concurrent-stream law

Channel layout belongs to a logical source/stream, not to Omniphony globally.

A desktop may contain:

```text
foobar       stereo 2.0
Overwatch    native 5.1 / 7.1 / spatial
voice chat   mono / stereo
future app   objects / richer spatial metadata
```

Correct model:

```text
Stream A { layout = stereo }
Stream B { layout = 7.1 }
Stream C { layout = mono }
Object stream D { richer spatial state }
        ↓
shared Omniphony output timeline
        ↓
binaural stereo
```

Wrong model:

```text
Omniphony global mode = stereo
or
Omniphony global mode = 7.1
```

Starting a surround game must not reinterpret a playing stereo song.
Playing stereo music must not flatten the game's native surround truth.

Permanent regression case:

```text
stereo alone
surround alone
stereo + surround simultaneously
→ all remain stable and correctly interpreted
```

The temporary Windows loopback prototype cannot yet preserve application boundaries this way. The portable core contract should.

---

# 13. Portable core law

The core should accept platform-neutral logical streams and emit binaural stereo.

```text
InputStream
  id
  sample_rate
  channel_layout
  PCM frames
  optional spatial metadata
  optional object metadata
  timing / generation
        ↓
source presentation
        ↓
Omniphony renderer
        ↓
stereo binaural PCM
```

The portable core should not know about:

```text
WASAPI
ASIO
VB-Audio
Windows device names
Windows sessions
Core Audio
PipeWire
ALSA
APO registration
```

Those belong to platform hosts.

Windows is the first route to the product, not the product's identity.

Later ports should replace/adapt the host while preserving the same core semantics.

---

# 14. Product shell and zero-config law

The mature experience should be almost boring:

```text
install
→ Omniphony ON
→ play anything normally
```

Plausible shell:

```text
Omniphony       ON
Output          automatic
Headphones      automatic / saved
Mode            automatic
```

Advanced controls may later expose spatial strength, depth/extent, HRTF/personalization, headphone correction, head tracking and diagnostics, but routine playback must not require them.

---

# 15. Incumbent chain and migration law

The existing perceptual reference remains:

```text
foobar2000
→ SoX
→ optional Skip Silence
→ Vocal Exciter
→ Reverb
→ Upmix to 5.1/side
→ Advanced Limiter
→ Hi-Fi Cable
→ Equalizer APO + HeSuVi
→ DTS Virtual:X for speakers HRIR
→ ASIO Bridge
→ FiiO
→ Dan Clark Noire X
```

Known incumbent transport:

```text
8-channel virtual transport
48 kHz
24-bit
512-sample ASIO buffer
2-channel physical headphone output
```

This is preference evidence and a benchmark, not a design specification.

Useful incumbent sound functions include:

```text
large bubble
rear presence
subjective density / energy
bass/body reinforcement
strong level without obvious clipping
```

Do not claim the current Omniphony prototype has beaten those functions yet. The longer clean listen only establishes a mildly enhanced, fidelity-preserving Omniphony foundation. The incumbent remains the spatial/energy comparison oracle until repeated matched listening clearly says otherwise.

Migration law:

> **Disable before uninstall.**

Keep the incumbent installed until Omniphony has actually earned replacement.

---

# 16. Single-path and bypass law

Correct:

```text
source
→ Omniphony
→ physical headphones
```

Forbidden:

```text
source ───────────────→ physical headphones
   └→ Omniphony ─────→ physical headphones
```

Duplicate delayed copies can create comb filtering, thinness, echo and hallway coloration.

Bypass law:

> **OFF must be sample-route clean.**

OFF may not leave:

```text
queued wet blocks
stale room tail
secondary physical forwarding
duplicate dry path
renderer leakage
```

The current prototype restarts its hidden worker on ON/OFF so old queues die. A polished implementation should eventually select latency-aligned wet/dry near physical output.

---

# 17. Current live-listening evidence

## 17.1 Transport proof

The native Windows app successfully plays arbitrary live Windows audio through Omniphony to the physical FiiO/headphones.

```text
native app shell       proven
hidden worker          proven
process-loopback input proven
Omniphony engine       proven
physical output        proven
```

## 17.2 Duplicate-route contamination removed

With HeSuVi disabled and the old ASIO forwarding path stopped, OFF became clean.

This established a trustworthy single-path comparison.

## 17.3 Generic full-wet stereo rejected

The generic channel-bed HRTF route produced:

```text
tinny / cheap-phone tonal character
bass/body loss
less useful spatiality than dry stereo
less rear extent / definition
audible room / reverb quality
```

Frozen conclusion:

> **Replacing a finished stereo master wholesale with the generic channel-bed HRTF path is the wrong default music architecture.**

## 17.4 Upstream demo remains a known-spatial control

The real upstream bundled demo uses roughly:

```text
SAF / KEMAR
unit_scale_m = 3.0
early reflections around 0.4
short reverb around 0.2 / 0.3 s
```

That remains useful for a known spatial 7.1.4 scene.
It is not automatically the right stereo-music preset.

## 17.5 Preserved-master side-only path established the fidelity floor

Keeping the exact stereo master and adding only a small aligned side field removed the former tonal damage.

At low and even nominally high support strengths, ON and OFF were effectively indistinguishable.

That experiment succeeded as a **safety proof** but failed as a sufficient spatial decoder.

## 17.6 Windows 7.1 endpoint was a level confound

The temporary Hi-Fi Cable endpoint remained configured as Windows 7.1 while the music worker requested a 2ch loopback stream.

Changing the endpoint to Stereo / 2.0 restored normal playback level.

Do not confuse that transport finding with the stereo spatial algorithm.

## 17.7 Frequency-evidence path is the current protected music foundation

The frequency-aware portable field path replaced the side-only scaffold.

The first few minutes suggested a much larger rear-heavy field, but longer listening corrected that impression. The reliable result is:

```text
raw stereo clarity remains
no obvious tinny / hallway regression
music still feels intact
ON is only mildly enhanced versus OFF
```

This is enough to protect the architecture because it solves the destructive-fidelity problem while producing the beginning of useful enhancement.

It is **not** enough to declare victory over HeSuVi or to claim the 360° world is already strong.

The next sound work is therefore:

```text
make ON unmistakably larger than OFF
redistribute support into full 360° wrap rather than rear-only gain
strengthen front externalization
fill side/front-side continuity
add conservative height / upper-shell support
add radial depth
improve extent and ambient continuity
restore / exceed incumbent energy and punch
```

The active 7.1.4-shell successor is the first experiment in that direction.

From here compare every candidate against both:

```text
A. dry / OFF stereo
B. protected frequency-evidence fidelity floor
C. incumbent HeSuVi/DTS chain when practical
```

A candidate that sounds more dramatic than B but less musical loses.

---

# 18. Helix and libaural boundary

Helix is the research laboratory.

libaural is the reusable artificial-hearing research platform.

Their role is to discover and compress useful hearing mechanisms, not to own Omniphony's playback runtime.

```text
HELIX
exact listening / correction / research
        ↓
LIBAURAL
hearing mechanisms / controlled experiments
        ↓
DISTILLATION
what cue or invariance actually matters?
        ↓
SMALLEST USEFUL MECHANISM
        ↓
OMNIPHONY
only when objective checks + listening improve
```

Preferred:

```text
research
→ compression
→ stable mechanism
```

not:

```text
research
→ giant mandatory runtime
```

Helix music concepts such as line + field, relational continuity, bass-function plurality, pressure topology, temporal sovereignty and world formation primarily inform **what must be preserved and how to test it**.

---

# 19. Research promotion rule

External work enters the product only through:

```text
source / influence
→ mechanism / lesson
→ observed product relevance
→ bounded experiment
→ objective validation + listening
→ retain / narrow / reject
```

Useful research is always parked even when it does not enter current code.

Primary durable surfaces:

- `docs/influence-ledger.md`;
- `docs/headphone-rendering-research.md`;
- `docs/music-presentation-contract.md`;
- `docs/frequency-evidence-music-path.md`;
- `docs/windows-integration-research.md`.

The influence ledger preserves Stereo Convolver, MathAudio, Trifield, LCC, FreeSurround, Real3D, professional upmixers, stereo crossfeed/width tools, Ambisonic tooling, Steam Audio, Dolby references and realtime convolution literature.

---

# 20. Validation lanes

Keep failures attributable.

## Deterministic DSP

```text
compile
→ unit tests
→ deterministic fixtures
→ fidelity/null checks
```

## Known scene

```text
known spatial geometry
→ upstream-style Omniphony
→ binaural output
```

## Stereo presentation

```text
controlled stereo
→ protected direct identity
→ frequency-evidence support
→ binaural output
```

## Source-safe collapse

```text
stereo master
→ derived presentation state
→ defined collapse/downmix
→ compare source relationships
```

Track at least:

```text
center
L/R balance
bass
phase
transients
width
```

## Transport

```text
same engine / PCM
→ host route A
→ host route B
→ compare timing / glitches / latency
```

## Concurrent formats

```text
stereo only
surround only
stereo + surround
→ no global-layout collision
```

## Exact music moments

```text
exact object / timestamp
+ why it matters
+ near miss
+ negative control
→ matched-loudness A/B
→ retain / narrow / reject
```

---

# 21. Listening scorecard

Score dimensions separately:

```text
front externalization
rear discrimination
rear/front balance
side precision
side-wrap continuity
elevation
below-listener plausibility
radial distance
apparent source width
listener envelopment
source separation
source stability
ambient continuity
room audibility
transient clarity
vocal/direct solidity
center authority
timbral fidelity
bass stability / groove
stereo definition
interaural cue stability
microdetail
dynamics
energy / density
fatigue
bypass-collapse strength
```

Do not let loudness masquerade as spatial quality, but do not mistake an under-energized presentation for fidelity either.

---

# 22. Current milestone ladder

## W0 - protect upstream sound

Established for known spatial scenes.

## P0 - native protected listen

Heard successfully.

## P0.1 - arbitrary live Windows audio

Heard successfully.

## P0.2 - clean stereo path

Single physical path established. Generic full-wet stereo rendering was rejected as the music default because it damaged bass, timbre and useful stereo space.

## P0.3 - preserved-master fidelity floor

Established. The dry master can remain authoritative while Omniphony support is added without the former phone/hallway failure.

## P0.4 - frequency-evidence fidelity-preserving field

**CURRENT PROTECTED FOUNDATION.**

Established listening wins:

```text
raw clarity retained
music identity retained
mild spatial enhancement over OFF
no obvious room-smear requirement
```

Not yet established:

```text
clear superiority to HeSuVi
strong full-sphere effect
height
radial layering
strong bypass collapse
final energy/punch
```

## P0.5 - stronger 7.1.4 evidence shell

**ACTIVE EXPERIMENT.**

Purpose:

```text
same protected stereo master
+ stronger derived support
+ overlapping front/side/rear lanes
+ conservative height lanes
+ less concentration of diffuse evidence into rear only
```

This becomes a new foundation only if listening says it preserves P0.4 fidelity while making ON clearly and desirably different from OFF.

## P1 - excellent everyday stereo music

Sound target:

```text
full 360° coherent shell
front/side/rear all distinct but connected
convincing height and some lower-shell structure
near/mid/far depth
source extent without smear
ambient continuity
strong energy and bass physicality
raw-master clarity intact
bypass collapses the world rather than restoring the song
```

## P2 - owned Windows routing

Replace loopback/cable scaffolding with the best native route without changing the protected sound.

## P3 - native surround and richer spatial inputs

Preserve 5.1/7.1/height/object semantics directly and let them coexist with stereo streams.

## P4 - deeper stereo presentation

Several P4 concepts now underpin the active foundation:

```text
center-anchor protection            active
frequency-dependent field evidence  active
amplitude/phase evidence             active
direct/field evidence                active
```

Still to earn:

```text
better source extent
continuous full-sphere field formation
front/rear/side balance
height / lower-shell field support
radial-depth control
bounded cross-ear support
source-collapse/downmix invariance tests
optional Ambisonic field representation
optional libaural-derived mechanisms
```

## P5 - personalization

- headphone correction;
- HRTF selection/import;
- listener fitting;
- head tracking;
- specialist controls.

## Later - other OS hosts

Port the host. Do not fork a Windows-shaped core.

---

# 23. Product anti-goals

- Do not replace good upstream-derived Omniphony rendering with research theater.
- Do not discard the stereo master and reconstruct a worse version merely to spatialize it.
- Do not require virtual 7.1 for normal music.
- Do not make channel layout a global mode.
- Do not let a surround game reconfigure a stereo song.
- Do not force rich surround through stereo reconstruction.
- Do not equate reverb with 3D.
- Do not accept audible room coloration as the price of externalization.
- Do not use bass EQ to hide a structural bass-loss bug.
- Do not hollow the center to obtain width.
- Do not erase useful source ITD/ILD merely because synthesized HRTFs are available.
- Do not treat a derived FreeSurround/Real3D-style bed as authored truth.
- Do not hallucinate object truth from stereo.
- Do not make sources wander because an analyzer changed its mind.
- Do not make AI availability a playback dependency.
- Do not let Windows APIs define the portable core.
- Do not require device selection every launch.
- Do not uninstall the incumbent before replacement is proven.
- Do not accept dry + wet duplicate physical paths.
- Do not accept OFF with wet leakage.
- Do not mix an unaligned delayed support path against the direct master.
- Do not adopt convolution complexity merely because it exists.
- Do not let rear placement become the default definition of “spatial.”
- Do not turn high frequencies into fake height or low frequencies into fake floor cues by simple register mapping.
- Do not let an exciting first impression outrank longer clean listening.
- Do not forget parked research.

Use:

```text
actual weakness
→ smallest attributable experiment
→ measure
→ listen
→ keep only if earned
```

---

# 24. Re-entry checkpoint

If context is lost, recover this hierarchy:

```text
1. upstream-derived Omniphony is still the spatial-rendering heart
2. preserve and improve that sound, never casually replace it
3. turn it into an always-on platform-agnostic headphone product
4. Windows is only the first host
5. physical headphone output is ordinary 2.0
6. ordinary stereo music is the dominant use case
7. a finished stereo master is authoritative and must remain recognizable
8. generic full-wet virtual-speaker stereo failed listening
9. preserved-master direct + support architecture solved the destructive-fidelity failure
10. pure side-only support was too weak to produce the desired world
11. frequency-dependent amplitude/phase evidence is the protected stereo architecture
12. the proven sound floor is raw clarity + intact music + mild enhancement, not yet a huge sphere
13. early claims of being bigger than HeSuVi were corrected after longer listening and are not frozen facts
14. the incumbent remains the spatial/energy oracle until repeated comparison proves replacement
15. the active successor expands the evidence field into overlapping logical 7.1.4 with height and stronger support
16. that successor must preserve the P0.4 fidelity floor before promotion
17. next sound work is unmistakable 360° wrap, front externalization, height, lower shell, radial depth and continuity
18. energy/punch/density remain a separate sound dimension that must eventually equal or exceed the incumbent without clipping
19. bass/foundation, direct identity, center authority and source interaural cues remain protected
20. analysis from amplitude/phase/M-S is evidence, not authored truth
21. direct / broad / diffuse / room remain separate responsibilities
22. richer surround/object sources keep their real source truth
23. simultaneous streams keep independent layouts
24. HeSuVi is incumbent/reference, not architecture
25. migrate by disabling pieces before uninstalling them
26. one physical path only and OFF must be clean
27. Stereo Convolver's 2x2 matrix is a useful conceptual influence
28. Trifield contributes center-stability law
29. LCC contributes source ITD/ILD preservation law
30. FreeSurround/Real3D contribute analysis evidence but not output truth
31. Halo/Penteo contribute source-safe / reversibility constraints
32. MathAudio contributes bounded correction/crossfeed lessons, not a renderer replacement
33. Helix/libaural research must compress into small earned mechanisms
34. bypass should collapse the world, not restore the music
```

That is the current view of Omniphony for Headphones.
