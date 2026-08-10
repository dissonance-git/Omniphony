# Always-on spatializer contract

Omniphony is intended to become normal playback infrastructure: something a listener can leave enabled for hours without thinking about the renderer itself.

The research surrounding it may be ambitious. The realtime product behavior must be boring.

> **libaural may be experimental. Omniphony's realtime core may not be experimental in the way it behaves.**

This document defines the stability boundary that must be satisfied before new perceptual features are allowed to dominate development.

---

## 1. Product identity

Omniphony is not a research dashboard, a DAW, or a model-hosting experiment.

At the listener boundary it should behave like a next-generation descendant of always-on headphone virtualization / upmix systems:

```text
ordinary playback
→ Omniphony
→ headphones
```

The difference is the quality and intelligence of the transformation, not additional ritual.

The user should not need to know whether the current scene decision came from:

- a simple stereo heuristic;
- a persistent local scene state;
- libaural;
- a slower learned specialist;
- a calibrated listener/headphone profile.

Those are implementation details behind one reliable audio device/path.

---

## 2. The realtime core is sovereign

The hard realtime renderer must remain able to process audio correctly when every optional subsystem is absent.

```text
OPTIONAL / ASYNCHRONOUS
libaural
learned models
network
UI
research workers
profile builders
telemetry consumers

        ↓ bounded validated publication only

ALWAYS-AVAILABLE REALTIME CORE
sample timeline
current conservative scene
trajectories
binaural / field / room DSP
headphone correction
output
```

If an optional subsystem stalls, crashes, returns late, produces invalid state, or disappears:

```text
KEEP PLAYING
+
keep last-known-good audible state
+
fall back conservatively when required
```

Never block audio waiting for intelligence.

---

## 3. Baseline mode must sound good by itself

Before advanced libaural-driven presentation is required, Omniphony needs a stable conservative baseline that already earns being left enabled.

That baseline should be comparable in product role to systems such as conventional headphone virtualization / surround upmixing, but should preserve the fork's stronger binaural foundation and music-fidelity rules.

Conceptually:

```text
stereo evidence
→ conservative stable scene
→ binaural renderer
→ optional room / calibration
```

A missing semantic/music model must never turn the product off or make playback fragile.

Advanced hearing should improve decisions above this floor.

---

## 4. Audio callback prohibitions

The steady-state realtime callback must not perform operations with unbounded or scheduler-dependent latency.

Forbidden in the final always-on path:

- blocking mutex/RwLock acquisition;
- filesystem access;
- network access;
- model inference that is not explicitly bounded realtime inference;
- device enumeration;
- SOFA/profile parsing;
- graph construction;
- unbounded heap allocation/deallocation;
- thread creation;
- waiting for worker completion;
- blocking logging;
- arbitrary callback-count-based control laws.

Any current occurrence is technical debt to remove before always-on release.

---

## 5. Allocation policy

The preferred lifecycle is:

```text
control thread
→ construct / reserve / prewarm
→ validate
→ publish ready state

realtime thread
→ reuse fixed/bounded storage
```

A first-use allocation hidden inside a musical transient is not acceptable merely because later callbacks are allocation-free.

At minimum, test separately:

```text
construction allocations
stream-preparation allocations
steady-state allocations
live profile/scene transition allocations
stream-reset behavior
```

A bounded stream reset should normally clear state **in place** and retain capacity.

---

## 6. No host-shaped sound

The host is a transport mechanism.

Changing legal callback partition must not redefine:

- gain slew;
- object motion;
- HRTF motion;
- bypass transition;
- profile crossfade;
- room modulation;
- head-pose interpolation;
- scene transition timing.

Required family:

```text
same PCM
+ same semantic/timed state
+ 40 / 128 / 240 / 480 / 960 / 1024 / irregular callback partitions
→ equivalent intended audible world
```

Exact waveform null is required where the algorithm should mathematically be identical.

Where a deliberately approximate perceptual control quantum is used, the tolerance and reason must be declared and the result must still be independent of callback boundaries.

---

## 7. Stream discontinuity law

A seek, decoder reset, new track, sample-rate restart, or other declared timeline discontinuity must not inherit old audio history accidentally.

Reset all stream-lifetime state that can contain previous audio or trajectory state, including as applicable:

