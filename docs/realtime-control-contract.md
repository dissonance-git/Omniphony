# Realtime control and sample-time contract

This document defines realtime correctness for Omniphony.

It is subordinate to the root `README.md` and must not become an infrastructure ladder that delays useful listening.

Core law:

> **Omniphony's audible world must not be defined by a host callback, UI refresh interval, platform buffer size, device name, model-worker completion time, or global channel mode.**

The canonical time domain for audible state is the audio sample timeline.

---

## 1. Portable realtime boundary

Conceptual split:

```text
PLATFORM / CONTROL SIDE
Windows / future macOS / future Linux host
UI and settings
device/session discovery
platform routing
HRTF/profile construction
optional model/libaural workers
file/network I/O
logging
        ↓ bounded validated publication

PORTABLE REALTIME SIDE
logical input streams
sample clocks / generations
current presentation/scene state
bounded trajectories
binaural / room DSP
stereo output samples
```

The realtime engine must remain independently useful with no GUI, network, libaural worker or platform-specific audio API attached.

---

## 2. Stream-local format law

Channel layout belongs to a logical source stream.

Conceptual contract:

```text
InputStreamState
  stream_id
  sample_rate_hz
  channel_layout
  stream_generation
  absolute_sample_index
  optional spatial/object metadata
```

Valid simultaneous state may include:

```text
Stream A = stereo music
Stream B = 7.1 game
Stream C = mono voice
```

The renderer must not require one global:

```text
current_channel_mode = stereo | 5.1 | 7.1
```

Starting, stopping or changing one stream must not reinterpret unrelated streams.

A platform host may temporarily provide an already-mixed bed. That limitation belongs to that host, not to the core contract.

---

## 3. Canonical sample clocks

Each independently meaningful stream needs monotonic timing.

Conceptually:

```text
StreamClock
  sample_rate_hz
  absolute_sample_index
  stream_generation
```

The renderer may also own a shared output/world timeline.

`stream_generation` distinguishes real discontinuities such as:

- new source stream;
- seek/reset;
- sample-rate reinitialization;
- source-layout restart;
- platform restart where continuity cannot be preserved.

A different callback size is not a semantic discontinuity.

---

## 4. Timed render events

Audible control changes need explicit time semantics.

Conceptual form:

```text
TimedRenderEvent
  stream_id?
  stream_generation
  absolute_sample_time?
  block_sample_offset?
  event_id
  source
  kind
  payload
  request_generation?
```

Possible kinds:

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
stream_start
stream_stop
stream_discontinuity
```

An event says when an audible change becomes true, not when a UI thread happened to send it.

---

## 5. Callback partition invariance

Equivalent source/control timing with different legal block partitions should render equivalently apart from declared buffering latency.

```text
same source streams
+ same timed events
+ same semantic timeline
+ different legal callback partition
→ equivalent intended output within tolerance
```

This protects the sound from host implementation details.

---

## 6. Targets versus trajectories

A target event and the audible path toward that target are different.

```text
TARGET EVENT
where state should go

TRAJECTORY POLICY
how audible state gets there
```

Trajectory classes may include:

```text
instant semantic change
sample ramp
smooth motion
bounded crossfade
```

Use instant changes only for genuinely discontinuous semantics.

---

## 7. Gain / position / HRTF continuity

Changes capable of clicks, zipper noise or unstable localization should be sample-ramped or analytically equivalent.

Bad:

```text
for each callback:
  gain += fixed_step
```

Good:

```text
elapsed_samples / sample_rate
→ time-domain interpolation
```

The same principle applies to:

- direct/field blend;
- room send;
- profile changes;
- bypass;
- HRTF transitions;
- listener calibration;
- object motion.

Desired position chain:

```text
sample-time trajectory
→ listener-relative trajectory
→ HRTF target
→ bounded filter transition
→ ears
```

---

## 8. Control work is not realtime work

Keep unpredictable/blocking work away from realtime audio:

- filesystem access;
- network calls;
- ordinary model inference;
- SOFA parsing;
- HRTF import/resampling;
- headphone-EQ optimization;
- large graph construction;
- device/session enumeration;
- UI calls;
- blocking logging;
- unbounded allocation/queues.

Workers may build immutable candidate state and publish a bounded reference inward.

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

This generalizes existing stale-HRIR protection.

---

## 10. Publication versus audible switching

A validated state may still require a bounded audible transition.

```text
candidate validated
→ atomically available
→ transition scheduled at sample T
→ old/new coexist during bounded transition
→ retire old state
```

Useful for:

- HRTF/profile changes;
- room response;
- convolution kernels;
- scene topology;
- listener profiles.

---

## 11. Bypass is an output-route decision

Matched-loudness bypass is both a research instrument and a user feature.

Hard law:

> **OFF must not leak previously selected wet audio merely because it was queued earlier.**

The current prototype selects wet/dry before a bounded playback queue. That means a toggle can leave stale wet blocks waiting to reach the physical output.

That prototype behavior is explicitly not the final contract.

Preferred architecture:

```text
input frames
  ├→ processed path
  └→ latency-matched comparison path
        ↓
