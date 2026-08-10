# Omniphony for Headphones

> **Private master project plan and canonical re-entry surface.**
>
> If conversational context disappears, recover the project from this README, recent `main` history, and the supporting contracts under `docs/` before inventing a new direction.

Omniphony for Headphones is a fork of [`mgth/Omniphony`](https://github.com/mgth/Omniphony) being turned into a **platform-agnostic universal headphone spatial processor**, with Windows as the first host implementation.

The foundation is unusually strong:

> **Upstream Omniphony already has a convincing binaural/spatial percept. Preserve that floor, make it practical for normal listening, and improve it only when the improvement earns itself.**

The target is broader than a HeSuVi clone and broader than a conventional upmixer.

Useful shorthand:

> **a modern open-source Sony-360-like headphone world that works intelligently with ordinary stereo, native surround, games, movies and richer spatial sources instead of requiring specially authored music.**

Another useful comparison:

> **an open-source alternative to HeSuVi / commercial headphone spatializers, but built around a reusable spatial renderer rather than a fixed virtual-speaker HRIR pipeline.**

Those comparisons describe the category. They do not define the architecture.

---

# 0. Perceptual north star

The product goal is not merely wider stereo.

> **The headphones perceptually disappear and the listener stands inside a coherent sphere of sound.**

That sphere may contain:

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

The strongest music aspiration is:

> **A song should sound as though an elite immersive mix/mastering engineer had already prepared that exact recording for this full-sphere headphone presentation before playback began.**

This does **not** mean an engineer-like runtime riding faders while playback happens.

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
constantly reclassified
obviously controlled by an algorithm
```

The processor may be stateful because audio DSP is stateful. Convolution, smoothing, interpolation, reflections, room decay, source trajectories and head tracking all require state.

But the musical presentation should feel as if it was already mixed this way.

```text
PRE-AUTHORED IMMERSIVE QUALITY
without requiring
LIVE SEMANTIC REMIXING
```

After acclimation, ordinary headphone playback should ideally feel dimensionally collapsed by comparison.

That is an aspiration, not a literal `100×` measurement claim.

---

# 1. Hard fidelity law

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
comfort
```

Desired bypass reaction:

> **“the world collapsed.”**

Not:

> **“the music came back.”**

This law outranks clever DSP, research elegance and feature count.

---

# 2. Product/core/platform hierarchy

The correct hierarchy is now:

```text
UPSTREAM OMNIPHONY
already-good spatial/binaural percept
        ↓
PROTECTED PERCEPTUAL FLOOR
must remain reproducible
        ↓
OMNIPHONY PORTABLE CORE
source contracts + presentation + renderer + binaural output
        ↓
PLATFORM HOSTS
Windows first; other platforms later if the product earns them
        ↓
CONTROLLED IMPROVEMENTS
only where tests/listening expose a weakness
        ↓
DISTILLED RESEARCH MECHANISMS
Helix / libaural only when they earn their way in
```

Do **not** reverse this into:

```text
Windows APIs
→ define the core model
```

or:

```text
Helix research
→ giant libaural runtime
→ elaborate music reasoning
→ speculative scene stack
→ someday hope Omniphony still sounds good
```

Omniphony is a **processor/product**, not the runtime manifestation of the entire research lab.

Windows is the first route to the product, not the identity of the product.

---

# 3. Portable core law

The core should conceptually accept platform-neutral logical streams and emit binaural stereo.

```text
InputStream
  id
  sample_rate
  channel_layout
  PCM / source frames
  optional spatial metadata
  optional object metadata
  stream generation / timing
        ↓
Omniphony presentation + scene + renderer
        ↓
Stereo binaural PCM
```

The portable core should not know about:

```text
WASAPI
ASIO
VB-Audio
Windows device names
Core Audio
PipeWire
ALSA
APO registration
Windows sessions
```

Those belong to platform adapters.

The same renderer semantics should be usable by:

```text
Windows host
future macOS host
future Linux host
file/reference harness
DAW/plugin host if useful
specialist ASIO route
```

Portability is preserved by architecture now, even though Windows remains the only active product host.

---

# 4. Main use case: ordinary stereo music

For most people, and for the primary private use case, the everyday source is ordinary stereo music.

The normal finished path should conceptually be:

```text
foobar / Spotify / browser / player
        ↓
ordinary stereo source
        ↓
Omniphony stereo presentation
        ↓
Omniphony binaural renderer
        ↓
ordinary 2-channel DAC/headphones
```

The listener should not need to configure Windows or the physical DAC as 7.1 merely to hear stereo music correctly.

The physical headphone output is naturally binaural stereo.

```text
physical output
= 2.0
```

Richer internal/source formats do not change that.

The difficult and distinctive long-term problem is therefore:

> **How can ordinary mastered stereo become a convincing full-sphere headphone world while still feeling like the same finished recording?**

Do not solve that by blindly manufacturing fake speaker channels merely because older virtual-surround systems are speaker-shaped.

---

# 5. Universal input and source-truth law

The amount of inference should depend on how much authoritative spatial truth each source already provides.

```text
STEREO
little explicit full-sphere truth
→ preserve the mastered image
→ add only validated presentation structure

5.1 / 7.1
real directional channel truth already exists
→ preserve it
→ render it beautifully

5.1.2 / 7.1.4 / HEIGHT BEDS
real elevation information exists
→ preserve it
→ render the authored sphere

OBJECT AUDIO
actual positions exist
→ do not infer what is already known
→ render objects directly

ALREADY-BINAURAL / VIRTUALIZED AUDIO
headphone spatial cues already exist
→ avoid destructive double virtualization
```

The intelligent behavior is often **doing less** when the source is richer.

Source truth outranks reconstruction.

---

# 6. Concurrent-stream law

Channel layout belongs to a **source/stream**, not to Omniphony globally.

A real desktop can contain several sources at once:

```text
foobar
  stereo 2.0

Overwatch
  native surround / home-theater bed

voice/chat
  mono or stereo

future spatial/object application
  richer metadata
```

These must be allowed to coexist.

Correct conceptual model:

```text
Stream A { layout = stereo }
Stream B { layout = 7.1 }
Stream C { layout = mono }
Object stream D { richer spatial state }
        ↓
shared Omniphony timeline/world
        ↓
binaural stereo output
```

Wrong model:

```text
Omniphony global mode = stereo
or
Omniphony global mode = 7.1
```

Starting a 7.1 game must not reconfigure a playing stereo song into some different global interpretation.

Likewise, stereo music playing beside a surround game must not flatten the game's richer source truth.

The first implementation may receive an already-mixed platform bed. The final core contract should still preserve per-stream layout so a platform adapter can expose richer source boundaries when the operating system makes them available.

This is a core invariant and a future regression case:

```text
stereo music alone
native surround alone
stereo music + native surround simultaneously
→ all remain stable
```

---

# 7. Source-truth hierarchy

When richer trustworthy information exists, preserve it.

Conceptual order:

```text
authored object metadata / rich scene
        ↓
native Ambisonics / HOA
        ↓
discrete authored stems / layers
        ↓
height / surround beds
        ↓
stereo
        ↓
mono
```

Controlled research may deliberately collapse rich scenes and test recovery. That is an experiment, not the normal playback law.

---

# 8. Renderer vocabulary without scene-model tyranny

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

These distinctions prevent all spaciousness from degenerating into reverb/decorrelation.

They are presentation vocabulary, not claims that a stereo master contained hidden authored object metadata.

---

# 9. Product promise and zero-config law

The mature experience should be almost boring:

```text
install
→ Omniphony ON
→ play anything normally
```

A plausible shell:

```text
Omniphony        ON

Output           automatic
Headphones       automatic / saved
Mode             automatic
```

Advanced users may later get optional controls for:

- spatial strength;
- apparent room/externalization;
- depth;
- source extent;
- HRTF / personalization;
- headphone correction;
- head tracking;
- diagnostics;
- specialist input/output routes.

Those controls must not become setup requirements.

Routine playback must not ask the listener to select devices every launch.

---

# 10. The two references Omniphony must preserve or beat

## 10.1 Upstream Omniphony = perceptual ancestor

The hosted upstream headphone demo is the primary oracle for the renderer's starting spatial character.

Protected local approximation:

```text
omniphony-renderer/assets/binaural-baselines/upstream-demo-reference.yaml
```

Approximate published ingredients:

```text
stock-style Omniphony defaults
+ SAF/KEMAR HRTF
+ early reflections enabled
+ late reverb disabled
```

The hosted site does not pin the exact render commit/command, so the local control is not claimed byte-identical.

A richer fork configuration that sounds worse at matched loudness loses.

## 10.2 Existing HeSuVi chain = end-to-end incumbent

The real daily listening system remains the second reference.

Omniphony does not graduate because it beats dry stereo. It should eventually make the tuned incumbent feel unnecessary.

Keep separate:

```text
Did the Omniphony renderer improve?

and

Would this finished product replace the current listening system?
```

---

# 11. Current incumbent snapshot

Restored from the actual Windows system on 2026-08-10.

This is evidence, not a specification to clone.

## foobar2000 DSP order

```text
Resampler (SoX)
→ Skip Silence (alternative)
→ Vocal Exciter
→ Reverb
→ Upmix to 5.1/side
→ Advanced Limiter
```

Active upmix channels:

```text
FL FR C LFE SL SR
```

## Virtual transport

```text
VB-Audio ASIO Bridge / Hi-Fi Cable
8ch transport
48 kHz
24-bit
512 sample buffer
FiiO ASIO Driver
2ch physical output
```

## HeSuVi / Equalizer APO reference

Current HRIR reference:

```text
DTS Virtual:X for speakers
Original-unmodified file, DTS Inc.
version shown: 2025.3.16.0
```

Observed matrix state:

```text
Stereo upmix: enabled
5.1 upmix:    enabled
Content:      Automatic
```

Observed position adjustments:

```text
front -5
side  +5
rear -15
```

Observed levels:

```text
Master  90
Center 100
Front  100
Side   100
Rear   100
LFE    200
```

## Hardware reference

```text
FiiO K7 / current FiiO Windows endpoint
→ Dan Clark Noire X
```

A resolving chain is useful because it exposes false spaciousness, phase smear, softened transients, HRTF coloration, weak bass timing and unstable localization.

---

# 12. Migration law: disable before uninstall

The existing Hi-Fi / HeSuVi route was difficult to assemble and remains the incumbent/reference.

Do **not** require uninstalling it while Omniphony is still proving itself.

Use an incremental migration:

```text
existing system remains installed
        ↓
disable one active stage
        ↓
Omniphony replaces that stage
        ↓
verify route + sound
        ↓
remove old component only after it is truly obsolete
```

For current development, keeping these installed is acceptable:

```text
Hi-Fi Cable
Equalizer APO
HeSuVi
ASIO Bridge
FiiO ASIO driver
```

But only **one physical audible path** may be active during a trustworthy listening comparison.

---

# 13. Single-path and bypass law

Ordinary playback must reach the listener exactly once.

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

Two copies separated by even a small delay can produce phase/comb-filter effects that sound thin, hollow, echoey or hallway-like.

Therefore:

> **No perceptual judgment is valid until the physical path is proven single.**

Bypass has an additional hard law:

> **OFF must be sample-route clean.**

OFF may not leave:

```text
queued wet blocks
room tail from a stale selected path
secondary physical forwarding
duplicate dry path
renderer leakage
```

A future polished bypass should be latency-aligned and transition cleanly while preserving useful renderer state.

The current prototype still needs work here before subtle A/B judgments are trusted.

---

# 14. First live Windows result: transport works, quality verdict deferred

On 2026-08-10 the native app prototype successfully played arbitrary live Windows/foobar audio through Omniphony to the real headphones.

That proves:

```text
native app shell
+ hidden worker
+ live Windows capture
+ protected Omniphony engine
+ physical FiiO output
= functioning end-to-end path
```

Observed first-listen report:

```text
sound plays
larger than flat in principle
but currently:
- tinny
- not very bubble-like
- hallway-like
- small echo remained after OFF
```

Do **not** treat those qualities as a renderer verdict yet.

The listener intentionally changed only one part of the incumbent system: HeSuVi was disabled. Hi-Fi Cable, virtual 7.1 state and the rest of the complicated route remained installed/configured.

Strong current hypothesis:

```text
old ASIO/forwarding path may still have reached FiiO
+
Omniphony also reached FiiO
→ duplicate slightly delayed copies
→ phase / comb-filter / echo coloration
```

There is also a known prototype-level bypass weakness: wet audio can be selected before it enters a bounded playback queue, so switching OFF can allow already-queued wet samples to emerge briefly.

Both must be removed before evaluating tonal/spatial quality.

Current evidence state:

```text
LIVE TRANSPORT = PROVEN
CLEAN SINGLE-PATH A/B = NOT YET PROVEN
RENDERER QUALITY ON REAL MUSIC = NOT YET FAIRLY JUDGED
```

---

# 15. Current Windows prototype

User-facing prototype:

```text
Omniphony.exe
```

Architecture:

```text
Omniphony.exe
        ↓
hidden omniphony_worker.exe
        ↓
Windows process-loopback capture
        ↓
protected Omniphony renderer
        ↓
automatically preferred FiiO output
```

The GUI/worker split is worth keeping.

Future platform routing can change without rebuilding the product concept:

```text
GUI / settings / tray
        ↓
platform host/worker boundary
        ↓
portable Omniphony core
```

Current loopback/cable plumbing is scaffolding, not the final route.

---

# 16. Windows is first host, not core

The active engineering target remains Windows because that is the current machine and listening environment.

Correct rule:

> **Build the Windows host first while preserving a platform-neutral Omniphony core.**

Do not divert current work into macOS/Linux shells.

Also do not bake Windows assumptions into the renderer merely because Windows is where the first product is being proven.

The Windows host should use native Windows audio facilities as much as practical for:

```text
session discovery
format/layout discovery
device/output ownership
clocking
capture/interception
endpoint changes
recovery
```

Then translate that state into portable Omniphony stream contracts.

The exact final Windows integration mechanism is not frozen yet.

Possible classes include:

```text
owned virtual endpoint
native system-effect/integration path
session-aware host routing
hybrid approach
```

Choose from measured reliability, latency, rich-source preservation, installability and user experience.

Windows API choices are host decisions, not core architecture votes.

---

# 17. Rich surround is a better source, not a separate product mode

Native surround should feed directly into Omniphony whenever the host can preserve it.

```text
game / film / multichannel music
→ authored 5.1 / 7.1 / height / object truth
→ Omniphony
→ stronger externalization / depth / height / continuity
→ binaural stereo headphones
```

Do not flatten a real rich source to stereo and then ask the stereo inference layer to rediscover it.

For ordinary music, however, stereo remains the dominant path.

So the priority is:

```text
1. excellent stereo music
2. preserve and enhance native surround when present
3. allow both simultaneously
```

---

# 18. HeSuVi relationship

HeSuVi remains an important influence/reference because it demonstrates both capability and UX limits.

Useful lesson:

```text
STEAL THE CAPABILITY
NOT THE CONFIGURATION BURDEN
```

Traditional HeSuVi-style topology is approximately:

```text
stereo / surround
→ speaker-channel matrix
→ fixed virtual-speaker HRIR responses
→ binaural headphones
```

Omniphony can eventually operate more directly:

```text
source truth / conservative presentation state
→ continuous spatial renderer
→ binaural headphones
```

The HeSuVi archive remains useful for studying:

- HRIR families;
- virtualizer behavior;
- channel matrices;
- position/level manipulation;
- headphone-EQ approaches;
- Windows routing conventions.

Do not redistribute proprietary HRIR material without appropriate rights.

---

# 19. Helix and libaural relationship

Helix is the research laboratory.

libaural is the separate reusable artificial-hearing project.

Their role is to improve mechanisms available to Omniphony, not to own its runtime architecture.

```text
HELIX
exact listening / music research / correction
        ↓
LIBAURAL
hearing mechanisms / representations / controlled experiments
        ↓
DISTILLATION
what cue, invariance or failure actually matters?
        ↓
SMALLEST USEFUL MECHANISM
        ↓
OMNIPHONY
only if objective checks + listening improve
```

Preferred direction:

```text
research
→ compression
→ stable mechanism
```

not:

```text
research
→ mandatory giant runtime
```

A rich model may remain a teacher.

A cheap deterministic mechanism that preserves the useful percept is often the better product implementation.

---

# 20. Helix music research: what transfers

Useful research coordinates include:

```text
line + field
identity under transformation
role legibility / role elasticity
synchronization under heterogeneity
bass-function plurality
pressure topology
closure latency
temporal sovereignty
world formation
```

They are useful for:

- asking better questions;
- exact-moment listening tests;
- finding failure modes;
- teaching libaural what relations may matter;
- building negative controls;
- discovering invariants spatial processing must preserve.

They are **not automatically runtime modules**.

Direct product laws are simpler:

```text
protect musical identity
protect center authority
protect groove / transient timing
protect bass function
protect pressure / weight where it matters
keep direct / broad / diffuse / room distinct
do not spread everything merely because the renderer can
make processing feel authored rather than algorithmically busy
```

---

# 21. Music presentation law

The default product should **enhance music dramatically without sounding like it remixed the song**.

```text
MASTERED RECORDING
        ↓
preserve identity / timing / hierarchy
        ↓
give the existing relational world convincing physical geometry
        ↓
OMNIPHONY BINAURAL RENDERER
        ↓
INHABITABLE HEADPHONE SPHERE
```

This is an evaluation philosophy, not a requirement to run dynamic musicological inference.

No audible scene rethinking.
No gratuitous source teleportation.
No chorus detector moving things because a chorus began.
No algorithm showing off.

See `docs/music-presentation-contract.md`.

---

# 22. Existing renderer foundation

Do not rewrite useful inherited machinery for aesthetics.

Retained substrate includes:

- stateful binaural DSP;
- analytic ITD;
- interpolated HRTF/HRIR rendering;
- embedded SAF/KEMAR, parametric and SOFA-capable providers;
- moving-filter crossfades;
- object position/size state;
- VBAP/layout machinery for known scenes;
- early image-source reflections;
- late FDN room-field machinery;
- deterministic DSP fixtures;
- headless engine/FFI boundaries.

Fork work already includes or has absorbed:

- complex M/S stereo evidence;
- persistence-aware evidence;
- conservative object/field separation;
- bass/foundation protection distinct from object identity;
- deterministic asynchronous HRTF switching;
- stale HRIR-build rejection;
- measured-HRIR direct-arrival validation;
- per-ear directional early-reflection timing;
- sample-time-oriented FDN modulation;
- true zero predelay;
- reusable fidelity metrics;
- optional upstream spectral-phantom extraction;
- optional distance-diffuse mirror-axis behavior;
- upstream runtime-isolation mechanisms.

These are capabilities, not a demand that every feature become part of the default sound.

---

# 23. Protected binaural controls

Directory:

```text
omniphony-renderer/assets/binaural-baselines/
```

### `upstream-demo-reference.yaml`

Perceptual ancestor. Minimal stock-style approximation of the hosted upstream demo.

### `baseline-room.yaml`

Fork room-assisted comparison. More DSP does not make it superior by definition.

### `dry-binaural.yaml`

HRTF/scale/air policy with room effects disabled, useful for isolating room contribution.

Experimental algorithms get explicit configs/flags. Never overwrite the protected control to make a candidate look better.

---

# 24. Realtime law

> **Host callback size is an implementation detail, not a coordinate system for the auditory world.**

Gain, movement, HRTF transitions, bypass, room changes and other intended continuous state belong in sample/time coordinates.

The same semantic engine should behave consistently across platform hosts.

Heavy work stays off realtime callbacks.

Bypass deserves special treatment:

```text
processed path
raw / comparison path
        ↓
selection as close to physical output as practical
        ↓
no stale queued wet tail
```

See `docs/realtime-control-contract.md`.

---

# 25. Validation lanes

Keep failures attributable.

## Lane A · compiler / deterministic DSP

```text
compile
→ unit tests
→ deterministic fixtures
→ fidelity/null checks
```

## Lane B · known scene → binaural

```text
known geometry
→ HRTF / ITD
→ extent / room
→ binaural output
```

## Lane C · stereo presentation

```text
controlled stereo
→ evidence
→ bounded presentation
→ protected renderer
```

## Lane D · platform transport

```text
same PCM / engine
→ host route A
→ host route B
→ compare timing / glitches / latency
```

## Lane E · upstream perceptual control

```text
upstream-style reference
↔ fork candidate
```

## Lane F · incumbent A/B

```text
CURRENT
foobar + VB-Audio + HeSuVi + ASIO

VERSUS

TARGET
Omniphony + physical FiiO + Noire X
```

## Lane G · concurrent source formats

```text
stereo only
surround only
stereo + surround simultaneously
→ no layout collision / global-mode bug
```

## Lane H · exact music moments

Helix-derived evaluation can preserve exact moments where a musical relation matters.

---

# 26. Listening scorecard

Score dimensions independently:

```text
front externalization
rear discrimination
side precision
elevation
below-listener plausibility
radial distance
apparent source width
listener envelopment
source extent
source separation
source stability
room presence / scale
ambient continuity
transient clarity
vocal/direct solidity
timbral fidelity
bass stability / groove
microdetail
dynamics
fatigue
bypass-collapse strength
```

Loudness-match comparisons.

Do not turn gain into perceived quality.

Do not score a candidate while duplicate physical routes are active.

---

# 27. Research parking rule

Useful external findings must be written into the repo even when not promoted into current code.

```text
source / influence
→ mechanism / lesson
→ observed product relevance
→ bounded experiment
→ objective validation + listening
→ adopt only if preserve / beat baseline
```

Durable research surfaces include:

- `docs/influence-ledger.md`;
- `docs/windows-integration-research.md`;
- `docs/headphone-rendering-research.md`;
- `docs/music-presentation-contract.md`.

External projects are mechanism sources and benchmarks, not architecture votes.

---

# 28. Current milestone ladder

## W0 · Protect/reproduce upstream sound — ESTABLISHED

Protected configs and deterministic known-scene paths exist.

## P0 · First native protected listen — HEARD

Native Windows output plus protected Omniphony rendering has been physically heard.

## P0.1 · Arbitrary live Windows audio — HEARD / ROUTE NOT CLEAN YET

The native app now plays arbitrary Windows audio through Omniphony.

Remaining requirement before judging sound:

```text
prove one physical path
+ clean bypass
```

## P0.2 · Clean stereo music baseline — NEXT

Use the existing system without uninstalling it, but deactivate competing forwarding:

```text
HeSuVi disabled
ASIO Bridge / old forwarding disabled
Hi-Fi Cable remains installed
ordinary stereo source
→ Omniphony only
→ FiiO
```

Then test dry stereo versus base Omniphony at matched loudness.

## P1 · Easy everyday Windows listening

- reliable persistent realtime path;
- automatic endpoint behavior;
- clean ON/OFF;
- ordinary stereo music without developer rituals;
- no dependence on uninstalling the incumbent during migration.

## P2 · Owned Windows routing

Replace development cable/loopback scaffolding with the best native Windows route.

Requirements:

- single physical path;
- rich source preservation;
- clean install/remove/update;
- no per-launch configuration;
- concurrent layouts do not collide.

## P3 · Native surround / rich spatial inputs

Preserve 5.1/7.1/height/object semantics directly and allow them to coexist with ordinary stereo streams.

## P4 · Better automatic stereo presentation

Improve stereo → full-sphere presentation without making the recording sound remixed.

## P5 · Calibration / personalization

- headphone profiles;
- optional correction;
- HRTF selection/import;
- listener personalization;
- head tracking where useful;
- deeper libaural-derived mechanisms only when earned.

## Later · Other platform hosts

If the Windows product proves compelling, port the **host**, not a Windows-shaped core.

---

# 29. Current immediate test sequence

Do not uninstall the incumbent yet.

Next trustworthy sequence:

```text
1. keep Hi-Fi Cable installed
2. keep the old virtual configuration intact for reversibility
3. disable HeSuVi
4. stop/disable ASIO Bridge or any old physical forwarding to FiiO
5. confirm Omniphony is the only process producing audible FiiO output
6. test stereo music
7. fix any remaining bypass leakage
8. only then judge tonal/spatial quality
9. then test native surround alone
10. then stereo + surround simultaneously
```

After the route is clean, start removing old scaffolding piece-by-piece only when Omniphony has replaced its function.

---

# 30. Product anti-goals

- Do not replace good sound with research.
- Do not make the user configure routing every launch.
- Do not require virtual 7.1 for ordinary stereo listening.
- Do not make channel layout a global product mode.
- Do not let a surround game reconfigure a playing stereo song.
- Do not build a settings forest.
- Do not turn music into an AI remix.
- Do not make placements wander because a classifier changed its mind.
- Do not hallucinate object truth from stereo.
- Do not equate reverb with 3D.
- Do not make AI/model availability a playback dependency.
- Do not force rich surround through stereo reconstruction.
- Do not let Windows APIs define the portable core.
- Do not uninstall the incumbent before migration proves a replacement.
- Do not accept dry + wet duplication.
- Do not accept OFF with queued wet leakage.
- Do not adopt complexity merely because libaural can research it.
- Do not rewrite useful inherited renderer machinery for aesthetics.
- Do not forget parked research.

Use:

```text
actual weakness
→ smallest tested mechanism
→ measure
→ listen
→ keep only if earned
```

---

# 31. Documentation precedence

This README owns:

- product identity;
- perceptual north star;
- platform/core boundary;
- stereo-primary use case;
- concurrent-stream law;
- source-truth hierarchy;
- migration law;
- bypass/single-path law;
- Helix/libaural boundary;
- current product frontier.

Supporting docs own narrower contracts.

Current docs include:

- `docs/windows-audio-route.md`;
- `docs/windows-integration-research.md`;
- `docs/influence-ledger.md`;
- `docs/headphone-rendering-research.md`;
- `docs/scene-renderer-contract.md`;
- `docs/realtime-control-contract.md`;
- `docs/music-presentation-contract.md`;
- `docs/headphone-calibration.md`;
- `docs/contraction-ledger.md`;
- `omniphony-renderer/assets/binaural-baselines/README.md`;
- `CONTRIBUTING.md`.

If a supporting doc conflicts with this README's product priority, this README wins until explicitly revised.

Do not create a second master-plan document beside it.

---

# 32. Re-entry checkpoint

If context is lost, recover this hierarchy:

```text
1. preserve the already-good upstream Omniphony percept
2. make headphones disappear into a coherent full sphere
3. ordinary stereo music is the main use case
4. the song should feel pre-authored for the presentation, not dynamically remixed
5. richer surround/object truth should be preserved whenever it exists
6. source layout belongs to each concurrent stream, never to a global mode
7. physical headphone output remains ordinary binaural stereo
8. Omniphony core stays platform-agnostic; Windows is the first host
9. keep the incumbent installed during migration and disable pieces before removing them
10. one physical path only; bypass must be clean
11. current first live listen proved transport, not clean sound quality
12. improve only where listening/testing finds an actual weakness
13. use Helix and libaural as research sources, then compress useful results
14. never trade the musical object for the spatial effect
```

That is the current view of Omniphony for Headphones.
