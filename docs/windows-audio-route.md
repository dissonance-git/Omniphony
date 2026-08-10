# Windows audio route

This document owns the **Windows transport decision**, not the overall project plan. The root `README.md` owns product intent and priority.

The immediate requirement is stronger than “capture system audio”:

> **Ordinary Windows playback must reach the listener exactly once, through Omniphony, without a simultaneous dry copy, while the protected Omniphony renderer remains unchanged by transport work.**

This is the practical bridge from the current HeSuVi/VB-Audio chain to a native product.

---

## 1. Protected signal path

The transport layer is not allowed to retune the sound in order to make integration easier.

Primary renderer control:

```text
omniphony-renderer/assets/binaural-baselines/upstream-demo-reference.yaml
```

Windows work must not silently change HRTF family, early reflections, room state, gain policy, scene logic, bass behavior, or renderer defaults that define the protected reference.

Transport earns the right to carry Omniphony. It does not redefine Omniphony.

---

## 2. Current incumbent context

The existing daily path remains available during development:

```text
foobar DSP
→ 5.1-side upmix
→ VB-Audio multichannel / ASIO Bridge
→ HeSuVi / DTS Virtual:X
→ FiiO ASIO
→ K7
→ Noire X
```

Do not require uninstalling or breaking the incumbent to test a transport milestone.

ASIO remains a useful specialist/reference route because it serves the current hardware/HeSuVi setup. It is not the default product requirement.

---

## 3. Current native progress

### `windows_host`

`omniphony-renderer/windows_host` is the thin Windows-native product/transport prototype.

It currently provides:

- ordinary Windows output-device discovery through CPAL's default Windows host;
- compilation without CPAL's optional ASIO feature;
- self-excluding WASAPI application-loopback activation as a diagnostic;
- `--smoke-output`, which sends a low-level test tone through `realtime_ffi` identity to the default endpoint;
- `--reference-demo`, which renders the bundled 7.1.4 reference through the protected Omniphony binaural engine and plays the stereo result over native WASAPI;
- `--render-reference-only`, which lets CI validate the packaged bridge/config/layout/renderer without requiring a physical audio endpoint.

The internal P0 Windows build compiled and packaged successfully on 2026-08-10.

### `realtime_ffi`

`omniphony-renderer/realtime_ffi` is the narrow PCM seam between native Windows transport and the eventual persistent realtime Omniphony renderer.

Current ABI:

```text
interleaved f32 PCM
sample rate + channel count at create time
bounded process callback
in-place or out-of-place processing
explicit reset
C ABI / published header
```

Its first implementation is deliberately **bit-exact identity**.

That gives the host boundary a deterministic oracle:

```text
Windows transport
→ realtime_ffi identity
→ exact same PCM
```

The P0 reference demo currently renders the controlled Omniphony scene before playback and then crosses this identity seam. The next renderer integration step is to make the protected renderer the persistent realtime processor behind the same host boundary for ordinary PCM.

---

## 4. Why loopback capture is diagnostic only

The current process-loopback probe uses self-exclusion so Omniphony does not immediately recapture itself.

That proves a useful Windows primitive, but loopback is a **copy** of the system mix, not an intercept.

Windows still sends the original dry audio to the active render endpoint.

Therefore replaying processed loopback to the same headphones would produce:

```text
dry system audio
+
processed Omniphony copy
```

which is unacceptable.

So:

```text
system/process loopback capture
!=
transparent HeSuVi replacement
```

Keep loopback for diagnostics, experiments, analysis, or development capture where its semantics are useful.

---

# 5. Candidate A: endpoint/system-effect APO

Conceptual route:

```text
application audio
    ↓
Windows shared audio engine
    ↓
Omniphony endpoint/system-effect APO
    ↓
physical headphone endpoint
```

Why it is attractive:

- in-place single-path processing;
- no second dry copy to suppress;
- ordinary apps can use the normal shared Windows endpoint;
- closely matches the desired set-and-forget product experience.

Why it is not automatically the winner:

- modern APO deployment is Windows-driver/component territory;
- endpoint association, installation, and signing must be handled correctly;
- an APO executes inside a sensitive realtime audio environment;
- crash containment matters;
- blocking I/O, general model inference, filesystem work, and heavyweight analysis do not belong in the realtime process path;
- the full renderer may require a carefully bounded realtime projection.

If this route graduates, keep control/UI/model/profile construction outside the realtime audio path and publish only bounded validated state inward.

---

# 6. Candidate B: virtual render endpoint

