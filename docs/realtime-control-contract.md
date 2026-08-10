# Realtime control and sample-time contract

This document defines realtime correctness for Omniphony. It is subordinate to the root `README.md` and must not become an infrastructure prerequisite ladder that delays the first useful Windows listening path.

The core law remains:

> **Omniphony's audible world must not be defined by a host callback, UI refresh interval, model-worker completion time, or device-buffer size.**

The canonical time domain for audible renderer state is the audio sample timeline.

Current product split:

```text
WINDOWS / CONTROL SIDE
UI and development controls
configuration
device management
HRTF/profile construction
optional model/libaural workers
file/network I/O
logging
        ↓ bounded validated publication

REALTIME AUDIO SIDE
sample clock
current scene/presentation state
bounded trajectories
binaural / room DSP
output samples
```

The realtime engine must remain independently useful with no AI/model worker attached.

---

## 1. Realtime processor independence

The audio processor must be able to run correctly without:

- a GUI;
- a controller window;
- a model server;
- libaural;
- network access;
- filesystem access during processing;
- a calibration database connection;
- a logging consumer.

Minimum running state:

```text
current bounded configuration
+ realtime DSP state
+ bounded timed-event input
+ audio buffers
```

A future UI or hearing model is an observer/controller, not part of the renderer's existence.

---

## 2. Canonical sample clock

Maintain a monotonic renderer sample position.

Conceptually:

```text
RendererClock
  sample_rate_hz
  absolute_sample_index
  stream_generation
```

`stream_generation` distinguishes real discontinuities such as:

- new playback stream;
- explicit seek/reset;
- sample-rate reinitialization;
- device/engine restart where continuity cannot be preserved.

A different callback size is not a semantic discontinuity.

---

## 3. Timed render events

Audible control changes need explicit time semantics.

Conceptual form:

```text
TimedRenderEvent
  stream_generation
  absolute_sample_time?
  block_sample_offset?
  event_id
  source
  kind
  payload
  request_generation?
  provenance?
```

Possible event kinds:

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

An event says when an audible change becomes true, not when a UI or worker happened to send it.

---

## 4. Block-local offsets are transport, not truth

Normalize host-local offsets to the renderer timeline where practical.

Equivalent source/control timing with different legal block partitions should render equivalently apart from explicitly documented buffering latency.

```text
same source
+ same timed events
+ different callback partition
→ equivalent intended output within tolerance
```

This protects the sound from host implementation details.

---

## 5. Targets versus trajectories

A target event and the audible path toward that target are different things.

```text
TARGET EVENT
where state should go

TRAJECTORY POLICY
how audible state gets there
```

Examples of trajectory classes:

```text
instantaneous semantic change
sample ramp
smooth motion
crossfade state
```

Use instant changes only for genuinely discontinuous semantics.

---

## 6. Gain continuity

Gain changes capable of clicks or modulation artifacts should be sample-ramped or analytically equivalent.

Bad:

```text
for each callback:
  gain += fixed_step
```

because callback size changes the time constant.

Good:

```text
elapsed_samples / sample_rate
→ time-domain interpolation
```

Apply the same principle to:

- object gain;
- direct/field blend;
- room send;
- profile crossfades;
- bypass;
- HRTF transitions;
- calibration transitions.

---

## 7. Position and HRTF continuity

The renderer already has stateful HRTF transition machinery. Position control should eventually expose an authoritative sample-time trajectory rather than forcing the HRTF stage to infer motion from arbitrary block-start updates.

Desired chain:

```text
sample-time object trajectory
→ listener-relative trajectory
→ HRTF target trajectory
→ bounded filter transition
→ ears
```

This remains a known renderer-correctness candidate, but it should be pulled forward according to audible/product need rather than treated as a reason to postpone the Windows host lane.

---

## 8. Control work is not realtime work

Keep unpredictable/blocking work away from the audio callback:

- filesystem access;
- network calls;
- ordinary model inference;
- SOFA parsing;
- HRTF import/resampling;
- headphone-EQ optimization;
- large FFT-plan construction;
- heap-heavy graph construction;
- device enumeration;
- UI calls;
- ordinary mutex waits;
- blocking logging.

Workers may build immutable candidate state and publish a bounded reference to the realtime side.

---

## 9. Transactional state publication

Construction beginning does not make a candidate authoritative.

Use:

```text
request generation N
→ build candidate away from audio thread
→ validate
→ publish only if N is still current
→ otherwise discard stale result
```

On failure:

```text
KEEP LAST-KNOWN-GOOD AUDIBLE STATE
+
report error outside realtime processing
```

This generalizes the existing stale-HRIR-rebuild protection.

---

## 10. Publication versus audible switching

A validated state may still require a bounded audible transition.

```text
candidate validated
→ atomically available
→ transition scheduled at sample T
→ old/new coexist during bounded crossfade
→ retire old state
```

Useful for:

- HRTF/profile changes;
- room response;
- convolution kernels;
- bypass;
- scene-topology changes.

---

## 11. Last-known-good law

A bad update should produce a diagnostic, not destroy the sounding world.

Examples:

```text
invalid SOFA
→ old HRTF remains

failed correction build
→ old profile remains

optional model timeout
→ existing conservative presentation continues

invalid room parameter
→ reject instead of corrupting DSP state
```

