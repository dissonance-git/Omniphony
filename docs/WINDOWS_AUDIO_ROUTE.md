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

Windows work must not silently change:

- HRTF family;
- early reflections;
- late room state;
- gain policy;
- scene logic;
- bass behavior;
- renderer defaults that define the protected reference.

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

The native route must coexist with this until it wins.

Do not require uninstalling or breaking the incumbent to test a transport milestone.

---

## 3. Current native progress

The Windows-native work already has two intentionally small layers.

### `windows_host`

`omniphony-renderer/windows_host` is a Windows-only transport probe.

It currently proves:

1. ordinary Windows output-device discovery through CPAL's default Windows host;
2. compilation without enabling CPAL's optional ASIO feature;
3. modern WASAPI application-loopback activation in a self-excluding diagnostic mode.

The probe explicitly says renderer integration is a later step.

### `realtime_ffi`

`omniphony-renderer/realtime_ffi` is the narrow PCM seam between native Windows transport and the protected Omniphony renderer.

Current ABI properties:

```text
interleaved f32 PCM
sample rate + channel count at create time
bounded process callback
in-place or out-of-place processing
explicit reset
C ABI / published header
```

The first implementation is deliberately **bit-exact identity**.

That is useful, not trivial: it gives the host boundary a deterministic oracle before real Omniphony DSP is connected behind it.

```text
Windows transport
→ realtime_ffi identity
→ exact same PCM
```

Only after that seam is stable should it become:

```text
Windows transport
→ realtime_ffi
→ protected Omniphony renderer
→ binaural PCM
```

The realtime ABI has CI/package coverage from the August 2026 host-work batch.

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

Do not promote it to the final system-wide route unless another routing mechanism guarantees the dry path is not simultaneously audible.

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
- closely matches the desired set-and-forget product experience;
- resembles the integration class that made Equalizer APO / HeSuVi practical, while Omniphony would own the renderer rather than only an HRIR convolution stage.

Why it is not automatically the winner:

- modern APO deployment is Windows-driver/component territory;
- endpoint association, installation and signing must be handled correctly;
- an APO executes inside a sensitive realtime audio environment;
- crash containment matters;
- the realtime callback cannot host blocking I/O, general model inference, filesystem work, or heavyweight analysis;
- the protected renderer may be too large to embed naively without a carefully bounded realtime projection.

If this route graduates, the split should be roughly:

```text
CONTROL PROCESS
UI / settings / diagnostics
profile construction
optional future libaural/model work
        ↓ bounded validated state

REALTIME APO
small deterministic host seam
protected Omniphony realtime projection
        ↓
headphones
```

`libaural` is optional future control evidence, not an APO dependency.

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
- easier isolation for richer control/diagnostic work;
- `realtime_ffi` already provides the beginning of the process boundary.

Costs:

- requires a virtual endpoint/driver solution;
- WDK, installation and signing complexity;
- adds a visible audio endpoint;
- more buffering/clock-domain opportunities;
- default-device switching/recovery become product responsibilities.

Microsoft SysVAD is a reference architecture for this class of work, not code to transplant wholesale into Omniphony.

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

Do not delete ASIO just because a normal route exists.

Do not force ASIO on ordinary users because it works in the current incumbent.

---

# 8. Decision gates

Do not choose APO or virtual endpoint because one diagram looks cleaner.

A route graduates only if it proves:

1. **single-path playback** — no audible/measurable dry + processed duplication;
2. **baseline preservation** — same renderer behavior for the same PCM/state input within declared numerical tolerance;
3. **normal app coverage** — ordinary shared-mode Windows apps work without per-player rituals;
4. **coexistence** — development/testing does not require destroying the current HeSuVi route;
5. **device behavior** — output changes, disappearance and recovery are deterministic;
6. **latency** — low enough for ordinary music/video and secondary gaming use without fragile tuning;
7. **glitch safety** — underrun, restart, sleep/wake and format changes fail safely;
8. **installer reality** — clean install/remove/update is reproducible;
9. **realtime separation** — UI/analysis/optional model work cannot block audio;
10. **ASIO independence** — normal Windows build does not require Steinberg SDK;
11. **reversibility** — disabling/uninstalling Omniphony restores the normal route cleanly;
12. **A/B usability** — the listener can compare Omniphony against the incumbent without rebuilding the audio environment each time.

---

# 9. Transport acceptance ladder

Keep implementation incremental and attributable.

### T0 · Host probe — EXISTS

```text
Windows
→ enumerate normal output devices
→ optional self-excluding loopback activation probe
```

No renderer DSP involved.

### T1 · Realtime identity seam — EXISTS

```text
PCM
→ realtime_ffi
→ bit-exact PCM
```

This is the contract oracle.

### T2 · Native output smoke path

```text
test PCM / fixture
→ realtime_ffi identity
→ normal Windows output
```

Prove stable device playback without Omniphony DSP first.

### T3 · Protected renderer behind seam

```text
controlled PCM / known scene
→ realtime_ffi
→ protected Omniphony renderer
→ Windows output
```

Compare against offline/reference rendering to ensure transport did not change the engine.

### T4 · Single-path ordinary app route

Prototype the smallest practical APO and/or virtual-endpoint boundary.

Do not accept loopback replay as success.

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

At this checkpoint:

```text
1. keep upstream-demo perceptual control frozen
2. keep windows_host + realtime_ffi building in CI
3. prove normal Windows output through the identity seam
4. connect protected Omniphony renderer behind realtime_ffi
5. verify realtime output matches controlled offline renderer behavior
6. prototype the smallest viable single-path APO boundary
7. compare against a minimal virtual-endpoint route if APO constraints are poor
8. choose the transport from evidence
9. establish incumbent ↔ Omniphony A/B
10. only then pull forward new renderer/adaptive DSP that addresses an actual audible weakness
```

Research can inform any step when a concrete missing capability appears.

Research does not replace this order merely because a more sophisticated architecture is imaginable.