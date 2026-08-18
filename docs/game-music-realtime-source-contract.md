# Realtime game-music source contract

## Purpose

Omniphony's causal source path accepts already-separated source audio plus source/musical evidence from systems such as Retro VGM Compiler. It is a streaming DSP boundary, not a prerendered soundtrack automation interface.

```text
causal source lanes
+ current source evidence
+ ordered intra-block evidence events
+ past-derived scene mix budget
        ↓
Omniphony source presentation policy
        ↓
canonical 8.1.4.4 semantic world + dynamic source objects
        ↓
22-direction render shell
        ↓
spatial renderer
        ↓
binaural output
```

The source path supplies a richer input to the existing Omniphony product architecture. It does not create a second scene model.

The audible source-aware Surround target is an **immersive remix from recovered real sources**. It is intentionally allowed to sound as though the soundtrack had always been mixed for a larger modern format. This is a presentation objective, not a claim about what the historical hardware or composer literally authored.

## Universal listening target

"Works on every soundtrack" does not mean that every soundtrack should converge toward one fixed geometry.

The invariant is the **quality law**, not a universal mix template:

```text
same aesthetic target
+ soundtrack-specific causal mix geometry
→ soundtrack-specific spatial expenditure
```

Omniphony should preserve what makes a soundtrack itself while changing how much of the available immersive field is safe to use.

The current source-aware governor can learn from source-agnostic completed-scene measurements such as:

```text
active-source density
energy concentration / distribution
low-band energy share
edge / transient density
historical shared-effect energy share
```

It does not need a genre, game, composer or soundtrack preset to make the first-order decision.

Examples:

```text
sparse / dry
→ larger individual source width
→ more available depth / height
→ more optional Omniphony externalization

dense / layered
→ tighter individual sources
→ more hierarchy through depth than indiscriminate width
→ preserve articulation

echo-heavy SPC
→ let the recovered S-DSP echo carry much of the envelopment
→ reduce additional Omniphony room support
→ keep dry voices more legible

bass / transient heavy
→ protect foundation and attack
→ spend more spatial scale on accompaniment / ambience than on the anchor
```

The governor contracts added treatment faster than it expands it. That makes a sudden dense or wet passage pull the scene inward promptly while a newly sparse passage opens more slowly, avoiding audible spatial pumping.

The budget for block N is derived only from audio completed before block N. Current PCM never decides its own presentation retroactively.

## Canonical scene destination

Omniphony's foundational product vocabulary remains the 17-lane 8.1.4.4 scene:

```text
L R C LFE Ls Rs Lb Rb Cb
Tfl Tfr Tbl Tbr
Bfl Bfr Bbl Bbr
```

The current 22-direction shell remains an internal render lattice above that scene.

A game-music source frontend is **not** required to emit seventeen PCM lanes. It should preserve the source-native topology it actually knows.

Examples include:

```text
YM2612       six complete FM channels
YM2151       eight complete FM channels
Genesis PSG  three tone voices + noise
SNES S-DSP   eight dry voices + one shared stereo wet field when proven
```

These dynamic source objects enter Omniphony with evidence. Omniphony decides how unauthored presentation dimensions inhabit the canonical world.

```text
source object count
!= canonical scene lane count
!= render-shell direction count
```

The 8.1.4.4 vocabulary therefore remains stable without forcing game-music hardware channels to impersonate loudspeaker channels.

## Authority law

The richer the source truth, the less Omniphony should infer **about source truth**. This does not prohibit deliberate creative placement once source-aware Surround is explicitly enabled.

The cross-project conceptual states are:

```text
AUTHORED
preserved by source / format / driver / device

DERIVED
chosen or inferred by musical / acoustic / perceptual presentation policy

EMPTY
no authored source fact exists for that dimension
```

`EMPTY` is a provenance statement, not necessarily an audible-silence requirement. A historically empty rear/elevation coordinate may receive a `DERIVED` position in FullSphere mode while remaining empty at the authored layer.

A source-side authority enum and Omniphony's scene provenance do not need identical binary representations, but they must preserve the same semantic distinction.

