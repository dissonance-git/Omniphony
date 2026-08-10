# Realtime control and sample-time contract

Omniphony's audible world must not be defined by a host callback, a UI refresh interval, a model-worker completion time, or an operating-system device buffer.

The canonical time domain for audible renderer state is the **audio sample timeline**.

This document defines the boundary between:

```text
CONTROL / RESEARCH PLANE
UI
configuration
libaural/model workers
HRTF/profile construction
file/network I/O
device management
calibration
logging

        ↓ bounded, timestamped state publication

REALTIME AUDIO PLANE
sample clock
scene trajectories
object gain/position
binaural filters
room state
output samples
```

The design is influenced by mature realtime contracts such as Steinberg VST3's separation of processor and edit controller, its sample-accurate automation model, and graph-oriented systems such as Glicol where a failed graph update need not destroy the currently sounding graph.

The concepts here are host-agnostic. Omniphony does not need to become a VST3 plug-in to benefit from them.

---

# 1. Realtime processor independence

The audio processor must be able to run correctly without:

- a GUI;
- a controller window;
- a model server;
- network access;
- filesystem access;
- a calibration database connection;
- a logging consumer.

The minimum running state is:

```text
immutable / bounded current configuration
+
realtime DSP state
+
bounded timed-event input
+
audio buffers
```

A future UI is an observer/controller of the renderer, not part of the renderer's existence.

---

# 2. Canonical sample clock

Maintain a monotonic renderer sample position.

Conceptually:

```text
RendererClock
  sample_rate_hz
  absolute_sample_index
  stream_generation
```

`stream_generation` distinguishes discontinuous timelines such as:

- a new playback stream;
- explicit seek/reset;
- sample-rate reinitialization;
- device/engine restart where continuity cannot be preserved.

Do not infer semantic discontinuity merely because a host chose a different callback size.

---

# 3. Timed render event

Audible control changes should have explicit time semantics.

Conceptual contract:

```text
TimedRenderEvent
  stream_generation
  absolute_sample_time?       # preferred across block boundaries
  block_sample_offset?        # host-local form when supplied that way
  event_id
  source
  kind
  payload
  request_generation?
  provenance?
```

Examples of `kind`:

```text
object_position_target
object_gain_target
object_extent_target
field_gain_target
head_pose
bypass
presentation_profile_switch
listener_profile_switch
room_parameter_target
scene_candidate_publish
stream_discontinuity
```

The event says **when an audible change becomes true**, not when a UI thread happened to send it.

---

# 4. Block-local offsets are transport, not truth

A host may deliver an event as:

```text
current block + sample offset
```

Normalize it immediately to the renderer's sample timeline where possible.

Equivalent input:

```text
block 1: 128 samples
block 2: 128 samples

vs

block 1: 64
block 2: 192
```

must not change the intended event trajectory when the same event occurs at the same absolute sample.

Required invariant:

```text
same source
+
same timed events
+
different legal callback partition
→ equivalent rendered samples within numerical tolerance
```

apart from explicitly documented buffering/latency behavior.

---

# 5. Parameter event versus trajectory

A scene object does not teleport merely because one control message arrived.

Separate:

```text
TARGET EVENT
where the state should go

TRAJECTORY POLICY
how the audible state moves there
```

Example:

```text
ObjectPositionTarget
  object_id
  target_position
  target_sample_time

TrajectoryPolicy
  interpolation
  duration_samples
  velocity_limit?
  continuity_class
```

Possible continuity classes:

```text
instantaneous_semantic_change
sample_ramp
smooth_motion
crossfade_state
```

Use instantaneous changes only when the underlying semantics are genuinely discontinuous.

---

# 6. Gain continuity

Gain changes that can create clicks or modulation artifacts must be sample-ramped or otherwise continuously transformed.

The gain trajectory must be invariant to callback size.

Bad:

```text
for every process_block:
  gain += fixed_step
```

because a 40-sample host and a 1024-sample host produce different time constants.

Good:

```text
elapsed_samples / sample_rate
→ time-domain interpolation
```

or an analytically equivalent sample-indexed ramp.

This law applies to:

- object gain;
- direct/field blend;
- room send;
- profile crossfades;
- bypass;
- HRTF transitions;
- calibration-filter transitions.

---

# 7. Position and HRTF continuity

Object motion should not create filter discontinuities.

The renderer already has stateful HRTF interpolation/crossfade machinery. Its control boundary should make the intended trajectory explicit so the filter path does not infer motion from arbitrary host update timing.

Desired chain:

