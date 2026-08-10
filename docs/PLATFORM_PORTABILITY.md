# Platform portability contract

Omniphony is a **cross-platform music-hearing / presentation / binaural system** whose first serious implementation and listening validation happens on Windows.

Windows is the current laboratory, not the product identity.

The eventual target includes:

- Windows;
- macOS;
- Linux;
- Android;
- iOS;
- possible plugin/embedded hosts where the same core contracts make sense.

The project must therefore avoid accidentally turning today's Windows plumbing into tomorrow's architecture.

---

## Core boundary

The intended mature split is:

```text
PLATFORM INPUT
system / app / player / file / stream
        ↓
============================================================
PORTABLE OMNIPHONY CORE

input audio timeline
→ libaural hearing state
→ music-aware presentation policy
→ persistent scene state
→ direct / broad / diffuse / room rendering
→ binaural output
→ listener + headphone calibration

============================================================
        ↓
PLATFORM OUTPUT
native device / host / plugin
```

Everything inside the lines should operate on platform-independent audio/time/state contracts unless measurement proves an OS-specific optimization is necessary.

---

## What belongs in platform adapters

Examples:

- system-wide capture;
- virtual endpoints/drivers;
- loopback APIs;
- audio-session policy;
- device enumeration;
- permissions;
- sample-format negotiation;
- hardware buffer negotiation;
- exclusive/shared mode;
- background-service lifecycle;
- mobile audio focus/session behavior;
- OS-specific low-latency APIs;
- driver signing / packaging;
- app installation/update integration.

These are essential product engineering, but they are **not auditory-scene semantics**.

---

## What must remain portable

At minimum:

- stereo/auditory evidence contracts;
- libaural state integration;
- scene entity semantics;
- music-aware presentation policy;
- sample-time trajectories;
- binaural/HRTF logic;
- broad-source rendering;
- diffuse musical-field rendering;
- room rendering;
- calibration layers;
- fidelity metrics;
- deterministic fixtures;
- confidence / uncertainty behavior;
- serialization of portable configuration/state where needed.

A host callback is merely one way of delivering samples to this machinery.

---

## Time law

The portable core owns a monotonic sample/time domain.

```text
WASAPI callback
CoreAudio callback
PipeWire callback
AAudio/Oboe callback
AVAudioEngine callback
file-render chunk
VST/AU host block

        ↓

same logical audio timeline
```

Therefore:

> **Changing host callback size must not change the auditory world or intended presentation trajectory.**

The implementation may process different block sizes, but gain, position, scene transitions, HRTF changes, room changes and other audible state must be defined in sample/time coordinates rather than callback coordinates.

This is already a known defect area in the current binaural path and remains an immediate test target.

---

## Capability negotiation instead of forks

Different platforms have different compute, latency and power budgets.

Do not create independent conceptual versions such as:

```text
Desktop Omniphony
Android Omniphony
iOS Omniphony
```

that gradually disagree about what the auditory scene means.

Prefer:

```text
same scene + presentation contract
        ↓
RendererCapabilityProfile
        ↓
implementation tier
```

Possible capability dimensions:

```text
max direct objects
max broad-field order
HRTF interpolation quality
room reflection order
late-field complexity
convolution partition size
analysis lookahead budget
model-provider availability
CPU budget
memory budget
power budget
latency target
```

A mobile renderer may use a cheaper numerical realization while targeting the same perceptual intent.

---

## Portability acceptance test

When a second platform is introduced, do not begin by judging whether the UI launches.

Begin with identical offline fixtures:

```text
same input PCM
same scene/state events
same configuration
same HRTF/calibration assets
        ↓
platform-independent core build A
platform-independent core build B
        ↓
compare rendered output + metrics
```

Expected result:

- exact equality where deterministic floating-point/toolchain behavior permits it;
- otherwise a documented numerical tolerance with perceptually irrelevant residual;
- identical scene/state interpretation;
- identical event timing;
- no platform-specific artistic policy.

Then test native realtime transport separately.

---

## Windows: current proving ground

Windows remains the first integration target because that is where current development and critical listening happen.

Current Windows-specific questions include:

- ordinary system/player capture;
- WASAPI shared/exclusive behavior;
- whether loopback is sufficient;
- whether a virtual endpoint is required for seamless product UX;
- output device changes;
- per-device calibration restoration;
- ASIO as an optional specialist path;
- installer/service behavior.

References such as Microsoft SysVAD and third-party virtual-audio drivers are **Windows adapter references only**.

They must not decide how Omniphony represents a guitar, field, room or scene trajectory.

---

## ASIO boundary

ASIO may remain useful for specialist interfaces and audiophile setups.

It must not remain a mandatory compile-time assumption for the product.

Target relationship:

```text
normal Windows path
→ native system audio route

optional specialist path
→ ASIO
```

The renderer core must be identical behind both.

---

## macOS / Linux / Android / iOS future mapping

The exact host choices remain experimental, but the conceptual boundary is fixed:

```text
macOS
CoreAudio / AudioUnit host shell
        ↓
portable core

Linux
PipeWire/ALSA-compatible host shell
        ↓
portable core

Android
AAudio/Oboe-style low-latency shell
        ↓
portable core

iOS
Core Audio / AVAudioSession-style shell
        ↓
portable core
```

These examples are host categories, not frozen implementation dependencies.

When each platform phase begins, research should compare the current native options rather than blindly transplant an old plan.

---

## Product UX law

The eventual user experience should be platform-native but conceptually boring:

```text
install
→ choose/detect headphones
→ calibrate or select profile
→ play music normally
```

The user should not need to understand the internal scene graph, renderer backend, driver topology or libaural model providers.

That simplicity must come from clean architecture rather than from hiding a tangled OS-specific renderer.

---

## Current rule

During the Windows validation phase:

> **Use Windows aggressively for testing, but treat every new Windows-specific dependency as guilty until it proves it belongs outside the portable core.**

The project succeeds when the hearing/presentation/renderer machinery can travel while only the thin audio plumbing changes.
