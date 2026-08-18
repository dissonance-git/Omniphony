# Realtime game-music source contract

## Purpose

Omniphony accepts already-separated causal game-music sources plus source evidence from Retro VGM Compiler and related frontends.

The contract is a realtime DSP boundary:

```text
causal source PCM
+ source identity / route / timing evidence
+ ordered intra-block evidence events
+ past-derived scene mix budget
        ↓
Omniphony source presentation policy
        ↓
canonical 8.1.4.4 semantic world + dynamic source objects
        ↓
22-direction System-H-derived shell
        ↓
cascaded binaural HRTF / ITD renderer
        ↓
headphones
```

It is not a prerendered soundtrack automation format and it does not create a second scene model.

The audible target is an **immersive remix from recovered real sources**. It may sound as though the soundtrack had always been mixed for a larger modern format. That is an explicit presentation choice, not a historical-authorship claim.

## Product law

The source-aware path has two simultaneous obligations:

```text
SOURCE TRUTH
preserve what the game / driver / chip / DSP actually did

PRESENTATION FREEDOM
use modern spatial dimensions where the source never had a way to author them
```

The first obligation belongs primarily to Retro VGM Compiler. The second belongs primarily to Omniphony.

The shorthand is:

> **The compiler reconstructs the musical reality. Omniphony mixes that reality into the larger world.**

## Canonical destination

Omniphony's foundational semantic vocabulary remains the 17-lane 8.1.4.4 scene:

```text
L R C LFE Ls Rs Lb Rb Cb
Tfl Tfr Tbl Tbr
Bfl Bfr Bbl Bbr
```

The current 22-direction System-H-derived shell is a render lattice above that vocabulary.

Neither structure dictates the number of PCM sources supplied by a game-music frontend.

Examples:

```text
YM2612       six complete FM channels
YM2151       eight complete FM channels
Genesis PSG  three tone voices + noise
SNES S-DSP   eight dry voices + one linked shared stereo wet field where proven
```

Therefore:

```text
source-object count
!= canonical scene-lane count
!= shell-direction count
```

Do not convert recovered chip voices into seventeen fake speaker channels.

## Provenance: AUTHORED / DERIVED / EMPTY

Spatial information has authority as well as value.

```text
AUTHORED
preserved from the source / driver / device / format

DERIVED
chosen or inferred by modern musical / acoustic / perceptual presentation policy

EMPTY
no authored source fact exists for that dimension
```

`EMPTY` is a provenance state. In `FullSphere`, an historically empty rear, elevation, distance or extent dimension may receive a `DERIVED` value.

Never relabel that modern decision as historical authorship.

Examples:

- YM2612 and YM2151 native L/R enables are authored route evidence.
- Stock Genesis PSG source identity is real, but it is not authored azimuth.
- Game Gear PSG L/R routing is authored.
- S-DSP echo send is authored send state, not authored world position.
- A genuinely supplied 3-D coordinate may be authored position.
- Foundation, foreground, diffuse, width and vertical affinity are derived presentation evidence.
- The adaptive scene mix budget is renderer intervention state, not source metadata.

## One physical source renderer, two policies

`NativeRouting` and `FullSphere` intentionally share the same physical renderer topology:

```text
recovered source objects
        ↓
SAME size-capable 22-direction shell
        ↓
SAME cascaded binaural engine
```

The difference is presentation policy, not a hidden renderer swap.

### NativeRouting

```text
sphere strength = 0
creative rear = closed
creative elevation = closed
creative extra depth = closed
creative source extent = [0,0,0]
shared-wet modern expansion = closed
native route / source identity = preserved
```

The source path still traverses the same shell and cascade so a runtime A/B isolates presentation policy.

The protected historical/reference stereo remains the truly untouched control below both source-aware modes.

### FullSphere

```text
same real recovered source objects
+ same authored route / identity constraints
+ stable DERIVED azimuth / depth / elevation / distance / extent
→ larger immersive mix
```

Stable source or persistent-part identity seeds repeatable placement. Musical evidence shapes that layout:

- authored left/right routing constrains side;
- foundation resists displacement and excessive extent;
- foreground resists excessive rear/depth movement;
- support/diffuse evidence may spend more extent and rear space;
- vertical affinity may strengthen elevation;
- shared wet remains broad and environmental;
- scene-wide budget limits how much of those freedoms are safe in the current arrangement.

The result should feel mixed, not randomly scattered.

## Source extent is audible

`SourcePresentation.size` is a real 3-D production dimension:

```text
[width, depth, height]
```

In FullSphere:

```text
source centre + size
→ SpatialChannelEvent
→ size-aware VBAP
→ 22-direction shell
→ cascaded binaural HRTF / ITD
→ headphones
```

The source topology precomputes at least five size states:

```text
0.00  0.25  0.50  0.75  1.00
```

and interpolates between them.

The shell spread path is constant-power by construction: increasing extent redistributes source energy over more shell directions rather than using source size as a volume control.