```text
sample-time object trajectory
→ listener-relative trajectory
→ directional HRTF target trajectory
→ bounded stateful filter transition
→ ears
```

A UI updating pose at 30 or 60 Hz may still drive a smooth sample-time trajectory between observations.

Head tracking should declare:

- observation timestamp when available;
- arrival timestamp;
- prediction/extrapolation policy if any;
- smoothing time constant;
- stale-pose policy.

Do not confuse network/OSC arrival jitter with head motion.

---

# 8. Control-plane work is never realtime work

The realtime process function must not perform operations that can unpredictably block or allocate large state.

Keep outside the audio thread:

- filesystem access;
- network calls;
- model inference unless a separately designed bounded realtime model exists;
- SOFA parsing;
- HRTF resampling/import;
- headphone-EQ optimization;
- large FFT-plan construction;
- heap-heavy graph construction;
- device enumeration;
- UI calls;
- ordinary mutex waits;
- logging that can block.

Control-plane workers may build immutable candidate state and publish a small reference/handle to the realtime plane.

---

# 9. Transactional state publication

A new graph/profile/configuration is not authoritative merely because construction started.

Use:

```text
request generation N
→ build candidate off thread
→ validate candidate
→ if N is still current:
     publish
   else:
     discard stale result
```

This generalizes the existing stale-HRIR-rebuild protection.

Candidate state examples:

- HRTF set;
- headphone correction profile;
- long convolution partitions;
- room model;
- scene-render plan;
- model/provider result that changes rendering topology;
- device/output configuration.

On build failure:

```text
KEEP LAST-KNOWN-GOOD AUDIBLE STATE
+
report error on control/diagnostic plane
```

Never replace working audio with half-built state merely because a settings change was requested.

---

# 10. Publication is not necessarily audible switching

After a candidate becomes valid, the realtime processor may still need an audible transition.

```text
candidate validated
→ atomically available
→ transition scheduled at sample T
→ old/new state coexist for bounded crossfade
→ retire old state after transition
```

This is especially important for:

- HRTF changes;
- listener/headphone profiles;
- room response;
- convolution kernel replacement;
- bypass;
- scene topology changes.

The control plane owns construction. The realtime plane owns artifact-free audible handoff.

---

# 11. Last-known-good graph law

Inspired by robust live-audio graph systems:

> A bad update should produce an error message, not silence or a corrupted sound world.

Examples:

```text
invalid SOFA file
→ old HRTF stays active

failed headphone filter build
→ old profile stays active

model worker timeout
→ conservative existing scene continues

invalid room parameter
→ reject update rather than NaN the FDN
```

Where safe, a neutral fallback may be preferable to stale state, but that must be an explicit policy for the state type.

---

# 12. Deterministic event ordering

Several events can target the same sample.

Define deterministic ordering.

Conceptually:

```text
1. stream/discontinuity events
2. topology/profile publication
3. object existence / scene membership
4. position/extent targets
5. gain/blend targets
6. presentation controls
```

The exact ordering may change, but it must be explicit and tested.

For two events of the same class at the same sample, use a stable sequence/event ID or define last-write semantics.

Never let thread scheduling decide audible output.

---

# 13. Bounded event transport

The realtime event queue must be bounded.

A producer that outruns the audio consumer requires an explicit overload policy.

Possible policies by event class:

### Latest-state-wins

Good for dense pose/continuous control updates where intermediate points have become stale.

### Must-deliver

Required for semantic discontinuities or topology state where dropping the event would leave inconsistent state.

### Coalescible trajectory targets

Multiple future targets may be reduced while preserving the newest valid trajectory endpoints.

Diagnostics should expose:

```text
events_received
events_coalesced
events_dropped
late_events
queue_high_watermark
```

A silent queue overflow is unacceptable in an auditory engine.

---

# 14. Late-event policy

An event may arrive after its intended sample time.

Do not leave behavior undefined.

Candidate policies:

```text
apply_immediately_with_diagnostic
short_catchup_ramp
reject_if_semantically_stale
recompute_from_newest_state
```

Head tracking, model inference and UI control may require different policies.

Every timed event family should declare its late-event rule.

---

# 15. Model / libaural updates

libaural may operate at much slower and less deterministic cadence than the audio thread.

That is acceptable.

A model result should contain the interval it describes and its evidence/confidence. The Omniphony adapter decides how that evidence can influence future presentation.

```text
libaural result describes [t0, t1]
arrives at t2
        ↓
Omniphony policy
  Is this still actionable?
  Does it alter persistent identity?
  Is a future trajectory appropriate?
  Is confidence high enough?
```