Conceptual route:

```text
application audio
    ↓
Omniphony virtual render endpoint
    ↓
Omniphony host process
    ↓
realtime_ffi
    ↓
protected Omniphony renderer
    ↓
WASAPI physical headphones
```

Why it remains viable:

- explicit single-path routing;
- keeps the full renderer out of the Windows audio-engine process;
- maps naturally onto the Rust host/core boundary already being built;
- easier isolation for richer control/diagnostics.

Costs:

- virtual endpoint/driver solution;
- WDK, installation, and signing complexity;
- another visible audio endpoint;
- more buffering/clock-domain opportunities;
- default-device switching/recovery become product responsibilities.

Microsoft SysVAD is a reference architecture for this class of work, not code to transplant wholesale.

---

# 7. Candidate C: ASIO

ASIO remains valuable as a specialist route.

The current listener already uses:

```text
VB-Audio ASIO Bridge
→ FiiO ASIO Driver
→ FiiO K7
```

So ASIO is useful for development comparison and may remain valuable permanently.

But ASIO alone does not solve normal system-wide interception, and the ordinary Windows product should not require the separately licensed Steinberg SDK or specialist drivers.

Correct relationship:

```text
normal Windows route
→ native shared/system integration

specialist route
→ ASIO where useful
```

Do not delete ASIO just because a normal route exists. Do not force ASIO on ordinary users because it works in the incumbent.

---

# 8. Decision gates

A route graduates only if it proves:

1. **single-path playback** — no dry + processed duplication;
2. **baseline preservation** — same renderer semantics for the same PCM/state input within declared tolerance;
3. **normal app coverage** — ordinary shared-mode Windows apps work without per-player rituals;
4. **coexistence** — development/testing does not require destroying the current HeSuVi route;
5. **device behavior** — output changes, disappearance, and recovery are deterministic;
6. **latency** — suitable for music/video and secondary gaming use;
7. **glitch safety** — underrun, restart, sleep/wake, and format changes fail safely;
8. **installer reality** — clean install/remove/update is reproducible;
9. **realtime separation** — UI/analysis/optional model work cannot block audio;
10. **ASIO independence** — normal Windows build does not require Steinberg SDK;
11. **reversibility** — disabling/uninstalling Omniphony restores normal routing cleanly;
12. **A/B usability** — the listener can compare Omniphony with the incumbent without rebuilding the audio environment.

---

# 9. Transport acceptance ladder

### T0 · Host probe — EXISTS

```text
Windows
→ enumerate normal output devices
→ optional self-excluding loopback activation probe
```

### T1 · Realtime identity seam — EXISTS

```text
PCM
→ realtime_ffi
→ bit-exact PCM
```

### T2 · Native output smoke path — EXISTS / COMPILED

```text
test PCM
→ realtime_ffi identity
→ normal Windows output
```

Physical endpoint listening remains the final validation.

### T3a · Controlled protected renderer → Windows output — EXISTS / COMPILED

```text
bundled known 7.1.4 scene
→ protected Omniphony renderer
→ realtime identity seam
→ Windows output
```

CI also validates the packaged protected render without a physical endpoint.

### T3b · Persistent realtime renderer behind the host seam — NEXT

```text
continuous ordinary PCM/state
→ realtime_ffi / host boundary
→ protected Omniphony renderer
→ Windows output
```

Prove callback/stream behavior matches controlled reference semantics.

### T4 · Single-path ordinary app route

Prototype the smallest practical supported APO and/or virtual-endpoint boundary. Do not accept loopback replay as success.

### T5 · Incumbent coexistence A/B

```text
current HeSuVi chain
↔
Omniphony native route
```

Matched loudness and fast enough switching to make listening useful.

### T6 · Product-route decision

Choose APO, virtual endpoint, hybrid, or another route from measured reliability/latency/installability and real use.

---

# 10. Current order of work

```text
1. physically test P0 native smoke/reference playback
2. keep upstream-demo perceptual control frozen
3. make protected Omniphony persistent behind the host seam
4. verify realtime output matches controlled renderer semantics
5. add the simplest ordinary-stereo music path without experimental DSP
6. establish incumbent ↔ Omniphony A/B
7. prototype the smallest viable single-path system boundary
8. compare APO / virtual-endpoint approaches only as evidence requires
9. harden device/recovery/latency behavior
10. pull forward new renderer/adaptive DSP only for an actual audible weakness
```

Research can inform a step when a concrete capability is missing. Research does not replace this order merely because a more sophisticated architecture is imaginable.