Examples:

- YM2612 or YM2151 native L/R enables are authored route evidence;
- stock Genesis PSG voice identity is not authored azimuth;
- an S-DSP echo send is authored send state, not authored reverberant source position;
- foundation/foreground/diffuse/width/vertical-affinity values are derived per-source presentation evidence;
- the scene mix budget is a derived renderer intervention control, not source metadata;
- source-supplied 3-D coordinates may be authored position;
- FullSphere may assign stable width/depth/height to a real source even when those dimensions were historically unavailable.

Never relabel a stable creative or inferred placement as authored geometry.

## Presentation modes

The source renderer exposes two intentionally different listening modes:

```text
NativeRouting
→ recovered real source objects
→ preserve native laterality and identity
→ no creative rear / height / extra depth

FullSphere
→ same recovered real source objects
→ preserve authored route / position constraints
→ stable identity-aware immersive placement
→ adaptive width + depth + height + extent + distance
→ 8.1.4.4 world → 22-direction shell → binaural
```

`FullSphere` is not a confidence level. It is an explicit production choice.

The design target is similar to remixing from multitracks into an immersive format: source authenticity is protected upstream, while the listening presentation may use dimensions the original delivery medium never possessed.

Stable source or persistent-part identity may therefore seed repeatable placement even when no historical 3-D coordinate exists. Musical evidence then shapes that creative layout:

- native left/right route constrains side;
- foundation strongly resists displacement;
- foreground resists excessive rear/depth movement;
- diffuse/support evidence can enlarge rear/depth/extent;
- vertical-affinity evidence can steer height more strongly;
- shared wet fields remain broad and environmental;
- the scene mix budget limits how aggressively these freedoms are spent for the current soundtrack.

The result should feel mixed rather than randomized.

## Dry, shared-wet and reference roles

The source boundary distinguishes materially different audio roles:

```text
dry / localizable source
shared effect return
protected reference mix
```

A dry source may become a dynamic object when isolated audio is actually available.

A shared effect return remains shared. Omniphony may present it as diffuse/environmental support, but it must not clone one historical shared return into N invented per-source wet stems.

For SNES S-DSP, the recovered echo is especially useful as a separate spatial production layer. The current source model can preserve the final post-EVOL left/right components as **two linked lanes belonging to one shared stereo feedback field**. They are not two independent reverbs. Keeping them separate from the eight dry voices preserves the original echo image while giving Omniphony independent control over that field's rear bias, height, radial depth and eventual audible extent.

The shared wet field also remains distinct from Omniphony's own optional listening-room reflections:

```text
historical S-DSP echo
!= Omniphony externalization room
```

A soundtrack rich in source-native echo should generally need less added room, not more.

The protected reference mix is the scientific and audible control. It is not accepted as an object lane and must not acquire object geometry or object-memory state.

An isolated dry lane is also not automatically an exact additive stem. Coupling, feedback, nonlinear arithmetic, shared state or finite-width mixing may make useful isolated source audio non-recomposable.

## ABI 0.4

ABI 0.4 keeps the ABI 0.3 source-evidence and exact timed-event model intact and adds a **scene mix budget control plane**.

The existing whole-block call remains:

```c
omniphony_source_process_f32(...)
```

and the timed call remains:

```c
omniphony_source_process_events_f32(...)
```

Each `OmniphonySourceEvidenceEventV1` still contains:

```text
frame_offset
lane_index
new evidence state
```

Events are ordered by nondecreasing frame offset. Multiple lanes may change at one boundary. The implementation validates the complete event list before rendering the first sample, then renders exactly to each event boundary before applying the new state.

ABI 0.4 adds:

```c
OmniphonySourceMixBudgetV1
omniphony_source_set_mix_budget(...)
```

with neutral-1.0 renderer controls for:

```text
depth capacity
height capacity
shared-wet strength
shared-wet extent
added externalization level
```

These fields are deliberately **not** source evidence. They are the slowly varying intervention budget for the renderer.

The runtime order is:

