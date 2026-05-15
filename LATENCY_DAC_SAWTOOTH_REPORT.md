# DAC Latency Sawtooth Report

Date: 2026-05-14

## Summary

Current symptom in `clock_mode=dac`:

- output latency still shows an extremely regular sawtooth
- amplitude observed: about `16 ms`
- period observed: about `1 s`
- after several measurement fixes, `latency_downstream/path` stays at `0 ms`
- `latency_control/ctrl` and `latency_instant/raw` still move together
- an additional fixed offset of about `128 ms` was observed and was not removed by the latest attempted fixes

This means the remaining problem is not a vague stability issue. It behaves like a deterministic accounting or scheduling error tied to callback quanta, buffer domains, or a discrete periodic control action.

## Initial Context

The original concern was that a previous latency-stabilization effort appeared to have been lost. That was checked against Git history and was not the case.

Relevant facts:

- tag `v0.2.4` is commit `c3638de`
- the stabilization merge after that tag is commit `b16eb6d`
- the post-`v0.2.4` latency-measurement refactor is still present in `main`

Important commits still present:

- `9a2929c` `fix(audio): separate control and measured latency`
- `70eeaec` `refactor: align latency telemetry semantics`
- `ee11e2f` `fix: account for in-flight resampler latency`
- `214a4ee` `fix: publish post-recovery latency metrics`

Conclusion from that check:

- the previous latency-measurement refactor was not lost
- the current regression must come from later behavior or from a path that was not covered the way we assumed

## Backend Findings

The active input backend paths were checked.

Observed backend split:

- `clock_mode=upstream` uses `PwClientNode`
- `clock_mode=pipewire` uses `PwStream`
- `clock_mode=dac` uses `PwStream`

This matters because:

- `upstream` does not exercise the same scheduling path as `dac`
- the current issue reproduces in `dac`
- `pipewire` was also problematic during the investigation, but the focus later narrowed to `dac`

Runtime visibility was added so Studio can expose the effective PipeWire backend and clock strategy instead of only the requested high-level clock mode.

## What Was Tried

### 1. PipeWire output latency display smoothing

Files changed:

- `omniphony-renderer/audio_output/src/adaptive_runtime.rs`
- `omniphony-renderer/audio_output/src/pipewire.rs`

Intent:

- harden `low-recover/settling`
- separate displayed latency from control latency
- reduce false oscillation caused by the displayed metric

Result:

- did not remove the deterministic sawtooth
- later evidence showed the remaining problem was not only display smoothing

### 2. Check whether the old latency refactor still exists

Intent:

- verify whether the previously merged latency-measurement overhaul had been lost

Result:

- confirmed still present in Git history and current code
- ruled out “lost merge” as the explanation

### 3. Remove `PwStream` callback accumulation on encoded input

Files changed:

- `omniphony-renderer/src/cli/decode/live_input.rs`
- `omniphony-renderer/audio_input/src/pipewire.rs`

Intent:

- reduce artificial burstiness by changing `PW_STREAM_ACCUMULATE_CALLBACKS` from `4` to `1`
- avoid re-bursting IEC61937 input when the parser already buffers fragments internally

Result:

- plausible cleanup for capture cadence
- did not remove the DAC sawtooth

### 4. Expose effective PipeWire backend and clock strategy to runtime/UI

Files changed:

- `omniphony-renderer/audio_input/src/control.rs`
- `omniphony-renderer/src/cli/decode/live_input.rs`
- `omniphony-renderer/runtime_control/src/osc.rs`
- `omniphony-renderer/runtime_control/src/snapshot.rs`
- `omniphony-studio/src/state.js`
- `omniphony-studio/src/init.js`
- `omniphony-studio/src/controls/input.js`

Intent:

- distinguish requested mode from real backend path
- avoid diagnosing the wrong runtime path

Result:

- useful for diagnosis
- not itself a fix for the sawtooth

### 5. PipeWire graph-driven recabling attempt

File changed temporarily:

- `omniphony-renderer/audio_input/src/pipewire.rs`

Intent:

- make `clock_mode=pipewire` stop using the same DRIVER scheduling path as `dac`

Result:

- broke `pipewire` badly, including extremely slow video playback
- reverted to the previous DRIVER behavior
- useful only insofar as it showed that this path was not viable in current form

### 6. Reduce idle polling delay in direct-trigger DAC capture loop

Files changed:

- `omniphony-renderer/audio_input/src/pipewire.rs`
- `omniphony-renderer/src/cli/decode/live_input.rs`

Intent:

- remove up to `20 ms` of mainloop sleep before draining pending output-driven triggers
- avoid scheduling jitter from coarse polling in DAC direct-trigger mode

Result:

- reasonable correction for responsiveness
- did not remove the regular sawtooth

### 7. Slow and deadband the `output_rate_adjust -> input DRIVER interval` feedback

File changed:

- `omniphony-renderer/audio_input/src/pipewire.rs`

Intent:

- avoid coupling two fast control loops
- keep long-term drift correction but suppress short-term reinjection of output servo motion into input cadence

Result:

- did not eliminate the current deterministic DAC sawtooth

### 8. Revisit output latency semantics: make `raw` actually raw

