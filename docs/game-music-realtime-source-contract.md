# Realtime game-music source contract

## Purpose

Omniphony's causal source path accepts already-separated source audio plus source/musical evidence from systems such as Game Music Interpreter. It is a streaming DSP boundary, not a prerendered soundtrack automation interface.

```text
causal source lanes
+ current source evidence
+ ordered intra-block evidence events
        ↓
Omniphony source presentation policy
        ↓
spatial renderer
        ↓
binaural output
```

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

## Evidence authority

The source ABI preserves these distinctions:

```text
native stereo route
!= authored 3-D position
!= inferred musical presentation
```

A source may carry native left/right gains as historical routing evidence without claiming that those gains are literal world coordinates.

An authored position passes through only when the source actually supplied one.

Musical fields such as foundation, foreground, diffuse, width and vertical affinity are presentation evidence. They can influence Omniphony's policy but do not become authored geometry.

The protected historical/reference mix is never accepted as an object lane.

## Identity and lane reuse

A physical source channel is not presentation identity.

The source renderer uses:

```text
persistent musical part, when present
otherwise source identity
```

for spatial-ramp continuity.

If an unrelated source reuses the same channel, its first new presentation event uses a zero-length pose ramp rather than interpolating from the outgoing source's last position. If the persistent musical part remains the same across a source/slot migration, ordinary smooth presentation continuity is retained.

This resets presentation motion only. It does not flush the entire room/binaural history merely because one source identity changed.

## Game Music Interpreter handoff

Game Music Interpreter's corresponding path is:

```text
raw spatial_source_block_view
→ prepare_block()
→ past-only musical role projection
→ projected_view()
→ Omniphony ABI 0.3 transport
→ this renderer
→ complete_block(raw block) after rendering
```

The final step intentionally uses GMI's raw block rather than its projected renderer sidecar. That prevents a semantic feedback loop in which an earlier role guess becomes evidence for itself.

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

A mechanically correct timed event path does not by itself prove that any particular source-placement policy sounds better.