```text
completed past scene
→ causal budget tracker
→ set mix budget for next block
→ source evidence + timed events
→ render
→ only after successful render, learn from raw completed block
```

If the budget setter fails, the block fails rather than rendering under stale soundtrack geometry.

A reset restores the budget to neutral along with renderer history, so a new track or seek cannot inherit the previous soundtrack's adaptive mix state.

The Rust `repr(C)` records and the Retro VGM Compiler C++ transport pin ABI 0.4. ABI minor 0.3 is intentionally rejected by the new adaptive client rather than silently omitting the scene-control layer.

## Evidence authority

The source ABI preserves these distinctions:

```text
native stereo route
!= authored 3-D position
!= per-source musical presentation evidence
!= scene-wide mix budget
```

A source may carry native left/right gains as historical routing evidence without claiming that those gains are literal world coordinates.

An authored position passes through only when the source actually supplied one.

Musical fields such as foundation, foreground, diffuse, width and vertical affinity can shape that individual source's presentation but do not become authored geometry.

The mix budget instead shapes renderer capacity and historical-wet/added-room treatment for the scene as a whole. Do not smuggle scene decisions into `confidence`, `vertical_affinity` or another unrelated source field.

The protected historical/reference mix is never accepted as an object lane.

## FM source-object boundary

For ordinary YM2612, YM2151 and related FM synthesis, the default audible source object is the **complete channel**, not an individual FM operator.

Operators participate in one synthesis network through algorithms, modulation and feedback. Treating them as independent spatial objects would confuse synthesis internals with musical-source identity unless a future source representation explicitly authored them as separate audible objects.

Likewise, a higher-fidelity whole-chip renderer is not automatically an exact source-stem renderer. Shared mixer/DAC paths, clamps or other nonlinear/coupled stages require an explicit decomposition/additivity witness before independent enhanced lanes can be called exact.

```text
better whole-chip render
!= proven independent source decomposition
```

## Identity and lane reuse

A physical source channel is not presentation identity.

The source renderer uses:

```text
persistent musical part, when present
otherwise source identity
```

for spatial-ramp continuity and stable creative placement.

If an unrelated source reuses the same channel, its first new presentation event uses a zero-length pose ramp rather than interpolating from the outgoing source's last position. If the persistent musical part remains the same across a source/slot migration, ordinary smooth presentation continuity is retained.

Presentation identity is committed only after successful rendering. A failed block therefore cannot change the continuity decision used by the next block.

This resets presentation motion only. It does not flush the entire room/binaural history merely because one source identity changed.

## Temporal stability

Derived source position and the scene mix budget are tracked states, not fresh guesses every host callback.

Stable identity plus stable evidence should produce stable presentation. FullSphere's creative base is deterministic from stable source/persistent-part identity, so historically centered sources do not jitter merely because the host callback size changes.

The mix budget uses time-based asymmetric smoothing. Contraction is faster than expansion, so a suddenly crowded/wet passage can reclaim clarity quickly while a newly sparse passage opens more gradually.

At the same time, continuity must not become glue. Authored route/position changes, source replacement or strong new evidence may legitimately move the object.

Authored timed evidence remains sample-accurate even when derived renderer motion and scene-budget changes are perceptually smoothed.

## Reset / seek lifecycle

A track change, seek or decoder restart is a true causal-timeline boundary.

`omniphony_source_reset()` therefore clears:

```text
binaural / spatial runtime history
source-presentation identity history
adaptive scene mix budget → neutral
```

The Retro VGM Compiler canonical pipeline clears its acoustic observer, musical role memory and mix-budget tracker in the same operation. Resetting only one side is invalid because it could attach old mix behavior to a fresh soundtrack.

## Retro VGM Compiler handoff

Retro VGM Compiler's canonical wrapper is:

```text
realtime_musical_omniphony_pipeline::process_block(...)
```

which enforces:

```text
raw spatial_source_block_view
→ prepare_block()
→ past-only musical role projection
→ past-only scene mix budget
→ projected_view()
→ Omniphony ABI 0.4 budget setter
→ ABI 0.4 timed source transport
→ render
→ complete_block(raw block) only after successful rendering
```