The protected baseline is always more valuable than speculative broken state.

---

## 12. Deterministic event ordering

Events at the same sample need explicit deterministic ordering.

A useful conceptual order is:

```text
1. stream/discontinuity
2. topology/profile publication
3. object/scene membership
4. position/extent targets
5. gain/blend targets
6. presentation controls
```

Exact ordering can evolve, but thread scheduling must never decide audible output.

---

## 13. Bounded event transport

Realtime queues must be bounded.

Useful overload classes:

### Latest-state-wins
For dense continuous state such as pose updates.

### Must-deliver
For semantic discontinuities/topology changes where dropping produces inconsistent state.

### Coalescible targets
For multiple future trajectory targets where intermediate stale updates can be reduced.

Diagnostics should expose counts such as:

```text
events_received
events_coalesced
events_dropped
late_events
queue_high_watermark
```

Silent overflow is unacceptable.

---

## 14. Late-event policy

Timed event families should define what happens if an event arrives after its intended sample.

Possible policies:

```text
apply_immediately_with_diagnostic
short_catchup_ramp
reject_if_stale
recompute_from_newest_state
```

Head tracking, UI state and optional model evidence may use different policies.

---

## 15. Optional model / libaural updates

This section describes a future optional input, not a current dependency.

A slower hearing/model result should carry the time interval it describes plus uncertainty/confidence. Omniphony decides whether it is still actionable.

```text
result describes [t0, t1]
arrives at t2
        ↓
Omniphony asks
  is it still useful?
  does it change persistent state?
  should it affect a future trajectory?
  is confidence sufficient?
```

Never pretend an inference was known earlier merely because it describes earlier audio.

If no model/libaural worker exists, the baseline renderer and local bounded scene path continue normally.

---

## 16. UI and diagnostics cadence

UI/diagnostics may run at low cadence:

```text
10 Hz
30 Hz
60 Hz
```

without changing realtime semantics.

A slow or absent UI must not stall audio.

Heavy plots, model explanations and history queries stay off the realtime thread.

---

## 17. Bypass contract

Matched-loudness bypass is both a research instrument and eventual user feature.

Bypass should be:

- artifact-free;
- latency-aligned where required;
- sample-time controllable;
- free from a loudness advantage;
- capable of preserving useful DSP state so re-entry is not a fake reset comparison.

Conceptually:

```text
processed latency-matched path
raw latency-matched path
        ↓
short validated transition
```

The desired result is spatial collapse, not discovery that bypass restores fidelity.

---

## 18. Windows host independence

The same engine semantics should survive:

```text
file/reference fixture
normal Windows system route
ASIO specialist route
future virtual endpoint if needed
unit/integration harness
```

Windows host adapters translate their timing/device conventions into the engine contract.

The renderer must not grow one motion law for WASAPI and another for ASIO.

Other operating systems are outside the current roadmap.

---

## 19. Clock-domain ownership

If Windows capture and output use different hardware clocks, name them explicitly.

Conceptually:

```text
CaptureClock
RendererClock
OutputClock
```

Define:

- master clock;
- drift measurement;
- resampling location;
- allowed correction rate;
- target buffering;
- hard-recovery behavior;
- diagnostics.

Inherited adaptive-resampling mechanisms may be useful, but do not preserve an old transport servo merely because it exists. Clock ownership should be understandable in the final Windows host path.

---

## 20. Core fixtures

Useful regression fixtures include:

### RTC-001 · block partition invariance
Same continuous input/events with several callback partitions.

### RTC-002 · timed gain event
Gain change begins at the same absolute sample across legal host partitions.

### RTC-003 · moving source trajectory
Equivalent intended continuous motion across control/update partitions.

### RTC-004 · stale async build
A late older build cannot replace a newer accepted state.

### RTC-005 · failed candidate state
Current audio remains alive after invalid HRTF/profile/room requests.

### RTC-006 · profile switch continuity
Bounded deterministic artifact-free switch.

### RTC-007 · event queue pressure
Bounded memory and declared coalescing/drop behavior.

### RTC-008 · late event
Declared late-event policy is deterministic.

### RTC-009 · bypass alignment
No temporal jump/loudness trick during comparison.

### RTC-010 · processor without controller
Complete deterministic renderer with no UI/model attached.

Do not require every fixture to be solved before W1 native Windows listening unless the failure directly threatens that listening path or protected sound.

---

## 21. Current implementation relevance

The immediate product frontier is the Windows host lane described in the root README.

This contract should guide that work without taking over its priority:

```text
Windows audio transport
→ one engine sample timeline
→ existing renderer
→ output
```

When scene/motion improvements are pulled forward, prefer one authoritative trajectory representation rather than independent motion state machines inside multiple DSP stages.

When optional libaural/adaptive work becomes live, feed it through the same bounded publication contract.

---

## 22. Acceptance law

A realtime renderer is correct when changing machinery around the same semantic event does not redefine the intended sound.

```text
same audio
same semantic controls
same sample timeline

+ different legal callback partition
+ different UI cadence
+ different equivalent worker completion timing
+ different Windows host route

→ same intended auditory world
```

The host supplies samples and timing opportunities.

It does not get to redesign the geometry of the music.

And this contract does not get to redesign the project roadmap. The root README remains the authority.