File changed:

- `omniphony-renderer/audio_output/src/pipewire.rs`

Intent:

- remove UI-facing smoothing from `latency_instant/raw`
- sample PipeWire graph delay more often to avoid a fake `~1 s` telemetry alias

Result:

- `latency_downstream/path` ended up observed as `0`
- `ctrl` and `raw` remained equal
- therefore the remaining sawtooth is not coming from downstream graph delay

### 9. Separate control latency from midpoint-of-callback measurement

Files changed:

- `omniphony-renderer/audio_output/src/adaptive_runtime.rs`
- `omniphony-renderer/audio_output/src/pipewire.rs`
- `omniphony-renderer/audio_output/src/asio.rs`

Intent:

- stop using `callback_input_domain_samples / 2` as part of the control quantity
- keep midpoint compensation only in the measured quantity

Result:

- after this change, `ctrl` and `raw` still moved together in the user-observed DAC case
- a fixed `128 ms` offset also remained
- therefore either:
  - the midpoint correction was not the dominant cause in this path, or
  - another callback-size or domain error still dominates the signal

### 10. Treat PipeWire output callback size as requested quantum instead of mapped capacity

File changed:

- `omniphony-renderer/audio_output/src/pipewire.rs`

Intent:

- stop using `slice.len()` as if it were the real callback block size
- use `pw_stream_get_time().size` instead

Reasoning:

- mapped buffer capacity can be much larger than the active cycle size
- that would corrupt latency accounting and DAC trigger ratios

Result:

- user reported no change

### 11. Convert `pw_time.size` from interleaved samples to frames

File changed:

- `omniphony-renderer/audio_output/src/pipewire.rs`

Intent:

- compensate for the fact that PipeWire reports interleaved raw-audio size in samples, not frames
- with `8` channels, this kind of mistake naturally creates an apparent `8x` inflation, matching the observed `16 ms -> 128 ms` clue

Result:

- user reported no change
- this implies the `128 ms` offset is not explained solely by the `pw_time.size` interpretation in that callback path

## What We Know Now

Based on the latest user observations:

- `latency_downstream/path = 0`
- `latency_control/ctrl` and `latency_instant/raw` are both still sawtoothing
- the sawtooth remains highly regular
- sawtooth amplitude is exactly about `16 ms`
- an additional offset of about `128 ms` is still present

These observations strongly suggest:

- the issue is inside the control-latency accounting or the real controlled buffer level
- the issue is not primarily in downstream PipeWire graph delay
- the issue is likely quantized by a callback/block size or a periodic trigger cadence
- the problem remains deterministic enough that it should be debuggable from exact sample-domain accounting

## Why The Problem Is Still Open

Several plausible causes were eliminated or weakened:

- lost merge / lost stabilization work: ruled out
- downstream graph latency wobble: not supported by current `path = 0`
- simple UI smoothing artifact: not supported anymore
- coarse DAC trigger polling: insufficient to explain the remaining exact periodicity
- straightforward `pw_time.size` capacity-vs-quantum bug in the tested callback path: attempted fix showed no user-visible change

The remaining likely categories are:

1. the control buffer itself is genuinely oscillating with a callback-sized amplitude
2. another discrete quantity is still being interpreted in the wrong domain
3. DAC direct-trigger scheduling is periodically injecting one exact extra/missing quantum
4. a projected/post-recovery control metric is still being republished as if it were the steady-state metric

## Recommended Next Investigation Steps

The next pass should be measurement-first and should avoid more speculative “stability” changes until the exact oscillating quantity is identified.

Recommended next steps:

1. Log exact sample-domain quantities at fixed cadence in DAC mode:
   - ring buffer length
   - output FIFO input-domain samples
   - pending resampler input samples
   - callback input-domain samples
   - control available
   - projected control available
   - direct-trigger pending counter
   - observed capture quantum
   - output callback quantum

2. Log these with exact integer sample counts, not only milliseconds.

3. Correlate one full sawtooth period and verify which integer quantity moves by the amount corresponding to `16 ms`.

4. Check whether the `128 ms` offset equals:
   - `8 ×` a `16 ms` quantum
   - one PipeWire mapped block
   - one internal buffer target unit
   - one resampler chunk-domain conversion mistake

5. Verify whether the DAC direct-trigger loop fires an extra or missing trigger at roughly one-second cadence.

6. Confirm whether the sawtooth is in the true ring buffer level or only in the computed `control_available`.

## Validation Performed

The code changes above were repeatedly validated with:

- `cargo fmt --manifest-path /home/user/dev/spatial-renderer/Omniphony/omniphony-renderer/Cargo.toml`
- `cargo check --manifest-path /home/user/dev/spatial-renderer/Omniphony/omniphony-renderer/Cargo.toml -p omniphony-renderer`

This confirms the attempted fixes build, but runtime behavior is still incorrect.

## Current Status

Status: unresolved

The problem is currently best described as:

- a deterministic DAC-mode latency sawtooth
- approximately `16 ms` amplitude
- approximately `1 s` period
- plus a persistent fixed offset around `128 ms`
- with `ctrl` and `raw` still moving together
- and no downstream/path contribution visible
