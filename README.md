# Omniphony for Headphones

> **Private master project plan and canonical re-entry surface.**
>
> If conversational context disappears, recover the project from this README, recent `main` history, and the supporting contracts under `docs/` before inventing a new direction.

Omniphony for Headphones is a fork of [`mgth/Omniphony`](https://github.com/mgth/Omniphony) being turned into a **platform-agnostic, always-on headphone spatial processor**, with Windows as the first product host.

The central product law is simple:

> **Upstream Omniphony already has a very good spatial/binaural effect. Preserve and extend that percept. Do not replace it with research for research's sake.**

The job is primarily to make that spatial renderer usable as a universal listening layer for an operating system, then make it unusually good for the dominant real-world source: **ordinary mastered stereo music**.

The target is not merely wider stereo and not merely a HeSuVi clone.

> **The headphones should perceptually disappear and the listener should stand inside a coherent sphere of the same recording.**

Useful category shorthand:

> **an open-source, modern Sony-360-like headphone world that works with ordinary stereo, native surround, games, movies and richer spatial sources without requiring specially authored music.**

---

# 0. The inheritance map

This project is **not a ground-up replacement for Omniphony**.

The load-bearing acoustical renderer remains upstream Omniphony.

## Inherited upstream core

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
- the HRTF/HRIR providers and convolution machinery;
- ITD and head-pose handling;
- channel/object metadata and ramps;
- VBAP and speaker layouts;
- early reflections and FDN room support;
- bridge-based decoding boundaries;
- the headless renderer/engine structure;
- upstream Studio's useful separation between rendering and supervision.

That is the **heart** of the product.

## What this fork is adding around it

The fork's major ownership is increasingly:

```text
portable source contracts
stereo-music presentation
source-truth preservation
concurrent-stream semantics
Windows system routing
future platform adapters
always-on product shell
clean bypass / transport
fidelity regression tests
music-specific preservation rules
research/evaluation machinery
selective renderer corrections only when earned
```

The desired architecture is therefore:

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
                  UPSTREAM OMNIPHONY CORE
        HRTF + ITD + geometry + binaural rendering
                           │
                           ▼
                    BINAURAL STEREO
                           │
                           ▼
                  ordinary 2.0 headphones
```

A useful engineering rule follows:

> **When a problem can be solved by feeding the upstream renderer better material or by preserving the source around it, prefer that over replacing the renderer.**

There is no meaningful single code-line percentage for "how much is upstream" because host, tests and research code can dwarf a small DSP kernel. Acoustically, however, upstream Omniphony remains the load-bearing spatial renderer and should continue to do so unless a specific component demonstrably fails.

---

# 1. Perceptual north star

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

Ordinary DSP state is expected. Convolution histories, delay lines, interpolation, smoothing, room decay and future head tracking are normal realtime audio mechanisms.

The distinction is:

```text
PRE-AUTHORED IMMERSIVE QUALITY
without requiring
LIVE SEMANTIC REMIXING
```

The full-sphere target can contain:

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

# 2. Hard fidelity law

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

> **"the world collapsed."**

Not:

> **"the music came back."**

A new invariant discovered by the first live stereo tests is:

> **Omniphony ON may never be narrower, less defined, or tonally cheaper than the untouched stereo master merely to obtain spatial cues.**

This law outranks clever DSP, research elegance and feature count.

---

# 3. Main use case: ordinary stereo music

For most listeners, and for the primary private use case, the everyday source is normal 2-channel music.

The mature path should be:

```text
foobar / Spotify / browser / player
        ↓
finished stereo master
        ↓
Omniphony stereo presentation
        ↓
upstream Omniphony binaural renderer
        ↓
ordinary 2-channel DAC / headphones
```

The listener should not configure the operating system or physical DAC as 7.1 just to hear stereo music.

```text
physical headphone output = 2.0
```

The difficult product question is:

> **How can a finished stereo master gain convincing full-sphere physicality while remaining recognizably and tonally the same finished master?**

Do not answer that by blindly turning `L` and `R` into two replacement virtual speakers.

The first clean live tests showed why.

---

# 4. Stereo is authoritative, not raw material to discard

The current research direction is now sharper than a generic stereo upmixer.

A finished stereo recording already contains valuable authored information:

```text
left/right image
center authority
bass relationships
phase relationships
width
transients
ambience
reverberation
mix hierarchy
```

The default stereo presentation should preserve those properties and grow a spatial world around them.

Conceptual model:

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

This is not necessarily three literal stems. It is a design law about what must remain authoritative.

## 4.1 Foundation protection

Low-frequency energy is especially dangerous to spatialize indiscriminately.

Existing fork research already encodes a conservative `80-220 Hz` bass-protection transition in `scene_inference.rs`.

The product implication is stronger:

```text
bass / pressure / groove foundation
→ preserve first
→ spatially move only when evidence earns it
```

Do not fix bass loss after the fact with a compensating shelf if the renderer caused the loss structurally.

## 4.2 Direct versus field

Useful separation:

```text
DIRECT
identity / attack / center / definition / authored stereo image

FIELD
ambient / decorrelated / broad / environmental support
```

Environment is not a substitute for geometry, and geometry is not permission to replace the direct master.

---

# 5. Stereo convolution / 2x2 transfer-matrix influence

The foobar Stereo Convolver lineage and related convolution research suggest a useful mathematical model.

A general stereo convolver is a 2x2 transfer matrix:

```text
yL = L * HLL + R * HRL
yR = L * HLR + R * HRR
```

The key Omniphony interpretation is not "use a convolver plugin." It is:

```text
DIAGONAL TERMS
HLL / HRR
→ preserve source authority and direct identity

OFF-DIAGONAL TERMS
HLR / HRL
→ add controlled interaural / spatial support
```

A useful conceptual target is therefore:

```text
H(z) = I + S(z)
```

where `I` is the protected stereo identity and `S(z)` is only the spatial support that earns its existence.

This is a **research architecture**, not a frozen implementation formula.

The important law is:

> **The master can remain explicitly present in the transfer function instead of being deleted and synthesized again.**

That is much closer to the product goal than "replace stereo with virtual speakers."

---

# 6. Direct, ambient and room should be separable

External research independently reinforces a three-way split:

```text
DIRECT
precise / short / identity-bearing

AMBIENT FIELD
broad / diffuse / spatial support

ROOM / REVERBERANT FIELD
longer temporal structure
```

Useful influences include:

- 3D Tune-In Toolkit: anechoic HRIR/ITD path separated from reverberation;
- Google's Open Binaural Renderer: distinct Direct, Ambient and Reverberant filter profiles;
- Steam Audio: direct, reflections and late environment are independent mechanisms;
- stereo convolution systems: explicit cross-channel transfer rather than one opaque wet effect.

Product consequence:

> **Do not make the whole stereo master pass through an audible room just to externalize it.**

For music, the default room contribution may be tiny or absent. Spatial structure should survive without "hallway" coloration.

---

# 7. MathAudio lessons, bounded carefully

MathAudio's foobar DSP lineage is useful as a design influence, not as an implementation dependency.

Useful lessons:

- headphone correction and spatial presentation are different problems;
- crossfeed demonstrates that natural interaural coupling can be useful;
- correction should be bounded rather than blindly inverting every measured error;
- deep notches and unstable inverse filters should not be "fixed" merely because mathematics allows it;
- phase, transient behavior and pre-ringing are audible product concerns.

Do **not** inherit the broad claim that FIR convolution itself is inherently undesirable.

Realtime auralization literature and multiple open binaural systems use partitioned convolution successfully. The useful distinction is:

```text
not: FIR bad

but:
poorly constrained phase correction
long inappropriate kernels
bad transitions
or bad latency architecture
→ can sound bad
```

---

# 8. Convolution engineering law

If longer convolution becomes useful, prefer a realtime architecture designed for it.

Research sources support:

```text
small early partitions
→ low latency

larger later partitions
→ efficient long tail
```

Non-uniform partitioned convolution is especially attractive for future direct/ambient/room separation.

Potential implementation references are parked in `docs/influence-ledger.md`, including `HiFi-LoFi/FFTConvolver` and real-time auralization literature.

This is future machinery. Do not replace the current short upstream HRIR kernel merely because a longer convolver exists.

---

# 9. Universal source-truth law

Inference decreases as source truth increases.

```text
STEREO
finished presentation but little explicit full-sphere truth
→ preserve master
→ add only validated support

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

---

# 10. Concurrent-stream law

Channel layout belongs to a logical source/stream, not to Omniphony globally.

A desktop may contain:

```text
foobar       stereo 2.0
Overwatch    native 5.1 / 7.1 / spatial
voice chat   mono / stereo
future app   object / richer spatial metadata
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

This becomes a permanent regression case:

```text
stereo alone
surround alone
stereo + surround simultaneously
→ all remain stable and correctly interpreted
```

The temporary Windows loopback prototype cannot yet preserve application boundaries this way. The portable core contract should.

---

# 11. Portable core law

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

# 12. Product shell and zero-config law

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

Advanced controls can exist later for:

- spatial strength;
- depth / extent;
- HRTF / personalization;
- headphone correction;
- head tracking;
- diagnostics;
- specialist routes.

They must not become routine setup requirements.

---

# 13. Current incumbent and migration law

The existing reference remains:

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

Migration law:

> **Disable before uninstall.**

Keep the complicated incumbent installed until Omniphony actually replaces each function.

During trustworthy listening tests there must be **one physical audible path only**.

---

# 14. Single-path and bypass law

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

The current prototype restarts its hidden worker on ON/OFF to guarantee old queues die. A later polished implementation should use latency-aligned wet/dry selection near physical output.

---

# 15. Current live-listening evidence

## 15.1 Transport proof

The native Windows app successfully plays arbitrary live Windows audio through Omniphony to the physical FiiO/headphones.

```text
native app shell       proven
hidden worker          proven
process-loopback input proven
Omniphony engine       proven
physical output        proven
```

## 15.2 Duplicate-route contamination was removed

With HeSuVi disabled and the old ASIO forwarding path stopped, OFF became clean.

That strongly reduced the likelihood that the remaining wet-path failure was only double playback.

## 15.3 Wet-path failure

Clean ON versus OFF produced:

```text
ON:
- tinny / cheap-phone tonal character
- bass/body loss
- less spatial than dry stereo
- less rear extent / definition than dry stereo
- audible room/reverb quality

OFF:
- clean
- fuller
- more defined
- surprisingly more spatial in useful ways
```

This is now a trustworthy product finding:

> **Replacing a finished stereo master with the generic channel-bed HRTF path is the wrong default music architecture.**

## 15.4 Upstream-demo config correction

The local upstream reference had also drifted from upstream's real bundled demo.

The actual upstream demo uses roughly:

```text
SAF / KEMAR
unit_scale_m = 3.0
early reflections enabled around 0.4
short reverb around 0.2 / 0.3 s
```

The protected control was corrected.

That configuration remains useful for a **known spatial 7.1.4 scene**.

It is not automatically the right preset for ordinary stereo music.

This distinction is now explicit:

```text
known spatial scene
→ upstream-style full binaural presentation

finished stereo music
→ preserve master
→ add controlled Omniphony spatial support
```

---

# 16. Current stereo prototype direction

The current Windows prototype is stereo-first.

Normal capture requests 2 channels before richer bed formats.

The old 8/6-channel-first route remains only as a diagnostic `--rich-bed` path until the host can preserve per-application source truth.

A dedicated stereo config exists separately from the upstream known-scene reference.

The next experimental architecture is:

```text
                 FINISHED STEREO MASTER
                         │
             ┌───────────┴───────────┐
             │                       │
             ▼                       ▼
       DIRECT IDENTITY          SPATIAL SUPPORT
       remains audible          derived conservatively
             │                       │
             │                upstream Omniphony
             │                 binaural machinery
             │                       │
             └───────────┬───────────┘
                         ▼
                     headphones
```

The first implementation may live in the Windows listening host as an **experiment only** so it can be rejected cheaply.

If listening validates the structure, promote the mechanism into the portable presentation layer rather than leaving product semantics in Windows code.

---

# 17. Protected renderer vocabulary

Useful distinctions remain:

```text
FrontalAnchor
DirectObject
BroadSource
DiffuseField
RoomField
```

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

Stereo inference may provide evidence for presentation, but it may not claim authored object truth that the master does not contain.

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

Useful Helix music concepts include line + field, identity under transformation, role legibility, bass-function plurality, pressure topology, closure latency, temporal sovereignty and world formation.

They primarily inform **what the renderer must preserve and how to test it**.

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
- `docs/windows-integration-research.md`.

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
→ bounded spatial support
→ binaural output
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
side precision
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
timbral fidelity
bass stability / groove
stereo definition
microdetail
dynamics
fatigue
bypass-collapse strength
```

Do not let loudness masquerade as quality.

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

## P0.3 - preserve-master + spatial-support prototype

**CURRENT.**

Goal:

```text
untouched stereo identity remains authoritative
+
small controlled Omniphony-derived spatial field
```

No audible reverb default.
No bass destruction.
No narrowing relative to dry stereo.

## P1 - excellent everyday stereo music

- persistent reliable route;
- automatic output;
- clean bypass;
- strong full-sphere improvement;
- no developer ritual;
- no need to dismantle incumbent prematurely.

## P2 - owned Windows routing

Replace loopback/cable scaffolding with the best native route.

Requirements:

- single physical path;
- clean install/remove/update;
- rich-source preservation;
- no per-launch device ritual;
- session/layout coexistence.

## P3 - native surround and richer spatial inputs

Preserve 5.1/7.1/height/object semantics directly and let them coexist with stereo streams.

## P4 - deeper stereo presentation

Only after the protected-master architecture is proven:

- direct/field evidence;
- smarter frequency-dependent support;
- source extent;
- controlled full-sphere field formation;
- optional libaural-derived mechanisms.

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

- Do not replace good upstream Omniphony rendering with research theater.
- Do not discard the stereo master and reconstruct a worse version merely to spatialize it.
- Do not require virtual 7.1 for normal music.
- Do not make channel layout a global mode.
- Do not let a surround game reconfigure a stereo song.
- Do not force rich surround through stereo reconstruction.
- Do not equate reverb with 3D.
- Do not accept audible room coloration as the price of externalization.
- Do not use bass EQ to hide a structural bass-loss bug.
- Do not hallucinate object truth from stereo.
- Do not make sources wander because an analyzer changed its mind.
- Do not make AI availability a playback dependency.
- Do not let Windows APIs define the portable core.
- Do not require device selection every launch.
- Do not uninstall the incumbent before replacement is proven.
- Do not accept dry + wet duplicate physical paths.
- Do not accept OFF with wet leakage.
- Do not adopt convolution complexity merely because it exists.
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
1. upstream Omniphony is still the spatial-rendering heart
2. preserve and improve that sound, never casually replace it
3. turn it into an always-on platform-agnostic headphone product
4. Windows is only the first host
5. physical headphone output is ordinary 2.0
6. ordinary stereo music is the dominant use case
7. a finished stereo master is authoritative and must remain recognizable
8. build spatial support around the master rather than deleting it
9. direct / field / room are separate responsibilities
10. richer surround/object sources keep their real source truth
11. simultaneous streams keep independent layouts
12. HeSuVi is incumbent/reference, not architecture
13. migrate by disabling pieces before uninstalling them
14. one physical path only and OFF must be clean
15. the generic full-wet stereo virtual-speaker approach failed listening
16. current frontier is preserved-direct stereo + bounded Omniphony spatial support
17. Stereo Convolver's 2x2 matrix is a useful conceptual influence
18. MathAudio contributes bounded crossfeed/correction lessons, not a renderer replacement
19. Helix/libaural research must compress into small earned mechanisms
20. bypass should collapse the world, not restore the music
```

That is the current view of Omniphony for Headphones.