- channel metadata/ramp history;
- gain boundaries;
- FIR history;
- ITD/fractional-delay history;
- crossover states;
- speaker delay history;
- reflection rings/taps;
- reverb/FDN stored energy;
- air/filter recursive state;
- per-stream loudness metadata.

Keep expensive immutable state when valid:

- current HRTF dataset;
- listener/headphone profile;
- precomputed tables;
- worker threads;
- reserved storage;
- session-level user settings.

Desired result:

```text
old stream ends
→ bounded in-place reset
→ new stream starts
→ zero stale audio
→ no renderer reconstruction pause
```

---

## 8. Failure containment

### HRTF/profile build fails

Keep current working profile.

### libaural/model result invalid or late

Ignore/reject it; keep current conservative scene.

### control message malformed

Reject on control plane; audio continues.

### queue overload

Apply declared coalescing/drop policy; never grow memory without bound.

### optional room/field processor fails validation

Disable/revert that optional layer rather than corrupt direct sound.

### NaN/non-finite state

Contain before it reaches the output bus; record a diagnostic outside the callback.

---

## 9. Latency budget

Low latency is part of correctness for an always-on spatializer.

Track separately:

```text
host/device buffering
algorithmic lookahead
FIR / convolution latency
resampling latency
control-state latency
optional hearing/model latency
```

The base realtime spatializer should not wait for slower hearing/music cognition.

Slower inference influences future presentation when it arrives.

It does not retroactively stall present audio.

---

## 10. CPU budget and worst case

Average CPU is insufficient.

Measure:

- median callback time;
- p95;
- p99;
- p99.9 where practical;
- maximum observed callback time;
- callback deadline misses;
- cost by active object count;
- moving versus stationary objects;
- reflections/reverb on/off;
- profile transitions;
- stream reset;
- HRTF updates;
- sample-rate/device changes.

A feature that is cheap 99% of the time but occasionally overruns the device buffer is not lightweight.

Prefer measured fixed control quanta, caching and interpolation over brute-force per-sample recomputation where perception does not require it.

---

## 11. Fidelity floor

No stability optimization may quietly make the renderer sound worse.

Every major change must preserve or improve:

- transient timing;
- bass timing/weight;
- timbre;
- dynamics;
- centre authority;
- vocal/instrument identity;
- groove/microtiming;
- stereo relationships that remain musically important;
- absence of clicks/zipper noise;
- absence of pumping or unexpected level changes.

Matched-loudness bypass remains the final sanity check.

---

## 12. Long-duration soak tests

A product intended to stay enabled needs tests that unit fixtures cannot replace.

Before always-on release, run automated and listening soaks covering at least:

```text
hours of continuous playback
many track boundaries
seek / pause / resume
sample-rate changes
mono / stereo / multichannel changes
silence → loud transient
quiet material
bass-heavy material
highly dynamic material
dense masters
sparse acoustic music
rapid device/control changes
optional worker failure / restart
```

Record:

- glitches/xruns;
- max callback time;
- peak memory growth;
- allocations after warm-up;
- non-finite samples;
- clipping interventions;
- state publication failures;
- worker errors;
- reset counts.

Memory and CPU should reach bounded steady state rather than creep over time.

---

## 13. Release ladder

No large new rendering feature should outrank these gates.

```text
A. core compiles/tests on supported proving platforms
B. deterministic DSP fixtures green
C. callback-partition invariance green
D. stream reset / seek boundaries clean
E. steady-state realtime allocation audit green
F. blocking-control-path audit green
G. worst-case callback budget green
H. device/sample-rate lifecycle green
I. matched-loudness fidelity gates green
J. multi-hour soak green
```

Only after the floor is boring should new scene intelligence become the dominant source of risk again.

---

## 14. Research integration rule

New libaural/AI capability enters Omniphony through a bounded projection.

```text
research state
→ validate
→ compress to small presentation update
→ timestamp
→ publish
→ realtime core accepts or rejects
```

The core never receives an instruction equivalent to:

```text
"wait while I understand the song"
```

It receives something more like:

```text
object 3: confidence 0.91
role: secondary melodic layer
presentation allowance: broad lateral expansion
valid from sample T forward
```

If that update never arrives, playback is still correct and stable.

---

## 15. The always-on acceptance sentence

> **Omniphony is ready to be treated as normal playback infrastructure when a listener can leave it enabled indefinitely and the only recurring reason to notice it is that turning it off makes the presentation worse.**
