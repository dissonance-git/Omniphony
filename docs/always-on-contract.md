# Always-on spatializer contract

Omniphony is intended to become normal playback infrastructure: something that can remain enabled for hours without demanding attention.

The surrounding research may be ambitious. The shipped audio behavior must be boring in the best sense: deterministic, bounded, recoverable and musically trustworthy.

> **Intelligence may run ahead of playback. Playback may never wait for intelligence.**

## Product identity

Omniphony is not a research dashboard, DAW, model host or realtime AI mixing service.

At the listener boundary:

```text
ordinary playback
→ Omniphony
→ headphones
```

The Current model must sound excellent and remain fully functional with no libaural process, neural model, network service or analysis cache present.

libaural and other learned systems may improve Omniphony in two legitimate ways:

```text
research
→ validated mechanism
→ deterministic Omniphony DSP
```

and, where the source is available ahead of playback:

```text
source
→ headless libaural AnalysisDsp
→ time-indexed bounded control state
→ Omniphony
```

The second path is permitted only when it preserves all failure and realtime laws below.

## Realtime core sovereignty

The hard realtime renderer is authoritative over continuity.

The audio callback must be able to execute using already-available bounded state only. It must never wait for:

- a neural model;
- an LLM;
- source separation;
- filesystem scanning;
- a cache miss;
- network access;
- another process to answer;
- an analysis worker to catch up.

A heavyweight analyzer may operate:

- before playback;
- faster than playback;
- with bounded lookahead;
- across a whole random-access track;
- in a separate process;
- as a headless library/service worker.

Its output is advisory control evidence. If the state is missing, stale, incompatible, late, invalid or low-confidence, playback falls back to Current model behavior.

```text
analysis available + valid
→ bounded optional modulation

analysis unavailable
→ Current model
```

There is no valid failure mode called "audio stops because AI is unavailable."

## Current model floor

Current model is the complete always-on baseline:

```text
protected stereo master
+ coherent foundation
+ bounded stereo evidence
+ derived support field
+ measured-HRTF spatial world
+ directional measured-HRTF early reflections
+ restrained late field
+ deterministic output safety
```

Future source-aware behavior must amend this floor, not replace it casually.

## Audio callback prohibitions

The steady-state realtime callback must not perform operations with unbounded or scheduler-dependent latency.

Forbidden:

- blocking mutex/RwLock acquisition;
- filesystem access;
- network access;
- LLM/general model inference;
- source-separation inference;
- device enumeration;
- SOFA/profile parsing;
- graph construction;
- unbounded heap allocation/deallocation;
- thread creation;
- waiting for worker completion;
- blocking logging.

Local bounded analysis is allowed only when its worst-case cost is measured and fits the realtime budget.

## Analysis-state handoff

If libaural-derived control enters a future build, the handoff must be deliberately small.

Preferred shape:

```text
heavy analysis state
        ↓ projection outside callback
immutable / lock-free bounded control frame
        ↓
audio callback reads current frame
```

Requirements:

- time-indexed state;
- versioned contract;
- source identity/provenance where relevant;
- finite values only;
- bounded dimensions;
- bounded gain/routing influence;
- explicit confidence/validity;
- smooth parameter transitions;
- deterministic fallback.

Separated stem waveforms do not enter the protected master merely because they were used to infer control state.

## Cache law

A cache is an acceleration mechanism, not the meaning of the effect.

Deleting analysis cache may cause analysis to be recomputed, but must not make ordinary audio playback impossible.

A stale cache must be rejected rather than silently applied to a different track/model/contract version.

## Allocation policy

Preferred lifecycle:

```text
control / preparation thread
→ construct / reserve / prewarm
→ validate
→ publish ready bounded state

realtime thread
→ reuse fixed/bounded storage
```

A first-use allocation hidden inside a musical transient is not acceptable merely because later callbacks are allocation-free.

## No host-shaped sound

Changing a legal callback partition must not redefine:

- gain slew;
- source motion;
- HRTF motion;
- bypass transition;
- room modulation;
- transient routing;
- analysis-state interpolation.

Required family:

```text
same PCM
+ same timed state
+ different legal callback partitions
→ equivalent intended audible world
```

Exact waveform null is required where the algorithm should mathematically be identical.

## Stream discontinuity law

A seek, new track, decoder reset or sample-rate restart must not inherit old stream history accidentally.

Reset stream-lifetime state as applicable:

- channel/ramp history;
- FIR history;
- ITD/fractional-delay history;
- crossover/filter states;
- reflection delay state;
- late-room stored energy;
- analysis-control interpolation state;
- source identity/timeline state.

Keep expensive immutable state only where still valid, such as HRTF data, tables and reserved storage.

## Failure containment

### Analysis unavailable or late

Use Current model. Do not block.

### Analyzer crashes

Audio continues. Supervisor/control plane may restart analysis separately if a future integration owns such a worker.

### Invalid/non-finite analysis

Reject the control frame before it reaches audio output.

### Queue overload

Use a declared bounded coalescing/drop policy. Never grow memory without bound.

### HRTF/profile preparation fails

Keep the last known-valid state or Current model.

### Optional room/source-aware mechanism fails validation

Disable/revert that optional layer rather than damage direct music.

## Latency budget

Track separately:

```text
host/device buffering
algorithmic lookahead
local realtime analysis
FIR/convolution latency
resampling latency
control-state interpolation latency
```

Offline/deep-lookahead libaural analysis does not belong in the audio callback latency budget because playback never waits for it.

## CPU budget

Average CPU is insufficient. Measure at least:

- median callback time;
- p95/p99;
- maximum observed callback time;
- deadline misses;
- reflection/room cost;
- stream reset;
- state transitions;
- device/sample-rate changes.

A feature that is cheap most of the time but occasionally overruns the device deadline is not lightweight.

## Fidelity floor

No stability optimization or intelligence layer may quietly make the renderer sound worse.

Every major change must preserve or improve:

- transient timing;
- bass timing/weight;
- kick impact;
- timbre;
- dynamics;
- center authority;
- vocal/instrument identity;
- groove/microtiming;
- important stereo relationships;
- absence of clicks/zipper noise;
- absence of pumping/spatial twitching;
- comfortable spectral balance.

Matched-loudness bypass remains a final sanity check.

## Long-duration soak tests

Before treating Omniphony as mature always-on infrastructure, soak tests should cover:

```text
hours of continuous playback
many track boundaries
seek / pause / resume
sample-rate changes
silence → loud transient
bass-heavy material
dense distorted guitars
sparse acoustic music
browser / desktop scheduling load
device interruptions
optional analysis present / absent / late / invalid
```

Record:

- glitches/xruns/underruns;
- maximum callback time;
- peak memory growth;
- allocations after warm-up;
- non-finite samples;
- clipping interventions;
- reset counts;
- analysis-frame rejection/fallback counts when applicable.

Memory and CPU should reach bounded steady state rather than creep over time.

## Research integration rule

A mechanism enters normal playback only after it earns a bounded contract.

```text
research observation
→ hypothesis
→ controlled experiment
→ objective + physical listening validation
→ smallest useful mechanism
→ bounded implementation or control projection
→ regression / performance / fidelity gates
→ Current model or optional validated intelligence
```

The project should prefer distilled deterministic mechanisms when they capture the useful effect. A richer libaural side channel is justified only when time-varying source understanding itself provides value that cannot be compressed into a fixed local rule.

## Always-on acceptance sentence

> **Omniphony is ready to be treated as normal playback infrastructure when it can remain enabled indefinitely, survive ordinary desktop stress without drawing attention to itself, and the recurring reason to notice it is that turning it off makes the presentation worse.**
