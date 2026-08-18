# Realtime game-music source contract

## Purpose

Omniphony's causal source path accepts already-separated source audio plus source/musical evidence from systems such as Retro VGM Compiler. It is a streaming DSP boundary, not a prerendered soundtrack automation interface.

```text
causal source lanes
+ current source evidence
+ ordered intra-block evidence events
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
SNES S-DSP   eight dry voices + one shared wet return when proven
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
- foundation/foreground/diffuse/width/vertical-affinity values are derived presentation evidence;
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
→ width + depth + height + extent + distance
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
- shared wet fields remain broad and environmental.

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

The protected reference mix is the scientific and audible control. It is not accepted as an object lane and must not acquire object geometry or object-memory state.

An isolated dry lane is also not automatically an exact additive stem. Coupling, feedback, nonlinear arithmetic, shared state or finite-width mixing may make useful isolated source audio non-recomposable.

## ABI 0.3

`omniphony-renderer/source_ffi/include/omniphony_source.h` keeps the original whole-block `omniphony_source_process_f32()` entry point and adds:

```c
omniphony_source_process_events_f32(...)
```

The legacy call is exactly the zero-event case of the timed path.

Each `OmniphonySourceEvidenceEventV1` contains:

```text
frame_offset
lane_index
new evidence state
```

Events are ordered by nondecreasing frame offset. Multiple lanes may change at one boundary.

The implementation validates the complete event list and converts every evidence record before rendering the first sample from the call. It then executes:

```text
render [start, next_event)
apply all events at next_event
render [next_event, following_event)
...
```

This follows the normal sample-offset event model used by realtime audio systems. It does not require future-song knowledge.

The Rust `repr(C)` evidence/event records and the Retro VGM Compiler C++ transport pin the ABI 0.3 size and critical field offsets from both sides, so a future layout drift fails validation instead of silently reinterpreting evidence.

The existence of the 17-lane scene or the creative FullSphere policy does not by itself require an ABI expansion. Add fields only when genuinely new source evidence must cross the boundary and cannot be represented safely by the current contract.

## Evidence authority

The source ABI preserves these distinctions:

```text
native stereo route
!= authored 3-D position
!= inferred / creative musical presentation
```

A source may carry native left/right gains as historical routing evidence without claiming that those gains are literal world coordinates.

An authored position passes through only when the source actually supplied one.

Musical fields such as foundation, foreground, diffuse, width and vertical affinity are presentation evidence. They can influence Omniphony's policy but do not become authored geometry.

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

Derived source position is tracked state, not a fresh coordinate guess every host callback.

Stable identity plus stable evidence should produce stable presentation. FullSphere's creative base is deterministic from stable source/persistent-part identity, so historically centered sources do not jitter merely because the host callback size changes.

At the same time, continuity must not become glue. Authored route/position changes, source replacement or strong new evidence may legitimately move the object.

The policy should preserve confidence and evidence age separately from position, use bounded inertia for derived motion, and allow high-information onsets/transients to trigger a position update only when other evidence supports that update. Onset alone is not a role or coordinate.

Authored timed evidence remains sample-accurate even when the derived renderer motion that follows is perceptually smoothed.

## Reset / seek lifecycle

A track change, seek or decoder restart is a true causal-timeline boundary.

`omniphony_source_reset()` therefore clears both:

```text
binaural / spatial runtime history
source-presentation identity history
```

The Retro VGM Compiler canonical pipeline binds this reset function and clears its own musical memory in the same operation. Resetting only one side is invalid because it would leave either old musical interpretation steering a new timeline or old renderer identity/pose state attached to fresh sources.

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
→ projected_view()
→ Omniphony ABI 0.3 transport
→ this renderer
→ complete_block(raw block) only after successful rendering
```

The final step intentionally uses the compiler's raw block rather than its projected renderer sidecar. That prevents a semantic feedback loop in which an earlier role guess becomes evidence for itself.

A render failure does not advance compiler musical memory, so a caller may retry or fall back without semantic state jumping ahead of the audio that actually sounded.

The compiler owns source truth, source-quality selection and reference-vs-enhanced admission. Omniphony owns presentation geometry, the 8.1.4.4 semantic world, the 22-direction shell and binaural/externalization behavior.

```text
source-quality decision
→ causal source witnesses
→ Omniphony presentation
```

Omniphony must not choose which emulator/source reconstruction is more truthful, and the compiler must not pre-render a second competing spatial world.

The compiler-side canonical companion is `dissonance-git/retro-vgm-compiler/docs/omniphony-realtime-spatial-path.md`.

## Artificial-hearing research

The active artificial-hearing research repository is `dissonance-git/deepSTRF`, which absorbed the durable `libaural` findings.

DeepSTRF is a teacher/ablation environment, not a mandatory runtime dependency. A rich learned or biological model earns a place in the realtime source path only after the relevant auditory obligation survives compression into a causal, bounded mechanism.

## Validation

The event ABI and identity rules are engineering mechanisms. Sound-quality promotion remains separate.

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
shared wet remains shared
stable source evidence does not jitter across block sizes
new identity does not inherit stale pose motion
FM operators are not promoted to independent objects by default
whole-chip fidelity does not imply stem additivity
reference control remains available
```

A mechanically correct timed event path does not by itself prove that any particular source-placement policy sounds better. The final criterion for FullSphere is perceptual: the soundtrack should feel coherently mixed into a larger world rather than algorithmically scattered through one.
