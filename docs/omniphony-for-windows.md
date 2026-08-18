# Omniphony for Windows

This document defines the Windows product boundary for Omniphony.

## 0.1 product law

The user-facing Windows product is deliberately small:

```text
OmniphonySetup.exe
        ↓
one UAC elevation
        ↓
attach Omniphony to the current Windows render endpoint
        ↓
restart the Windows audio graph
        ↓
tray icon appears
        ↓
Current renders system audio headlessly
```

Normal use has:

- one installer EXE;
- no virtual cable;
- no loopback host;
- no console;
- no taskbar window;
- no helper application that must remain open;
- one small notification-area/tray icon for preferences;
- the renderer remaining active even if the tray icon is closed.

The normal 0.1 path is an **unsigned user-mode endpoint APO**. The installer explicitly enables Windows' unprotected AudioDG compatibility mode for that APO deployment and records the previous machine value so rollback/uninstall can restore it.

Microsoft-signed DriverStore packaging remains an optional future deployment experiment. It is not a prerequisite for using Omniphony 0.1 and is not bundled into the normal installer.

## Audio topology

Conventional PCM and Windows Spatial Audio are two source-truth paths into the same Omniphony renderer. They must not become two renderers.

```text
conventional PCM applications
        ↓
Windows Audio Engine / stream + endpoint APO path
        ↓
authored PCM bed or stereo ingress
        ┐
        │
        ├──────────────→ Omniphony source scene → Current shell → binaural
        │
        ┘
Windows Spatial Audio applications / games
        ↓
Windows spatial-renderer boundary
        ↓
static 8.1.4.4 objects + dynamic 3-D objects
        ↓
Omniphony spatial-object ingress
        ↓
physical endpoint driver
        ↓
DAC / headphones
```

The conventional endpoint/stream APO path remains the universal fallback. When Windows exposes richer authored spatial truth before headphone rendering, Omniphony must ingest that richer representation rather than intentionally collapsing it to stereo or reconstructing geometry that the source already supplied.

The exact system-wide Windows boundary that can receive another application's raw Spatial Audio objects is still an implementation question and must be proven from Microsoft-supported interfaces before deployment. `ISpatialAudioClient` is an application-side spatial render client/sink; opening a second client is not treated as evidence that Omniphony can intercept another application's object stream.

The previous virtual-device and process-loopback designs are migration history and diagnostic material, not the product architecture.

## Full Current renderer

The Windows APO loads `omniphony_realtime.dll`, which hosts the same Current renderer used by the portable engine. Packaging must not fork, simplify or replace Current.

The internal Current scene uses the canonical 17-anchor 8.1.4.4 vocabulary:

```text
L R C LFE Ls Rs Lb Rb Cb Tfl Tfr Tbl Tbr Bfl Bfr Bbl Bbr
```

Stereo-derived Current presently populates the evidence-backed lanes:

```text
L R Ls Rs Lb Rb Tfl Tfr Tbl Tbr
```

and leaves these EMPTY:

```text
C LFE Cb Bfl Bfr Bbl Bbr
```

That canonical scene feeds the Current System-H-derived spatial shell and binaural renderer before returning stereo to the physical headphone endpoint.

```text
Windows stereo endpoint input
→ canonical Current scene
→ Current spatial support / room / binaural pipeline
→ listener correction and safety
→ stereo headphones
```

For authored Windows speaker beds, the realtime native-bed path instead maps the supplied `WAVEFORMATEXTENSIBLE` channel mask to authored source positions and bypasses stereo spatial inference. Missing canonical anchors remain empty. LFE remains non-directional source evidence and is handled separately from HRTF placement.

Windows Spatial Audio must go one step richer: static objects map directly to their supplied fixed spatial roles, including the lower hemisphere when present, and dynamic objects retain their supplied 3-D positions over time. Omniphony may render those sources through its existing shell and binaural machinery, but it may not erase their authored spatial identity first.

The installer build runs the renderer, realtime ABI, DSP fixture, reference-bridge and engine tests before producing `OmniphonySetup.exe`.

## Current Windows ingress boundary

The proven endpoint fallback remains:

```text
stereo float32 → Current → stereo float32
```

A separate native-bed realtime ABI and pre-mix stream APO now exist for authored multichannel `WAVEFORMATEXTENSIBLE` PCM. That path is the PCM half of source-aware Windows ingress. Its Windows SFX format negotiation and installer promotion are still being hardened, so repository implementation is not yet equivalent to a physically verified released multichannel product.

```text
authored 5.1 / 7.1 / height PCM bed
→ Windows stream APO negotiation
→ native-bed realtime ABI
→ authored source coordinates
→ Current shell / binaural
→ stereo headphones
```

Raw Windows Spatial Audio object ingress is now a **required Windows host capability**, not an optional research curiosity. The target representation is the source data available before the active Windows spatial renderer collapses it for the physical headphone endpoint:

```text
static spatial objects
→ preserve fixed object role / identity

+ dynamic spatial objects
→ preserve per-object audio + supplied XYZ trajectory

→ Omniphony source-scene adapter
→ existing Current shell / binaural renderer
→ stereo headphones
```

This requirement does not authorize an undocumented hook. The host mechanism must first be demonstrated with an actual spatial-aware application and a Microsoft-supported boundary. Until that boundary is proven, the conventional PCM APO remains the safe production path and the native-bed stream APO remains the authored-PCM path.

### Windows Spatial Audio acceptance conditions

Spatial ingress is not complete merely because Omniphony can open an `ISpatialAudioClient`. Completion requires evidence that the path receives the source application's spatial representation before Windows' headphone renderer consumes it.