paired/aligned output state
        ↓
selection near physical output
        ↓
short validated transition
```

Switching OFF must invalidate or bypass stale wet selection immediately enough that no perceptible wet tail remains solely because of queue history.

Bypass should ultimately be:

- artifact-free;
- latency-aligned;
- sample-time controllable;
- loudness-fair;
- free of wet queue leakage;
- free of duplicate external forwarding;
- capable of preserving useful DSP state so re-entry is not a fake reset comparison.

For development, a brief gap is preferable to an ambiguous double/phase path.

---

## 12. Single physical output law

Realtime correctness extends beyond the core renderer.

If two independent routes reach the same physical headphones:

```text
old dry/ASIO path
+
Omniphony path
```

then small timing differences can create comb filtering, echo and hallway-like coloration.

That is not a valid A/B environment.

The platform host/test harness must expose enough diagnostics to establish:

```text
one physical audible route
```

before subtle listening judgments are accepted.

---

## 13. Bounded event/PCM transport

Realtime queues must be bounded.

Useful overload classes:

### Latest-state-wins
Dense continuous control such as pose updates.

### Must-deliver
Semantic discontinuities/topology changes.

### Coalescible targets
Intermediate future targets that can be reduced safely.

Diagnostics should eventually expose:

```text
events_received
events_coalesced
events_dropped
late_events
queue_high_watermark
pcm_queue_high_watermark
underruns
overruns
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

## 15. Optional libaural/model updates

This is a future optional input, not a dependency.

A slower hearing/model result should carry the interval it describes plus uncertainty/confidence.

```text
result describes [t0, t1]
arrives at t2
        ↓
Omniphony asks:
  is it still useful?
  does it change persistent state?
  should it affect a future trajectory?
  is confidence sufficient?
```

Never pretend an inference was known earlier merely because it describes earlier audio.

If no model/libaural worker exists, baseline playback remains complete.

---

## 16. UI cadence independence

UI/diagnostics may run at:

```text
10 Hz
30 Hz
60 Hz
```

or disappear entirely without changing realtime semantics.

The current `Omniphony.exe` / hidden worker split is consistent with this law.

A slow GUI must not stall audio.

---

## 17. Platform host independence

The same core semantics should survive:

```text
file/reference fixture
Windows host
future macOS host
future Linux host
ASIO specialist route
future virtual endpoint / native route
unit/integration harness
```

Platform adapters translate device/session/timing conventions into the core contract.

Do not grow one motion law for WASAPI and another for another operating system.

Windows is the current implementation priority, not the renderer ontology.

---

## 18. Clock-domain ownership

If capture/input/output use different hardware or logical clocks, name them explicitly.

Conceptually:

```text
InputClock(s)
RendererClock
OutputClock
```

Define:

- master clock;
- per-stream timestamp mapping;
- drift measurement;
- resampling location;
- allowed correction rate;
- target buffering;
- hard-recovery behavior;
- diagnostics.

Clock ownership should be understandable in each platform host.

---

## 19. Core regression fixtures

### RTC-001 · block partition invariance
Same continuous input/events across several callback partitions.

### RTC-002 · timed gain event
Gain change begins at the same absolute sample.

### RTC-003 · moving source trajectory
Equivalent intended motion across control partitions.

### RTC-004 · stale async build
Older completion cannot replace newer accepted state.

### RTC-005 · failed candidate state
Current audio remains alive after invalid state publication.

### RTC-006 · profile switch continuity
Bounded deterministic switch.

### RTC-007 · queue pressure
Bounded memory and declared overload behavior.

### RTC-008 · late event
Declared late-event policy is deterministic.

### RTC-009 · bypass queue cleanliness
Toggle OFF does not emit previously queued wet-selected audio.

### RTC-010 · processor without controller
Complete renderer with no UI/model attached.

### RTC-011 · concurrent layout independence

```text
stereo stream + 7.1 stream
```

can coexist without either changing the other's semantic layout.

### RTC-012 · stream lifecycle isolation
Starting/stopping one source does not reset unrelated streams.

---

## 20. Current implementation relevance

Immediate frontier:

```text
prove one physical Windows path
→ clean bypass
→ fair stereo music baseline
→ surround baseline
→ stereo + surround simultaneously
```

The first live Windows app already proved arbitrary audio transport.

The next realtime correction is known: move wet/dry selection to a point where queued history cannot leak the old choice.

---

## 21. Acceptance law

A realtime renderer is correct when machinery around the same semantic event does not redefine the intended sound.

```text
same logical streams
same per-stream layouts
same audio
same semantic controls
same sample timelines

+ different legal callback partitions
+ different UI cadence
+ different equivalent worker timing
+ different platform host

→ same intended auditory world
```

The host supplies samples, metadata and timing opportunities.

It does not get to redesign the geometry of the music.