A wider object may still change binaural spectrum or perceived loudness because different HRTFs participate. That is a separate empirical listening/measurement problem and must not be confused with shell-energy normalization.

NativeRouting explicitly zeroes added extent even when a source carries `width` or `diffuse` presentation evidence. The current ABI has authored position authority but no authored extent authority, so creative extent remains closed with the sphere.

## Dry sources, historical wet, and modern room are different things

The source contract distinguishes:

```text
dry / localizable source
shared historical effect return
protected reference mix
```

A dry source can become a dynamic object when isolated causal audio is genuinely available.

A shared return stays shared. Do not manufacture one wet stem per dry voice.

The protected reference mix is not accepted as another object lane.

## SPC / S-DSP echo is its own spatial layer

SNES is the clearest example of why the shared-return distinction matters.

Where the source capture proves it, preserve:

```text
8 dry S-DSP voices
+ signed per-voice L/R route
+ per-voice echo-send state
+ final post-EVOL shared echo L/R
```

The final echo is **one historical stereo feedback field represented by linked L/R components**.

It is not:

```text
voice 1 wet stem
voice 2 wet stem
...
voice 8 wet stem
```

Omniphony may independently control the historical field's:

```text
rear bias
height
radial depth
presentation strength
3-D shell extent
```

without moving or rewriting the eight dry voices.

The field centre and field extent are independent. Opening shared-wet extent may occupy more of the 22-direction shell while preserving the same centre target.

Also keep this distinction explicit:

```text
historical S-DSP echo
!= Omniphony externalization room
```

An echo-rich SPC can use its own S-DSP field for much of the envelopment and therefore request less generic Omniphony room support.

## Source-object boundaries

For ordinary Yamaha FM synthesis, the default spatial object is a **complete audible channel**, not an FM operator.

```text
FM operator
!= independent spatial object
```

Algorithms, modulation and feedback make operators internals of one synthesis network unless a future representation proves independent authored objects.

Likewise:

```text
better whole-chip renderer
!= proven independent enhanced stems
```

Shared DAC/mixer paths, clamps, feedback or other coupling require explicit decomposition/additivity evidence before independent enhanced lanes are called exact.

## Soundtrack-adaptive spatial budget

"Works on every soundtrack" does not mean every soundtrack converges toward one geometry.

The invariant is the quality law:

> **Same aesthetic target, different spatial expenditure.**

The governor learns from completed past audio only. Current measurements include:

```text
active-source density
energy concentration / distribution
low-band energy share
edge / transient density
historical shared-effect energy share
coarse dry-source spectral overlap
```

No genre, game, composer, soundtrack name or cue label is required.

### Coarse spectral overlap

The compiler now tracks a causal three-band profile for each source using two one-pole splits plus a time-based persistent power envelope.

The scene statistic compares **active dry sources only** using energy-weighted pairwise profile overlap.

```text
0 → little broad spectral overlap
1 → broadly similar three-band energy distributions
```

This is deliberately called `coarse_spectral_overlap`, not masking.

It is not a psychoacoustic masking probability and does not claim critical-band precision.

Its current first-order use is conservative:

```text
more spectral crowding
→ tighter dry-object extent
→ less dry diffuseness
→ slightly smaller shared wet field
→ less added Omniphony room
```

It does **not** make individual objects wider in the name of "separation". True masking-aware source-to-source repulsion/panning requires its own explicit control rather than an abuse of extent, depth or height fields.

### Causality and smoothing

The spatial budget for block N is derived only from completed audio before block N.

```text
completed block N-1
→ observer update
→ smoothed budget
→ render block N
```

Current budget smoothing is asymmetric:

```text
contraction ≈ 0.30 s
expansion   ≈ 1.50 s
```

Dense, wet, transient-heavy or crowded material can reclaim clarity quickly. Newly open material expands more slowly so the image does not pump with callbacks or tiny arrangement gaps.

The coarse spectral profiles themselves are time-based and sample-driven, so callback partitioning should not become a hidden spectral feature.

## Practical adaptive examples

```text
sparse / dry FM or PSG
→ larger individual objects
→ more available depth / height
→ more optional room support

dense layered cue
→ tighter object extent
→ preserve articulation
→ avoid turning every source into haze

spectrally crowded dry sources
→ tighten width / diffuse treatment
→ suppress extra room
→ preserve future room for an explicit separation mechanism

echo-heavy SPC
→ let historical S-DSP field carry envelopment
→ preserve dry-voice definition
→ reduce generic added room

bass / transient heavy
→ protect foundation and attack
→ spend more scale on accompaniment and environment
```

## ABI 0.4

The source DLL boundary is ABI 0.4.

The evidence/event records from 0.3 remain intact. ABI 0.4 adds the scene-control record:

```c
OmniphonySourceMixBudgetV1
```

with:

```text
depth_scale
height_scale
shared_wet_strength_scale
shared_wet_extent_scale
externalization_scale
```

and the setter:

```c
omniphony_source_set_mix_budget(...)
```