The minimum gates are:

1. enumerate the static spatial-object mask actually offered by the active Windows spatial renderer;
2. preserve every received static role without remapping it through stereo inference, including the four lower 8.1.4.4 roles when supplied;
3. preserve dynamic-object identity, PCM and 3-D position updates without quantizing them to a fixed speaker bed unless Omniphony's renderer itself intentionally performs that render step;
4. prove object motion and static lane identity with deterministic fixtures before using a game as evidence;
5. prove a real spatial-aware Windows application reaches the ingress path;
6. demonstrate that disabling Omniphony returns the normal Windows spatial-renderer path cleanly;
7. retain the ordinary stereo/native-PCM fallback if raw object ingress is unavailable;
8. keep Windows-specific capture/provider concepts out of the portable renderer core.

Microsoft's public Spatial Sound contract is the primary platform reference for this boundary:

- <https://learn.microsoft.com/windows/win32/coreaudio/spatial-sound>
- <https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nn-spatialaudioclient-ispatialaudioclient>
- <https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nn-spatialaudioclient-ispatialaudioobject>
- <https://learn.microsoft.com/windows/win32/api/spatialaudioclient/nn-spatialaudioclient-ispatialaudioobjectrenderstream>

## Realtime architecture

The AudioDG callback does not run the allocating renderer graph directly. The callback-facing layer uses bounded/preallocated PCM transfer while a dedicated worker owns Current DSP.

The runtime retains:

- preallocated callback-facing rings;
- dedicated Current worker processing;
- time-aligned dry fallback;
- non-finite sanitization;
- linked peak safety;
- explicit create/destroy lifecycle testing;
- manifest/import/ABI checks in CI.

The audio callback must not perform filesystem I/O, network activity, device discovery or research-time analysis.

A future spatial-object host should obey the same realtime law: Windows-facing callbacks exchange bounded preallocated object audio/state with the renderer worker; they do not move the allocating graph onto an OS realtime callback.

## Installer behavior

`OmniphonySetup.exe` performs the complete normal installation without asking the user to run scripts manually.

It:

1. stops any obsolete Omniphony host/tray instance;
2. stages the current APO and realtime renderer;
3. resolves the current default Windows render endpoint;
4. saves the previous endpoint and AudioDG state;
5. verifies the realtime renderer before attachment;
6. registers the user-mode APO;
7. enables the unsigned-APO AudioDG compatibility mode;
8. attaches Omniphony to the endpoint and keeps Windows enhancements enabled;
9. restarts the Windows audio graph;
10. runs APO and physical-endpoint smoke probes;
11. rolls back automatically if installation fails;
12. starts the tray icon when setup succeeds.

The installed runtime is intentionally small:

```text
C:\Program Files\Omniphony\APO\OmniphonyAPO.dll
C:\Program Files\Omniphony\APO\omniphony_realtime.dll
C:\Program Files\Omniphony\support\...
```

There is no resident `Omniphony.exe` audio host. Audio processing occurs in the Windows endpoint APO path.

If the eventual Microsoft-supported raw-object boundary requires an additional installed Windows component, that component must remain headless and subordinate to the same one-installer product law. It does not become a second user-facing audio host or a second renderer.

## Tray contract

The notification-area icon is the normal UI.

Current tray controls include listener EQ selection and right-ear compensation. The tray writes tiny preference state only. It does not carry the audio stream and exiting it does not stop Current.

Future controls can be added to the tray as capabilities mature, while preserving the headless renderer.

## Failure and uninstall law

Installation must leave ordinary Windows audio recoverable.

If the APO fails to attach or the post-install endpoint probe fails, setup attempts to bypass/detach Omniphony, restart the audio graph, unregister the APO and restore the captured AudioDG setting.

Uninstall removes Omniphony's endpoint attachment and runtime files and restores the previous AudioDG state. It must not replace or uninstall the physical DAC/audio driver.

Any future spatial-object component must obey the same rollback law. Failure must restore the user's previous Windows Spatial Sound behavior rather than leaving the endpoint in an Omniphony-only or partially intercepted state.

## Optional future signed deployment

`windows_installer/endpoint_apo/production/` contains the more elaborate componentized DriverStore/APO work developed during hardening.

Keep it as an optional research/deployment route for later. It must not make the simple unsigned 0.1 product harder to install or use.

If a future signed route becomes genuinely one-click and provides a clear benefit, it can replace the deployment mechanism underneath the same product law:

```text
one EXE
tray icon only
headless full Current renderer
physical endpoint remains the user's normal output
```

## Next product frontier

Build upward from source truth rather than sideways:

1. finish and physically verify native authored multichannel SFX negotiation without touching the renderer;
2. prove the Microsoft-supported system boundary, if any, that exposes raw Windows Spatial Audio static/dynamic objects to an installed system-wide renderer;
3. add a Windows spatial-object adapter that preserves static 8.1.4.4 roles and dynamic XYZ object trajectories into Omniphony's existing source-scene contract;
4. prove the path with deterministic object fixtures, then a real spatial-aware game such as Overwatch, without assuming in advance which bed/object mix the game emits;
5. make endpoint selection/change handling smoother and prove reboot, sleep/resume and device reconnect behavior;
6. improve the tray icon and controls;
7. add trustworthy already-binaural detection/bypass when host evidence exists;
8. add optional deepSTRF/research capabilities only when validated and without replacing the established Current sound.

The baseline remains deliberately simple for the user: **click install, see the tray icon, and hear Omniphony everywhere through the selected Windows output.** The source path underneath that simplicity should become richer whenever Windows exposes richer authored truth.
