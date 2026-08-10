# Windows audio route

Windows is the first live product target, but the renderer must stay independent of Windows transport details.

The immediate product requirement is stricter than “capture system audio”:

> **Ordinary shared-mode playback must reach the listener exactly once, through Omniphony, without a simultaneous dry copy.**

That requirement rules out treating WASAPI loopback capture by itself as the final HeSuVi replacement.

## Protected signal path

The sound-quality reference remains the upstream-demo-style binaural control in:

```text
omniphony-renderer/assets/binaural-baselines/upstream-demo-reference.yaml
```

Windows work is transport work first. It must not silently alter HRTF, early-reflection, scene, gain, room or other renderer behavior while solving capture/output.

## Current native probe

`omniphony-renderer/windows_host` is a deliberately thin Windows-only host seed.

It currently proves two independent primitives:

1. ordinary Windows output-device discovery through CPAL's default Windows host, compiled without the optional ASIO feature;
2. modern WASAPI application-loopback activation in a self-excluding diagnostic mode.

The second primitive is useful for diagnostics and transport experiments, but it is not an intercept. Windows still sends the original mix to its render endpoint, so replaying the captured copy to the same headphones would create dry + processed playback.

Therefore:

```text
system loopback capture
!=
transparent system-wide replacement
```

## Candidate A: endpoint APO

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

- processes the stream in-place, so there is no second dry copy to suppress;
- matches the “install, choose headphones, play normally” product goal;
- keeps the normal Windows shared-mode route as the default;
- resembles the class of integration already proven useful by Equalizer-APO/HeSuVi-style systems, while allowing Omniphony to own its renderer rather than merely convolving an HRIR.

Why it is not yet chosen:

- a modern APO is an in-process realtime COM component;
- Windows 11 deployment uses the AudioProcessingObject driver-package/component model;
- an APO must be associated with the relevant audio device rather than registered once globally for every unrelated driver;
- the realtime callback cannot host heavy analysis, model inference, blocking I/O or arbitrary control-plane work;
- installation, signing, endpoint association, crash containment and device-change behavior must be proven on the actual target machine.

The intended split, if this route graduates, is:

```text
CONTROL / HEARING PROCESS
UI
libaural later
profile/calibration construction
slow musical context
        ↓ bounded immutable state

REALTIME APO
small deterministic state
protected Omniphony render projection
no blocking/model work
        ↓
headphones
```

## Candidate B: virtual render endpoint

Conceptual route:

```text
application audio
    ↓
Omniphony virtual render endpoint
    ↓
Omniphony host process
    ↓
portable renderer core
    ↓
WASAPI physical headphones
```

Why it remains viable:

- cleanly keeps the full renderer out of the Windows audio-engine process;
- the existing Rust host/core boundary maps naturally onto it;
- easier isolation for richer control-plane behavior;
- explicit routing guarantees the listener receives only the processed copy.

Costs:

- requires a virtual audio driver/endpoint, with WDK, installation and signing work;
- exposes another audio device to Windows and applications;
- adds transport/buffering opportunities that must be measured;
- seamless default-device switching and recovery become product responsibilities.

Microsoft SysVAD is a reference architecture, not something to copy wholesale into the renderer.

## Candidate C: ASIO

ASIO remains optional specialist output for interfaces that benefit from it.

It is not the normal product route because it can require specialist drivers and the separately licensed Steinberg SDK at build time. It also does not solve ordinary system-wide interception by itself.

## Decision gates

Do not choose the Windows route because one architecture is cleaner on paper. Choose it by bounded product evidence.

A route can graduate only if it proves:

1. **single-path playback**: no audible or measurable dry + processed double path;
2. **baseline preservation**: identical portable-renderer behavior for the same PCM/state input within documented numerical tolerance;
3. **normal app coverage**: ordinary shared-mode Windows applications work without per-player configuration;
4. **device behavior**: headphone/DAC changes, disappearance and recovery are deterministic;
5. **latency**: measured end-to-end latency is low enough for normal music/video use without fragile tuning;
6. **glitch safety**: underrun, restart, sleep/wake and format changes fail safely rather than blasting or hanging;
7. **installer reality**: a clean Windows machine can install/remove/update the route reproducibly;
8. **control separation**: UI/libaural/control work cannot block the realtime audio callback;
9. **ASIO independence**: the ordinary Windows build does not require the ASIO SDK;
10. **reversibility**: disabling/uninstalling Omniphony restores the normal endpoint cleanly.

## Current order of work

```text
1. keep upstream-demo perceptual control frozen
2. keep Windows WASAPI probe compiling in CI
3. package a runnable windows_host.exe diagnostic artifact
4. prototype the smallest viable APO boundary
5. compare APO deployment/latency/reliability with a minimal virtual-endpoint route
6. choose transport
7. feed ordinary stereo PCM through the protected Omniphony binaural path
8. listen and measure before enabling any new DSP
9. only then graduate optional libaural-informed improvements
```

The transport layer earns the right to carry Omniphony. It does not get to redefine what Omniphony sounds like.