The final step intentionally uses the compiler's raw block rather than its projected renderer sidecar. That prevents a semantic feedback loop in which an earlier role guess or mix decision becomes evidence for itself.

A render failure does not advance compiler musical memory or the adaptive mix budget, so a caller may retry or fall back without state jumping ahead of the audio that actually sounded.

The compiler owns source truth, source-quality selection, causal scene observation and reference-vs-enhanced admission. Omniphony owns presentation geometry, the 8.1.4.4 semantic world, the 22-direction shell and binaural/externalization behavior.

```text
source-quality decision
→ causal source witnesses
→ past-derived scene budget
→ Omniphony presentation
```

Omniphony must not choose which emulator/source reconstruction is more truthful, and the compiler must not pre-render a second competing spatial world.

The compiler-side canonical companion is `dissonance-git/retro-vgm-compiler/docs/omniphony-realtime-spatial-path.md`.

## Research grounding for adaptation

The adaptive architecture follows a useful pattern from immersive-audio research without treating any paper as proof of one tuning.

- Object-scene work by Jot, Carpentier and Warusfel supports treating position, distance, presence and reverberance as perceptually meaningful production dimensions rather than requiring literal room reconstruction.
- Landschoot & Jot (2023, DOI `10.1121/10.0018389`) supports object-aware externalization rather than one global stereo effect.
- Ziemer (2017, DOI `10.1007/978-3-319-47292-8_10`) treats source width as a genuine music-production dimension.
- McCormack, Politis & Pulkki (2021, DOI `10.1109/WASPAA52581.2021.9632724`) gives a fidelity-conscious covariance approach to spatial source spread.
- Anemüller, Thiergart & Habets (2024, DOI `10.1109/ICASSP48485.2024.10448024`) specifically studies binaural rendering of sources with extent.

These support the obligations: preserve localization and signal fidelity, control source extent separately from position, and adapt spatial treatment to scene structure. They do not prove the numeric constants currently used by Omniphony.

## Current extent limitation

`SourcePresentation.size` is already generated and reaches object metadata, but the active **direct binaural** path currently consumes object position/gain and does not yet turn `size` into a physical binaural source-extent mechanism.

Therefore:

```text
position / height / radial depth are audible today
size metadata exists today
true direct-binaural source extent remains unfinished
```

Do not claim source extent complete until the direct binaural renderer converts that field into a controlled spread mechanism, likely through bounded multi-direction HRTF rendering or a fidelity-constrained decorrelation/covariance method.

## Artificial-hearing research

The active artificial-hearing research repository is `dissonance-git/deepSTRF`, which absorbed the durable `libaural` findings.

DeepSTRF is a teacher/ablation environment, not a mandatory runtime dependency. A rich learned or biological model earns a place in the realtime source path only after the relevant auditory obligation survives compression into a causal, bounded mechanism.

## Validation

The event ABI, identity rules and adaptive governor are engineering mechanisms. Sound-quality promotion remains separate.

Required evidence states should not be collapsed:

```text
compile/test success
reference/source correctness
realtime/chunk invariance
perceptual obligation
physical listening quality
```

Boundary regressions should additionally defend:

```text
authored route survives unchanged
creative/inferred geometry never becomes authored
NativeRouting disables creative rear/height/depth
FullSphere gives stable real sources repeatable immersive scale
scene budget comes only from completed past audio
scene budget resets between timelines
ABI 0.3 is rejected by the adaptive client
shared wet remains one source-native field
SPC wet L/R remain linked components, not independent reverbs
wet-heavy scenes can reduce added Omniphony room
stable source evidence does not jitter across block sizes
new identity does not inherit stale pose motion
FM operators are not promoted to independent objects by default
whole-chip fidelity does not imply stem additivity
reference control remains available
```

A mechanically correct adaptive path does not by itself prove that any particular tuning sounds better. The final criterion for FullSphere is perceptual: **different soundtracks should remain recognizably different while each gains as much stable width, depth, height and envelopment as its own arrangement can support without sacrificing impact, clarity or musical hierarchy.**
