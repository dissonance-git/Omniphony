# Always-on spatializer contract

Omniphony is intended to become normal playback infrastructure: something a listener can leave enabled for hours without thinking about the renderer itself.

The research surrounding it may be ambitious. The realtime product behavior must be boring.

> **libaural may be experimental. Omniphony's shipped realtime behavior may not be experimental in the way it behaves.**

This document defines the stability boundary that must be satisfied before new perceptual features are allowed to dominate development.

---

## 1. Product identity

Omniphony is not a research dashboard, a DAW, a model host, or a realtime AI mixing service.

At the listener boundary it should behave like a next-generation descendant of always-on headphone virtualization / upmix systems:

```text
ordinary playback
→ Omniphony
→ headphones
```

The difference is the quality and intelligence of the transformation, not additional ritual.

libaural, AI systems, auditory research, listening experiments and external models belong primarily to the **development process**. Their useful discoveries are validated, compressed and compiled into Omniphony as deterministic algorithms, policies, coefficients, classifiers, priors, tables or other bounded local mechanisms.

The released player should not need a live LLM, network service or general AI reasoning loop to decide how the current song is mixed.

```text
research / libaural / experiments / models
                 ↓
        validated discoveries
                 ↓
 algorithms + policies + parameters + assets
                 ↓
         compiled Omniphony release
                 ↓
      ordinary realtime playback
```

Research compounds across versions. Playback inherits the result as engineering.

---

## 2. The realtime core is sovereign

The hard realtime renderer must remain self-contained for normal playback.

```text
DEVELOPMENT / RESEARCH TIME
libaural
LLMs
learned specialists
large offline analyses
listening studies
simulation / search

        ↓ validate + distill + compile

SHIPPED OMNIPHONY
bounded local signal analysis
conservative scene inference
sample timeline
continuous trajectories
binaural / field / room DSP
headphone correction
output
```

Runtime workers may still exist for narrow engineering jobs such as preparing an HRTF/profile or other precomputable state off the audio thread. They are not an AI decision layer and do not continuously remix the music.

The audio callback never waits for intelligence. The intelligence has already been converted into the product.

---

## 3. Baseline mode must sound excellent by itself

Omniphony needs a stable conservative path that already earns being left enabled.

That baseline should be comparable in product role to conventional headphone virtualization / surround upmix systems, while preserving the fork's stronger binaural foundation and music-fidelity rules.

Conceptually:

```text
stereo PCM
→ bounded local auditory evidence
→ conservative persistent scene
→ binaural renderer
→ optional room / calibration
```

No external semantic/music model is required during playback.

Future hearing research improves the code that performs these decisions in later builds. It does not become a dependency the current song has to wait for.

---

## 4. Audio callback prohibitions

The steady-state realtime callback must not perform operations with unbounded or scheduler-dependent latency.

Forbidden in the final always-on path:

- blocking mutex/RwLock acquisition;
- filesystem access;
- network access;
- LLM or general AI inference;
- device enumeration;
- SOFA/profile parsing;
- graph construction;
- unbounded heap allocation/deallocation;
- thread creation;
- waiting for worker completion;
- blocking logging;
- arbitrary callback-count-based control laws.

Any current occurrence is technical debt to remove before always-on release.

Local bounded analysis that is explicitly part of the compiled DSP/scene algorithm is allowed only when its cost and latency are measured and bounded like every other realtime component.

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
- narrow preparation workers;
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

### HRTF/profile preparation fails

Keep the current working profile/state.

### Control message malformed

Reject it on the control plane; audio continues.

### Queue overload

Apply a declared coalescing/drop policy; never grow memory without bound.

### Optional room/field processor fails validation

Disable/revert that optional layer rather than corrupt direct sound.

### NaN/non-finite state

Contain it before it reaches the output bus; record a diagnostic outside the callback.

There is intentionally no normal-playback failure mode called "AI unavailable" or "libaural result arrived late". Those systems are not live dependencies of the released audio path.

---

## 9. Latency budget

Low latency is part of correctness for an always-on spatializer.

Track separately:

```text
host/device buffering
algorithmic lookahead
local analysis latency
FIR / convolution latency
resampling latency
control-state latency
```

No network/model round trip belongs in that budget.

If a future local analysis method cannot meet the realtime budget, it must be simplified, moved out of the critical path, precomputed where possible, or remain research until it can be distilled into something that can.

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
HRTF/profile preparation failure / restart
```

Record:

- glitches/xruns;
- max callback time;
- peak memory growth;
- allocations after warm-up;
- non-finite samples;
- clipping interventions;
- state publication failures;
- preparation-worker errors;
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

New libaural/AI capability enters Omniphony **between releases**, not as a live mixer sitting in the playback graph.

```text
research observation
→ hypothesis
→ controlled experiments
→ listening + objective validation
→ identify the smallest useful mechanism
→ compress into deterministic algorithm / policy / parameter / asset
→ compile into Omniphony
→ regression + fidelity + performance gates
→ ship
```

A useful research result might eventually become things such as:

```text
better stereo-evidence equations
better confidence thresholds
better grouping persistence
better bass/front-anchor policy
better broad/direct/field discrimination
better HRTF interpolation
better room behavior
better headphone compensation
```

The runtime sees the resulting mechanism, not the research conversation that produced it.

This is how the project compounds without making playback fragile:

```text
more research
→ stronger next release
not
more research
→ more live dependencies
```

---

## 15. The always-on acceptance sentence

> **Omniphony is ready to be treated as normal playback infrastructure when a listener can leave it enabled indefinitely and the only recurring reason to notice it is that turning it off makes the presentation worse.**
