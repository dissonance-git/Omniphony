# Building Omniphony on Windows

Omniphony is Windows-first, but the fork is currently between two host-audio architectures:

```text
CURRENT INHERITED HOST
CPAL + ASIO-oriented Windows output

TARGET PRODUCT HOST
normal Windows system/player stereo capture
+ normal system output route
+ ASIO optional for specialist low-latency hardware
```

The **renderer engine itself** does not require ASIO and is validated separately in CI.

This distinction is intentional. Do not make a proprietary SDK a prerequisite for compiling or testing the DSP core.

---

## 1. Toolchain

Core development requires:

- Windows 10/11 x64;
- Visual Studio 2022 Build Tools / MSVC C++ workload;
- Rust `1.87.0`;
- Git.

Install the Rust MSVC toolchain normally:

```powershell
rustup toolchain install 1.87.0 --profile minimal --component rustfmt
rustup override set 1.87.0
```

From `omniphony-renderer/`:

```powershell
cargo fmt --all -- --check
cargo test -p dsp_fixtures
cargo test -p renderer
```

These commands are the important DSP-core baseline.

---

## 2. Headless renderer engine

The clean CI packaging boundary is currently:

```text
orender_engine
orender_ffi
reference_bridge
```

`orender_engine` deliberately excludes host audio.

Build the engine/FFI boundary with:

```powershell
cargo test -p orender_engine -p orender_ffi -p reference_bridge
cargo build --profile release-deploy -p orender_ffi -p reference_bridge
```

Expected Windows release artifacts include:

```text
target\release-deploy\orender.dll
orender_ffi\include\orender.h
```

and, when built as a dynamic reference bridge:

```text
target\release-deploy\reference_bridge.dll
```

This is what the GitHub workflow currently packages.

---

## 3. Why CI does not build the full listening executable yet

The inherited Windows `audio_output` crate currently declares CPAL with its `asio` feature unconditionally.

That makes the full host executable depend on Steinberg ASIO SDK source/header material.

The ASIO SDK has separate licensing terms and is not bundled with this fork.

Therefore:

```text
renderer engine CI
→ must stay independent of ASIO

full host/listening executable CI
→ deferred until Windows audio-output feature gating is cleaned up
```

This is not a renderer limitation. It is a host-integration boundary that the fork is intentionally changing.

---

## 4. Current ASIO development path

If you explicitly want to build the inherited ASIO host locally, obtain the ASIO SDK under Steinberg's terms and point CPAL at it using the environment expected by `asio-sys`/CPAL.

Historically this project used:

```powershell
$env:CPAL_ASIO_DIR = 'C:\path\to\asio_sdk'
```

Then build the relevant host/executable from a Visual Studio developer environment.

Treat this as a **specialist development route**, not the baseline contributor setup.

Do not commit the ASIO SDK into this repository.

---

## 5. Target Windows audio architecture

The product requirement is broader than “support ASIO.”

Normal users should be able to:

```text
install Omniphony
→ choose/calibrate headphones
→ play audio from normal Windows applications
→ hear processed stereo output
```

without routing every player through a proprietary musician-oriented driver API.

The target host layer should therefore separate:

```text
SYSTEM / NORMAL ROUTE
Windows system/player capture
normal endpoint/device lifecycle
per-device profile persistence
robust recovery / diagnostics

SPECIALIST ROUTE
ASIO where desired
low-latency DAC/interface use
explicit user selection
```

Microsoft's open Windows Audio repositories are useful references for endpoint diagnostics, AudioDG/APO behavior, ETW/WPA tracing and low-latency driver considerations.

ASIO can remain valuable without becoming the architecture's center of gravity.

---

## 6. Required host refactor

The next Windows-host cleanup should make the dependency graph honest.

Desired Cargo shape:

```text
cpal default Windows backend
→ normal Windows output build

feature `asio`
→ enables cpal/asio explicitly
→ requires user-provided ASIO SDK
```

The current code instead assumes ASIO inside `cpal_output.rs` and labels the Windows backend as ASIO.

That code should be generalized before changing the Cargo feature alone.

Required coordinated changes:

1. make Windows default-host/WASAPI behavior compile without `cpal/asio`;
2. gate `HostId::Asio` behind the `asio` feature;
3. make backend naming/reporting truthful (`WASAPI` vs `ASIO`);
4. forward the root `asio` feature into `audio_output/asio`;
5. keep shared buffering/resampling logic common;
6. add Windows tests/builds for both default-system and optional ASIO configurations where licensing infrastructure permits.

Do not merely remove the Cargo feature while leaving `HostId::Asio` hard-coded in source.

---

## 7. SAF / SPARTA / VBAP

`renderer` retains mature speaker/VBAP machinery because known speaker geometry is valuable calibration truth and some shared scene logic still depends on it.

The optional `saf_vbap` feature is **not required for the normal headphone product**.

If you are specifically researching SAF-backed VBAP, refer to upstream `mgth/Omniphony` history and the Spatial Audio Framework documentation for its C/OpenBLAS build requirements.

This fork no longer keeps a multi-page SAF/OpenBLAS build recipe in the primary Windows guide because that is not the normal product path.

Rules:

```text
native/pure-Rust renderer path
→ baseline

SAF-backed VBAP
→ optional experiment/reference
```

Keep its licensing and external dependency requirements explicit when used.

---

## 8. Headphone calibration tooling

SOFA/HRTF import and future headphone calibration belong primarily to the control/offline side of the product.

See:

- [`BINAURAL.md`](BINAURAL.md)
- [`../docs/HEADPHONE_CALIBRATION.md`](../docs/HEADPHONE_CALIBRATION.md)

Do not perform SOFA parsing, large filter construction, profile optimization, or other expensive calibration operations on the realtime audio thread.

---

## 9. Windows diagnostics

A mature Windows product should make audio failures observable.

Useful future evidence includes:

- endpoint/device identity;
- requested and actual sample rate/channel format;
- buffer/latency state;
- underrun/overrun counts;
- renderer block-time statistics;
- profile/HRTF identity;
- Windows Audio/AudioDG diagnostic traces when platform failures require them.

Do not confuse tracing overhead with normal audio quality. Heavy tracing can itself cause glitches.

---

## 10. CI contract

The authoritative workflow is:

```text
../.github/workflows/windows-renderer.yml
```

It currently proves:

```text
portable renderer core
Windows renderer core
Windows headless renderer-engine artifact
```

It intentionally does **not** claim that the inherited ASIO host is the final Windows product shell.

When the normal Windows audio route is implemented and stable, add a separate listening/system-integration gate rather than weakening the engine-core signal.

---

## 11. Practical rule

If a Windows build failure occurs, first identify which layer owns it:

```text
DSP / scene renderer
host audio
ASIO SDK/licensing setup
Windows endpoint lifecycle
optional SAF toolchain
```

Do not let an optional platform/toolchain failure masquerade as evidence that the spatial renderer is broken.