Never pretend an inference was known earlier merely because it describes earlier audio.

Offline lookahead and realtime causal modes should remain distinguishable.

---

# 16. UI / diagnostics cadence

The UI and diagnostics do not need sample-rate updates.

They may observe renderer state at a much lower cadence:

```text
10 Hz
30 Hz
60 Hz
```

without changing realtime semantics.

Use snapshots/telemetry that are safe to produce from the audio side and consume elsewhere.

Heavy FFT plots, model explanations or history queries should be generated away from the realtime process.

A slow or absent UI must never stall audio.

---

# 17. Bypass contract

Matched-loudness bypass is a core Omniphony research instrument and eventual user feature.

Bypass must be:

- artifact-free;
- time-aligned where processing latency exists;
- sample-time controllable;
- free from a loudness advantage;
- capable of preserving processing state where useful so re-entry does not sound like a reset.

Conceptual transition:

```text
processed delayed path
raw latency-matched path
        ↓
short bounded equal-power / validated crossfade
```

Do not use bypass as `if enabled { copy input }` when the processed route carries latency or state that makes the comparison invalid.

---

# 18. Host/device independence

These semantics must survive different hosts:

```text
file fixture
WASAPI/system route
ASIO optional route
future virtual device
future VST3 diagnostic/plugin host
unit/integration harness
```

Host adapters are responsible for converting their timing/control conventions into the renderer contract.

The renderer must not grow separate motion laws for WASAPI and ASIO.

---

# 19. Clock-domain ownership

If capture and output use different device clocks, the system must name the clock domains explicitly.

Conceptually:

```text
CaptureClock
RendererClock
OutputClock
```

Define:

- which clock is master;
- how drift is measured;
- where resampling occurs;
- maximum allowed correction rate;
- buffer target;
- hard recovery behavior;
- diagnostics.

The current inherited adaptive-resampling machinery may provide useful mechanisms, but the final Windows architecture should make clock ownership obvious rather than inheriting a mysterious servo from an earlier transport path.

---

# 20. Required fixtures

## RTC-001 · block partition invariance

Same continuous input and events rendered with multiple callback partitions.

Expected: equivalent output within tolerance.

## RTC-002 · sample-offset gain event

Schedule a gain change at a known absolute sample.

Expected: transition begins at the same sample regardless of host block layout.

## RTC-003 · moving source trajectory

Feed the same position trajectory through several control-update cadences and callback sizes.

Expected: equivalent intended continuous trajectory after interpolation policy.

## RTC-004 · stale asynchronous build

```text
request A
request B
B completes
A completes late
```

Expected: B remains authoritative; A is rejected.

## RTC-005 · failed candidate graph

Request an invalid HRTF/profile/room state.

Expected: current audio continues with old state; error appears diagnostically.

## RTC-006 · profile switch continuity

Switch headphone/listener profile during sustained audio.

Expected: bounded click-free transition with deterministic duration.

## RTC-007 · event queue pressure

Overload a coalescible high-rate control source.

Expected: bounded memory; declared coalescing/drop behavior; diagnostics count it; no realtime stall.

## RTC-008 · late event

Deliver a timed control after its timestamp.

Expected: event-family late policy is followed deterministically.

## RTC-009 · bypass alignment

Toggle matched-loudness bypass with a processing path that has known latency.

Expected: no temporal jump and no click; bypass comparison remains meaningful.

## RTC-010 · processor without controller

Run the complete renderer with no UI/control consumer attached.

Expected: normal deterministic processing.

---

# 21. Immediate implementation relevance

This contract directly clarifies the next binaural refactor.

Current target:

```text
stereo / scene evidence
→ persistent scene state
→ sample-time object gain + position trajectories
→ binaural renderer
```

Do **not** create a second motion state machine inside the HRTF renderer.

Prefer one authoritative trajectory representation feeding all presentation stages.

A useful implementation sequence is:

```text
1. introduce canonical sample clock / timed event representation
2. route binaural object gain through sample-time trajectories
3. route binaural position through the same timing model
4. prove block-size invariance
5. adapt head tracking to timestamped observations
6. generalize transactional state publication beyond HRTF rebuilds
7. use the same contract when libaural scene updates become live
```

---

# 22. Acceptance law

A realtime renderer is correct when changing the machinery around it does not change the intended auditory event.

```text
same audio
same semantic controls
same sample timeline

+ different callback size
+ different UI refresh rate
+ different worker completion timing where results are equivalent
+ different host adapter

→ same intended audible world
```

The host supplies samples and timing opportunities.

It does not get to redefine the geometry of the music.