Source evidence and scene intervention stay separate.

The timed source call continues to accept ordered events of:

```text
frame_offset
lane_index
new evidence state
```

A source evidence change at frame 137 remains a change at frame 137. Derived presentation may ramp perceptually afterward, but authored timing is not quantized for convenience.

The adaptive client requires ABI minor 0.4. An older 0.3 DLL is rejected rather than silently dropping the new scene-control layer.

## Runtime mode switching

ABI 0.4 may switch `NativeRouting` and `FullSphere` without recreating the source processor.

That is valid because both modes now share one extent-capable shell/cascade topology.

```text
same processor
same shell
same binaural engine
policy A → policy B
```

A runtime regression must prove:

- NativeRouting and FullSphere produce different presentation when the same neutral source is rendered;
- switching back to NativeRouting after reset recovers deterministic control output;
- no renderer reconstruction is required merely to change source spatial policy.

## Identity continuity

Physical hardware slot is not presentation identity.

The source renderer prefers:

```text
persistent musical part
otherwise bounded source identity
```

for spatial continuity.

If an unrelated source reuses a chip channel, it must not inherit the outgoing source's pose ramp.

If a persistent musical part genuinely migrates across hardware slots, smooth continuity may survive.

Presentation identity is committed only after successful rendering.

## Reset / seek

Track change, seek and decoder restart are causal timeline boundaries.

Reset clears:

```text
renderer / binaural history
source-presentation identity history
compiler acoustic observer state
compiler role memory
coarse spectral-profile state
adaptive scene budget → neutral
```

A new track must not inherit the previous track's adaptive mix character.

## Ownership

```text
Retro VGM Compiler
  source truth
  source-quality admission
  exact timing
  source / part identity
  native route and send evidence
  completed-scene observation
  coarse spectral-overlap observation
  causal scene intervention budget
        ↓
Omniphony
  NativeRouting / FullSphere presentation policy
  canonical 8.1.4.4 semantic world
  source centre and extent presentation
  22-direction shell
  cascaded binaural HRTF / ITD
  distance / air / optional externalization
```

The compiler must not pre-render a competing spatial world.

Omniphony must not decide which emulator or source reconstruction is more truthful.

## Research grounding

The architecture borrows obligations rather than numeric presets from prior work:

- object-scene research supports separating source description from perceptually motivated immersive presentation;
- source-width research treats apparent extent as an independent music-production dimension;
- direct/reverberant work supports keeping localizable dry energy distinct from shared diffuse fields;
- masking-aware automatic mixing research supports using spectral decomposition and overlap/masking evidence to protect clarity rather than applying one spatial preset to all material;
- covariance/decorrelation work remains a useful future comparison against the current shell-spread implementation for extended binaural sources.

Relevant anchors include:

- Jot, Carpentier & Warusfel, 2023, DOI `10.1109/I3DA57090.2023.10289196`
- Landschoot & Jot, 2023, DOI `10.1121/10.0018389`
- McCormack, Politis & Pulkki, 2021, DOI `10.1109/WASPAA52581.2021.9632724`
- Anemüller, Thiergart & Habets, 2024, DOI `10.1109/ICASSP48485.2024.10448024`
- Hafezi & Reiss, 2015, masking reduction for multitrack mixing
- Tom, Reiss & Depalle, 2019, automatic multitrack spatialization based on unmasking and panning practice

These works do not validate Omniphony's current numeric constants. Physical listening and corpus testing remain required.

## Validation obligations

Keep engineering, provenance and perception evidence separate.

Minimum regressions:

```text
AUTHORITY
source-native route / timing / identity survive
DERIVED geometry never becomes authored

MODES
NativeRouting closes creative rear / height / depth / extent
FullSphere opens deterministic immersive geometry
both modes use one 22-direction/cascade topology
runtime mode switch round-trips without processor recreation

EXTENT
source extent changes FullSphere headphone audio
source centre does not move merely because extent changes
shell spread remains approximately constant power

SPC WET
post-EVOL L/R remain linked components of one shared field
shared-wet extent changes headphone field without moving centre
historical wet never becomes N fabricated wet stems
historical wet remains distinct from Omniphony room

ADAPTATION
block N cannot choose its own budget
callback partitioning does not materially change persistent spectral profile
spectrally similar dry sources yield higher coarse overlap than disjoint sources
shared wet is excluded from dry-source overlap
higher crowding cannot make dry extent/diffuseness or added room larger
render failure does not advance observer / role / budget memory
reset returns budget and analysis state to neutral

SOURCE OBJECTS
FM operators are not spatial objects by default
whole-chip fidelity does not imply additive stems

REFERENCE
protected historical/reference playback remains available
```

A mechanically correct path is not automatically a great-sounding mix.

Final promotion remains perceptual:

> **Different soundtracks should remain recognizably different while each gains as much stable width, depth, height, source body and envelopment as its own arrangement can support without sacrificing impact, clarity, timbre, transients or musical hierarchy.**