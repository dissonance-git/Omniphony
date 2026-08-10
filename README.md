# Omniphony for Headphones

> **Private master project plan and canonical re-entry surface.**
>
> This repository is being built for one listener first. This README is the engineering memory, product contract, roadmap, listening target and context-recovery document. If chat context disappears or research starts pulling the project sideways, recover from this file and recent `main` history before inventing a new direction.

Omniphony for Headphones is a fork of [`mgth/Omniphony`](https://github.com/mgth/Omniphony) being turned into a **Windows-first universal headphone spatial processor**, with ordinary music as the primary use case and real surround/object audio as first-class richer inputs.

The foundation is unusually strong:

> **Upstream Omniphony already sounds good. Preserve that perceptual floor, make it practical for normal Windows listening, and improve it only when the improvement earns itself.**

The upstream hosted headphone demo already produces a convincing external acoustic volume rather than a flat lateral pan. The project exists because that missing foundation is now present.

The target is broader than a HeSuVi clone and broader than a conventional upmixer.

A useful shorthand is:

> **a modern open-source Sony-360-like headphone world that works intelligently with ordinary stereo, surround, games, movies and richer spatial sources instead of requiring specially authored music.**

Another useful comparison is:

> **an open-source alternative to HeSuVi / commercial headphone spatializers, but built around a real reusable spatial renderer rather than a collection of fixed virtual-speaker HRIR pipelines.**

Those comparisons communicate the product category. They do not define the architecture.

---

# 0. Perceptual north star

The product goal is not merely wider stereo.

The goal is:

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

This is intentionally **not** the image of an engineer riding faders while the song plays.

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

The processing can be stateful because realtime audio DSP is stateful. HRTF convolution, delays, smoothing, interpolation, reflections, room decay, source trajectories and head tracking all require state.

But the musical presentation should not feel like the system is changing its mind about the recording while the listener hears it.

The aspirational quality target can therefore be written as:

```text
PRE-AUTHORED IMMERSIVE QUALITY
without requiring
LIVE SEMANTIC REMIXING
```

After acclimation, ordinary headphone playback should ideally feel dimensionally collapsed by comparison.

That is an aspiration, not a literal `100×` measurement claim.

---

# 1. The hard fidelity law

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

The desired bypass reaction is:

> **“the world collapsed.”**

not:

> **“the music came back.”**

This rule outranks clever DSP, research elegance and feature count.

---

# 2. Project hierarchy

The correct hierarchy is:

```text
UPSTREAM OMNIPHONY
already-good binaural/spatial percept
        ↓
PROTECTED PERCEPTUAL FLOOR
must remain reproducible
        ↓
WINDOWS PRODUCT / TRANSPORT
make it effortless to hear every day
        ↓
CONTROLLED RENDERER IMPROVEMENTS
only where tests/listening expose a weakness
        ↓
DISTILLED RESEARCH MECHANISMS
Helix / libaural only when they earn their way in
```

Do **not** reverse this into:

```text
Helix research
→ giant libaural runtime
→ elaborate music reasoning
→ speculative scene stack
→ someday hope Omniphony still sounds good
```

Omniphony is a **processor/product**, not the runtime manifestation of the entire research lab.

The research may be vast. The product should feel simple.

---

# 3. Product promise and zero-config law

The mature user experience should be almost boring:

```text
install
→ Omniphony ON
→ play anything normally
```

The project should ultimately require almost no configuration from an ordinary user.

A plausible final shell is:

```text
Omniphony        ON

Output           automatic
Headphones       automatic / saved
Mode             automatic
```

Advanced users may later get optional controls for things such as:

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

### Device-selection law

Routine playback should **not** ask the listener to choose an audio device every launch.

For the private development baseline, the existing stable Windows setup is known. The host should discover/reuse it automatically and expose manual selection only as a diagnostic escape hatch.

For the finished product, device selection belongs to one-time setup, automatic endpoint tracking, or a settings page, not to every playback session.

---

# 4. Universal input law

The amount of inference should depend on how much authoritative spatial truth the source already provides.

```text
STEREO
little explicit full-sphere truth
→ preserve the mastered image
→ add only validated presentation structure

5.1 / 7.1
real directional channel truth already exists
→ preserve it
→ render it beautifully

5.1.2 / 7.1.4 / height beds
real elevation information exists
→ preserve height
→ render the authored sphere

OBJECT AUDIO
actual object positions exist
→ do not infer what is already known
→ render objects directly

ALREADY-BINAURAL / VIRTUALIZED AUDIO
headphone spatial cues already exist
→ avoid destructive double virtualization
```

The intelligent behavior is often **doing less** when the source is richer.

Omniphony should not force every source through a fake 7.1 reconstruction merely because traditional virtual-surround systems are speaker-channel-shaped.

---

# 5. Source-truth hierarchy

When richer trustworthy information exists, preserve it.

Conceptual order:

```text
authored object metadata / rich spatial scene
        ↓
native Ambisonics / HOA
        ↓
discrete authored stems / layers
        ↓
sequencer / synthesis structure
        ↓
height / surround beds
        ↓
stereo
        ↓
mono
```

Controlled research may deliberately collapse rich scenes and test recovery. That is an experiment, not the normal playback law.

---

# 6. Renderer vocabulary without scene-model tyranny

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

# 7. The two references Omniphony must preserve or beat

## 7.1 Upstream Omniphony = perceptual ancestor

The hosted upstream headphone demo is the primary oracle for the renderer's starting spatial character.

Local protected approximation:

```text
omniphony-renderer/assets/binaural-baselines/upstream-demo-reference.yaml
```

Published ingredients approximated by that control:

```text
stock-style Omniphony defaults
+ SAF/KEMAR HRTF
+ early reflections enabled
+ late reverb disabled
```

The hosted site does not pin the exact render commit/command, so the local control is not claimed to be byte-identical. Its role is perceptual ancestry.

A richer fork configuration that sounds worse at matched loudness loses.

## 7.2 Existing HeSuVi chain = end-to-end incumbent

The real daily listening system remains the second reference.

Omniphony does not graduate because it beats dry stereo. It should eventually make the actual tuned incumbent feel unnecessary.

Keep separate:

```text
Did the Omniphony renderer improve?

and

Would this finished product replace the current listening system?
```

---

# 8. Current incumbent snapshot

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

ASIO is useful incumbent/specialist plumbing. It is not the required ordinary-product route.

## HeSuVi / Equalizer APO

Current HRIR reference:

```text
DTS Virtual:X for speakers
Original-unmodified file, DTS Inc.
version shown: 2025.3.16.0
```

Observed HeSuVi matrix state:

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

# 9. What the incumbent teaches us

Do not cargo-cult literal stages such as:

```text
Vocal Exciter
Reverb
5.1 upmix
LFE = 200
DTS virtualization
Hi-Fi Cable
ASIO Bridge
```

Ask instead:

> **What audible function was each stage buying, and can Omniphony provide that function more directly and coherently?**

Current preference evidence says:

- large external acoustic volume is desirable;
- stable behind-head presentation is desirable;
- height/below should ultimately feel like real dimensions, not gimmicks;
- bass physicality and timing matter strongly;
- center authority matters;
- music must keep punch, identity and clarity;
- ordinary stereo music is the primary use case;
- real surround should be preserved and enhanced rather than flattened;
- set-and-forget use matters;
- complicated internals are acceptable;
- complicated listener rituals are not.

---

# 10. HeSuVi relationship

HeSuVi remains an important influence/reference because it demonstrates both capability and UX limits.

The uploaded/reference HeSuVi material contains:

- large HRIR collections;
- commercial-style virtualizer references;
- channel matrices;
- position/level manipulation;
- headphone-EQ profiles;
- Equalizer APO routing/configuration patterns;
- stereo/surround handling conventions.

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

HeSuVi is therefore both:

1. a practical incumbent to beat, and
2. a museum of virtual-surround design choices worth studying.

Do not redistribute proprietary HRIR material without appropriate rights.

---

# 11. Helix and libaural relationship

Helix is the research laboratory.

libaural is the separate reusable artificial-hearing project.

Their role is to improve the quality of mechanisms available to Omniphony, not to own its runtime architecture.

Correct relationship:

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

The preferred direction is:

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

A rich learned or biological model may remain a teacher.

A cheap DSP rule that preserves the useful auditory behavior is often the better product implementation.

---

# 12. Helix music research: what transfers and what does not

The current Helix music work provides valuable concepts such as:

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

These are extremely useful for:

- asking better questions;
- designing exact-moment listening tests;
- finding failure modes;
- teaching libaural which auditory relations may matter;
- building negative controls;
- discovering invariants that spatial processing must not destroy.

They are **not automatically runtime modules**.

For Omniphony, the most direct transferable laws are simpler:

```text
protect musical identity
protect center authority
protect groove / transient timing
protect bass function
protect pressure / weight where it is part of the recording
keep direct / broad / diffuse / room distinct
do not spread everything merely because the renderer can
make spatial processing feel authored rather than algorithmically busy
```

Personal music research is an unusually precise listening oracle. It is not a universal taste profile to bake into defaults.

---

# 13. Music presentation law

The default product should **enhance music listening dramatically without sounding like it remixed the song**.

A strong conceptual model is:

```text
MASTERED RECORDING
        ↓
preserve its identity / timing / hierarchy
        ↓
give its existing relational world convincing physical geometry
        ↓
OMNIPHONY BINAURAL RENDERER
        ↓
INHABITABLE HEADPHONE SPHERE
```

But this description is an evaluation philosophy, not a command to run dynamic musicological inference.

The goal is for the song to feel like it **was already mixed this way**.

No audible scene “rethinking.”
No gratuitous source teleportation.
No chorus detector moving things because a chorus began.
No algorithm showing off.

Advanced adaptive behavior is allowed only if it is demonstrably more transparent and stable than a simpler mechanism.

See `docs/music-presentation-contract.md`.

---

# 14. Windows is the product platform now

Correct current rule:

> **Build the Windows product first.**

Keep OS/device code above the portable renderer where practical. Do not divert current effort into macOS/Linux/mobile products.

Portability is preserved through sane boundaries, not through an active multi-platform roadmap.

---

# 15. Final Windows single-path law

Ordinary playback must reach the listener **once**, through Omniphony.

```text
source app
→ Omniphony processing
→ physical headphones
```

Never:

```text
source app ─────────────→ dry physical headphones
      └→ Omniphony ─────→ wet physical headphones
```

The final production route may eventually be:

### Endpoint/system-effect APO

```text
application
→ Windows shared audio engine
→ Omniphony APO
→ physical endpoint
```

or:

### Virtual render endpoint

```text
application
→ Omniphony virtual endpoint
→ Omniphony host
→ renderer
→ physical endpoint
```

ASIO remains a specialist/reference route, not the universal consumer solution.

Microsoft Spatial Sound APIs are useful future input semantics, but reviewed public documentation has not established a simple generic path for registering Omniphony beside Sonic/Atmos/DTS in the Windows Spatial Sound dropdown. That remains a dream integration route, not a prerequisite.

---

# 16. Current temporary Windows development route

The current machine already routes ordinary Windows playback to a stable VB-Audio Hi-Fi Cable render endpoint.

The latest test exposed an important detail:

```text
Windows sees:
Dan Clark Noire X (VB-Audio Hi-Fi Cable)   [render/output endpoint]
Speakers (FiiO Q series)                   [physical render/output endpoint]

Windows capture/input endpoints:
none exposed
```

Therefore the temporary baseline must **not** depend on a microphone/recording endpoint.

The chosen development shortcut is:

```text
foobar / Windows apps
→ existing Hi-Fi Cable render endpoint
        │
        └→ self-excluding WASAPI process-loopback capture
             ↓
           Omniphony
             ↓
        physical FiiO output
             ↓
          headphones
```

The repository already contains a `wasapi-rs` self-excluding process-loopback activation probe. The next live host should promote that primitive into continuous PCM capture.

Why this is useful:

- no Windows recording device is required;
- no device-selection prompt should be required for routine launch;
- Omniphony can capture ordinary app playback;
- Omniphony's own FiiO output is excluded from capture, avoiding immediate feedback;
- the dry signal remains on the virtual cable rather than the physical FiiO while the old ASIO forwarding path is closed;
- it is good enough to judge arbitrary music before building the final endpoint/APO solution.

Important limitation:

> **Loopback is still a development scaffold, not the final transparent product route.**

It proves live arbitrary-audio rendering quickly. The final product still needs owned single-path routing that does not depend on an externally installed cable.

---

# 17. Current native Windows implementation

Workspace:

```text
omniphony-renderer/windows_host/
```

Current pieces include:

### `windows_host.exe`

- CPAL/WASAPI native output;
- output smoke test;
- protected reference-demo playback;
- packaged reference render validation;
- WASAPI loopback activation probe.

### `omniphony_live.exe`

A disposable live-listening baseline currently built around CPAL capture/output plus the protected Omniphony engine.

Its first capture design assumed a Windows recording endpoint. Real-machine testing proved that assumption wrong for the current Hi-Fi Cable setup.

That is a transport bug/assumption, not a renderer failure.

The next implementation should consume self-excluding WASAPI loopback instead of asking the user to select a recording device.

### `realtime_ffi`

A tiny interleaved-f32 PCM ABI for native-host boundaries. Its first implementation is deliberately bit-exact identity and remains useful for isolating transport correctness from renderer behavior.

### `reference_bridge`

A deterministic bridge into the protected Omniphony engine. Canonical channel order:

```text
L R C LFE Ls Rs Lb Rb Tfl Tfr Tbl Tbr
```

It already supports streaming-style PCM framing used by the current prototypes.

---

# 18. P0 result: HEARD

P0's job was only to prove:

```text
native Windows output
+
protected Omniphony renderer
+
known reference content
+
packaged executable
```

That has now been physically tested on the real Windows machine.

Observed result:

- `windows_host.exe --reference-demo` plays successfully;
- the protected Omniphony path reaches the real headphones;
- it sounds spatially larger than the flat test;
- the bundled fixture is only a very short synthetic reference, so it is insufficient for a serious judgment of music quality.

Therefore:

```text
P0 transport/render viability
= PASSED / HEARD

P0 perceptual product-quality judgment
= intentionally not answered by the tiny fixture
```

Do not regress to repeatedly proving the same two-second demo. The next value comes from arbitrary real audio.

---

# 19. Current live-audio frontier

The immediate target is deliberately crude:

```text
ordinary foobar / Windows audio
→ temporary cable / loopback scaffold
→ protected Omniphony renderer
→ physical FiiO output
→ headphones
```

This is **not** the final product architecture.

It exists to answer the important listening question now:

> **How good is the existing Omniphony foundation on arbitrary real music and multichannel material when it replaces the HeSuVi virtualization stage?**

For the first useful music baseline, keeping the existing foobar 5.1/side upmix is acceptable because it isolates the renderer comparison:

```text
foobar
→ existing normal music DSP
→ existing 5.1/side upmix
→ clean temporary transport
→ Omniphony binaural renderer
→ headphones
```

That avoids forcing the unfinished future stereo-presentation layer to prove itself at the same time as the renderer.

Once the renderer is heard on real content, stereo presentation can be developed independently.

---

# 20. Surround and rich-input opportunity

Real surround is not merely compatibility baggage. It is one of the strongest cases for Omniphony.

When a game, film or multichannel music source already provides 5.1/7.1/height information:

```text
source supplies spatial truth
→ Omniphony preserves it
→ renderer enhances externalization / depth / height / continuity
→ headphones
```

The processor should not flatten a real 7.1.4 scene to stereo and then try to rediscover the scene.

Long-term, this is how Omniphony can become more than a stereo enhancer:

> **one headphone spatial layer for ordinary stereo, real surround, height beds and true objects.**

---

# 21. Existing renderer foundation

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

# 22. Protected binaural controls

Directory:

```text
omniphony-renderer/assets/binaural-baselines/
```

### `upstream-demo-reference.yaml`

Perceptual ancestor. Minimal stock-style approximation of the hosted upstream demo.

### `baseline-room.yaml`

Fork room-assisted comparison. More DSP does not make it superior by definition.

### `dry-binaural.yaml`

Fork HRTF/scale/air policy with room effects disabled, useful for isolating room contribution.

Experimental algorithms get explicit configs/flags. Never overwrite the protected control to make a candidate look better.

---

# 23. Realtime law

> **Host callback size is an implementation detail, not a coordinate system for the auditory world.**

Gain, movement, HRTF transitions, room changes and other intended continuous state belong in sample/time coordinates.

The same semantic engine should behave consistently across:

```text
WASAPI
ASIO
file rendering
future APO / endpoint host
```

Heavy work stays off realtime callbacks.

---

# 24. Validation lanes

Keep failures attributable.

## Lane A · Compiler / deterministic DSP

```text
compile
→ unit tests
→ deterministic fixtures
→ fidelity/null checks
→ callback invariance where required
```

## Lane B · Known scene → binaural

```text
known geometry
→ HRTF / ITD
→ extent / room
→ binaural output
```

## Lane C · Stereo evidence / presentation

```text
controlled stereo
→ evidence
→ bounded presentation state
→ protected renderer
```

## Lane D · Windows transport

```text
same PCM / engine
→ host route A
→ host route B
→ compare timing / glitches / latency
```

## Lane E · Renderer perceptual reference

```text
upstream Omniphony reference
↔ fork candidate
```

## Lane F · Real incumbent A/B

```text
CURRENT
foobar + VB-Audio + HeSuVi + ASIO

VERSUS

TARGET
Omniphony + physical FiiO + Noire X
```

## Lane G · Exact music moments

Helix-derived evaluation can preserve exact moments where a musical relation matters:

```text
exact excerpt
+ why it matters
+ near-miss / negative control
+ Omniphony ON/OFF
→ what spatially improved?
→ what musically broke?
```

The music itself need not live in the public repo.

---

# 25. Listening scorecard

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

---

# 26. Research parking rule

Useful external findings must be written into the repo even when they are not promoted into current code.

Promotion ladder:

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

# 27. Influences and reference families

Useful families already mined include:

- upstream Omniphony;
- HeSuVi;
- Steam Audio;
- Resonance Audio;
- Meta XR Audio;
- Cavern;
- CamillaDSP;
- `wasapi-rs`;
- HEnquist realtime/device patterns;
- Microsoft Core Audio / Spatial Sound;
- Dolby public architecture/examples;
- HRTF/BRIR/SOFA research;
- Ambisonics tooling;
- psychoacoustics and auditory-scene analysis;
- MIR/source separation;
- the older `spatial-dsp` lineage.

The older `spatial-dsp` project remains a mechanism mine, not the target topology.

Useful harvested concepts include hard-pan-safe directness, complex M/S evidence, persistence, center preservation, direct/diffuse distinction, bass anchoring, source spread and Windows CI lessons.

---

# 28. Upstream relationship

`mgth/Omniphony` remains the technical ancestor, perceptual foundation and source of useful mechanisms/fixes. It does not define this fork's product roadmap.

Use:

```text
inspect exact upstream change
→ identify mechanism
→ check whether already present
→ ask whether it serves this product
→ import smallest useful missing part
→ validate
```

Do not merge broad upstream work merely to keep histories visually similar.

When this fork proves a general fix, isolate the portable/general part before considering upstream contribution.

---

# 29. Current milestone ladder

## W0 · Protect/reproduce upstream sound — ESTABLISHED

Protected configs and deterministic known-scene paths exist.

## P0 · First native protected listen — HEARD

Native Windows output plus protected Omniphony rendering has been physically heard on the real machine.

## P0.1 · Arbitrary live audio through Omniphony — CURRENT

Immediate task:

```text
Windows / foobar audio
→ self-excluding WASAPI loopback
→ Omniphony
→ auto physical output
```

No microphone/capture endpoint requirement.
No per-launch device-selection ritual.
No polished UI.
No installer.

Just enough live routing to judge real audio.

## P1 · Easy everyday Windows listening

- persistent reliable realtime path;
- automatic endpoint behavior;
- fast incumbent ↔ Omniphony A/B;
- transport hardening based on actual failures;
- ordinary music listening without developer rituals.

## P2 · True system-wide single-path ON/OFF

Choose/prototype supported APO or virtual-endpoint route from measured reliability, latency, installability and user experience.

## P3 · Native surround / rich spatial input

Preserve 5.1/7.1/height beds and object semantics directly.

## P4 · Better automatic stereo presentation

Improve stereo → full-sphere presentation without making the recording sound remixed.

## P5 · Calibration / personalization

- headphone profiles;
- optional headphone correction;
- HRTF selection/import;
- listener personalization;
- head tracking where useful;
- deeper libaural-derived mechanisms only when earned.

---

# 30. Product anti-goals

- Do not replace good sound with research.
- Do not make the user configure routing every launch.
- Do not build a settings forest.
- Do not turn music into an AI remix.
- Do not make placements wander because a classifier changed its mind.
- Do not hallucinate object truth from stereo.
- Do not equate reverb with 3D.
- Do not make AI/model availability a playback dependency.
- Do not force rich surround through stereo reconstruction.
- Do not let infrastructure consume the product.
- Do not build cross-platform shells before Windows works.
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

# 31. Repository contraction law

Keep code when it serves at least one of:

```text
1. native Windows daily playback
2. protected renderer behavior
3. deterministic known-scene truth
4. HRTF / calibration truth
5. migration / A-B observability
6. current isolated experiment
```

Do not delete useful layouts, deterministic fixtures, protected configs, specialist ASIO support, or renderer internals merely because the final UI will not expose them.

Detailed ownership lives in `docs/contraction-ledger.md`.

---

# 32. Documentation precedence

This README owns:

- product identity;
- perceptual north star;
- zero-config law;
- incumbent context;
- source-truth hierarchy;
- migration law;
- roadmap priority;
- Helix/libaural boundary;
- current Windows frontier.

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

# 33. Working development loop

When uncertain what to do next:

```text
make native Omniphony easier to hear on real audio
        ↓
compare against protected upstream-style reference
        ↓
compare against real HeSuVi incumbent
        ↓
identify actual weakness
        ↓
research only that weakness
        ↓
implement smallest candidate
        ↓
measure + listen
        ↓
keep only what earns itself
```

This loop is deliberately hostile to research drift.

---

# 34. Re-entry checkpoint

If conversational context is lost, start with:

1. this README;
2. recent commits on `main`;
3. `docs/windows-audio-route.md`;
4. `docs/influence-ledger.md`;
5. `docs/windows-integration-research.md`;
6. `docs/music-presentation-contract.md`;
7. `docs/scene-renderer-contract.md` and `docs/realtime-control-contract.md`;
8. protected binaural baselines;
9. the real incumbent snapshot above.

Durable hierarchy:

```text
1. preserve the already-good upstream Omniphony percept
2. make the headphones perceptually disappear into a coherent full sphere
3. make music feel pre-authored for that presentation, not dynamically remixed
4. preserve richer surround/object truth whenever it already exists
5. keep the HeSuVi incumbent available until Omniphony clearly wins
6. finish the simplest live Windows route around the existing renderer
7. remove routine configuration from the user experience
8. improve only where listening/testing finds an actual weakness
9. use Helix and libaural as research sources, then compress useful results
10. never trade the musical object for the spatial effect
```

That is Omniphony for Headphones.