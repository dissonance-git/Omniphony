# Platform boundary guardrail

This document does **not** define an active cross-platform roadmap.

The current product is the Windows Omniphony described in the root `README.md`:

```text
ordinary Windows audio
→ Omniphony realtime core
→ binaural headphones
```

The only purpose of this document during the current phase is to prevent Windows host plumbing from leaking into the renderer so deeply that it becomes expensive to change later.

Windows is the product now. Other operating systems are deferred until there is a Windows product worth porting.

---

## 1. Current rule

Use Windows aggressively.

Keep the renderer boundary clean where doing so is cheap and obvious.

Do **not** delay useful Windows listening work to design hypothetical macOS, Linux, Android or iOS products.

```text
NOW
Windows host
→ portable-enough engine boundary
→ real listening
→ product iteration

LATER, IF JUSTIFIED
another host
→ same proven engine boundary
```

Portability is a future option preserved by sane ownership, not a current feature target.

---

## 2. Core versus host

The useful architectural split remains:

```text
WINDOWS INPUT / DEVICE / SESSION LAYER
        ↓
============================================================
OMNIPHONY ENGINE

PCM / sample timeline
→ bounded evidence / scene state
→ presentation state
→ binaural / field / room rendering
→ calibrated stereo PCM

============================================================
        ↓
WINDOWS OUTPUT / DEVICE LAYER
```

The engine should not need to know whether Windows samples arrived through WASAPI, ASIO, a development file source, a loopback route, or another thin host adapter.

That is enough portability discipline for the current project.

---

## 3. What belongs in Windows host code

Examples:

- system/player capture;
- loopback APIs;
- virtual endpoints if eventually required;
- device enumeration;
- sample-format negotiation;
- shared/exclusive mode;
- hardware-buffer negotiation;
- ASIO device handling;
- session/device lifecycle;
- installer/service details;
- device changes and recovery;
- Windows-specific latency/glitch diagnostics.

These are first-class product engineering tasks even though they do not belong in renderer semantics.

---

## 4. What belongs in the engine

At minimum:

- one authoritative sample/time domain;
- stereo/scene evidence contracts;
- current bounded presentation policy;
- object/field/room semantics;
- HRTF/binaural rendering;
- source extent and motion trajectories;
- calibration application;
- deterministic fixtures and fidelity metrics;
- validated state publication;
- behavior that must sound the same regardless of host callback partitioning.

`libaural` is not a mandatory engine owner. If later research provides useful hearing state, Omniphony should consume a bounded optional projection without making the native product depend on the full research system.

---

## 5. Windows audio routes

The product should support the route that best serves ordinary Windows use without erasing useful specialist routes.

Target relationship:

```text
normal Windows route
→ native Windows system audio

optional specialist route
→ ASIO
```

The current listener already uses a FiiO ASIO path, so ASIO remains valuable for development and may remain valuable permanently.

Do not make ASIO mandatory for ordinary users.

Do not delete ASIO merely because a normal Windows route exists.

The renderer core behind both must remain the same.

---

## 6. Time law

A host callback is not an auditory coordinate system.

```text
40-sample callback
240-sample callback
960-sample callback
file-render block
ASIO buffer
Windows shared-mode packet
        ↓
same logical sample timeline
```

Gain, position, HRTF movement, scene transitions and room changes should be defined in sample/time coordinates rather than callback coordinates.

This law exists because audible behavior should not change merely because the host partitions the same continuous stream differently.

It is an engine-correctness rule, not permission to postpone Windows product work indefinitely while chasing theoretical invariance.

---

## 7. First Windows acceptance tests

The next host work should prove practical behavior on the real machine:

```text
48 kHz stereo PCM
→ Windows input route
→ existing Omniphony engine
→ Windows output route
→ FiiO K7 / Noire X
```

Test independently:

- clean start/stop;
- device enumeration;
- stable sustained playback;
- no obvious underruns/clicks;
- expected channel count/sample rate;
- bounded latency;
- device restart/recovery;
- enable/bypass behavior;
- coexistence with the incumbent HeSuVi route;
- no change to the protected renderer baseline merely because transport changed.

The first useful listening lane matters more than a complete host abstraction hierarchy.

---

## 8. Future second-platform rule

There is no current commitment to a second platform.

If one is later justified, begin with identical offline fixtures around the already-proven engine:

```text
same PCM
same state events
same config
same HRTF/calibration assets
        ↓
engine build / host A
engine build / host B
        ↓
compare timing, semantics and rendered output
```

Then implement native transport separately.

Do not create different artistic policies for different operating systems.

Do not revive old Linux/PipeWire or mobile plans merely because historical code/docs exist.

---

## 9. Current product priority

The root README owns roadmap priority.

At the current checkpoint:

```text
protect upstream Omniphony sound
→ build coexisting native Windows listening lane
→ compare against real HeSuVi incumbent
→ fix the next actual audible/product weakness
→ only later decide whether another platform deserves work
```

That is the portability policy.