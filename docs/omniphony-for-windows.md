# Omniphony for Windows

This document defines the Windows product boundary for Omniphony.

## 0.1 product law

The user-facing Windows product is deliberately small:

```text
OmniphonySetup.exe
        ↓
one UAC elevation
        ↓
recover / verify the selected Windows render endpoint
        ↓
establish the proven stereo Current EFX rollback floor
        ↓
attach the Omniphony stream SFX
        ↓
accept authored 7.1 upstream while the physical endpoint remains stereo
        ↓
prove a real 48 kHz / float32 / 7.1 shared client can initialize
        ↓
remove the temporary stereo EFX so Current runs exactly once
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

The normal 0.1 path uses unsigned user-mode APOs. The installer explicitly enables Windows' unprotected AudioDG compatibility mode for that deployment and records the previous machine value so rollback/uninstall can restore it.

Microsoft-signed DriverStore packaging remains an optional future deployment experiment. It is not a prerequisite for using Omniphony 0.1 and is not bundled into the normal installer.

## One renderer, one enhancement law

Omniphony is not a stereo enhancer plus a separate surround renderer. It is one Windows-wide spatial renderer whose behavior becomes more source-authoritative as the host supplies richer input.

```text
stereo
→ preserve the finished master
→ infer only the spatial dimensions the source does not explicitly contain
→ enhance through Current

5.1 / 7.1 / height PCM
→ preserve the authored channels and their positions
→ do less spatial inference because more of the scene is already known
→ enhance through the same Current renderer

8.1.4.4 static objects + dynamic XYZ objects
→ preserve the supplied spatial scene directly
→ avoid reconstructing geometry that the game / host already supplied
→ enhance through the same Current renderer
```

The product goal is therefore continuous across source types:

> **The richer the source truth, the less Omniphony invents and the more authority it gives the source, while preserving the same Omniphony presentation character and final binaural renderer.**

Stereo is the hardest case because Omniphony must infer missing spatial structure from two channels. Native surround should be a stronger input to the same enhancement system, not a weaker or separate mode, because authored direction replaces guesswork. Raw spatial objects are richer again.

The final physical endpoint remains ordinary stereo headphones in every case. The difference is how much trustworthy scene information reaches Omniphony before that final binaural reduction.

## Accepted Windows baseline

As of 2026-08-18, Omniphony's conventional Windows surround path is physically verified and is the baseline product topology:

```text
Windows shared client
48 kHz / float32 / authored 7.1
        ↓
Omniphony stream SFX
        ↓
AUTHORED FL FR C LFE SL SR BL BR
        ↓
Omniphony source scene
        ↓
Current shell / binaural renderer
        ↓
Windows endpoint mix
48 kHz / 32-bit / stereo
        ↓
DAC / headphones
```

The physical endpoint staying stereo is intentional. `IAudioClient::GetMixFormat` describes the endpoint/shared engine mix and therefore remains two-channel on the headphone DAC. The accepted baseline proves the richer input at the client-stream boundary instead:

```text
SHARED_7_1_FORMAT_SUPPORTED      RATE=48000 CHANNELS=8 BITS=32 FORMAT=float32
SHARED_7_1_INITIALIZE_OK         INPUT_CHANNELS=8 ENDPOINT_CHANNELS=2
NATIVE_SURROUND_CLIENT_FORMAT_OK INPUT_CHANNELS=8 ENDPOINT_CHANNELS=2 RATE=48000 BITS=32
NATIVE_SURROUND_SFX 1
NATIVE_SURROUND_EFX 0
OMNIPHONY_INSTALL_STAGE native-surround-active
```

The stereo Current EFX remains a rollback floor during installation and recovery. It is removed after successful native-surround promotion so the signal is rendered by Current exactly once.

For conventional games the intended baseline is:

```text
Windows Spatial Sound: OFF
game output: Home Theater / surround
in-game headphone virtualization: OFF
```

This lets the game author an ordinary multichannel PCM bed and lets Omniphony own the only headphone binaural render. A game-specific listening or telemetry pass is still required to prove that a particular title actually opens and populates the accepted 7.1 stream.

## Audio topology

Stereo, conventional multichannel PCM, and Windows Spatial Audio are progressively richer ingress representations into the same Omniphony renderer. They must not become separate renderers or separate enhancement philosophies.

```text
stereo applications
        ↓
2-channel source
        ↓
Omniphony evidence / inference
        ↓
        ┐
        │
        ├──────────────→ Omniphony source scene → Current shell → binaural
        │
        ┘
conventional surround applications / games
        ↓
shared-mode 5.1 / 7.1 / height PCM
        ↓
authored bed → same Omniphony source scene
        ↓
        ┐
        │
        ├──────────────→ same Current shell → same binaural renderer
        │
        ┘
future richer path:
Windows Spatial Audio applications / games
        ↓
static 8.1.4.4 objects + dynamic 3-D objects
        ↓
Omniphony spatial-object ingress before Windows headphone rendering
        ↓
same source scene → same Current renderer
        ↓
stereo physical endpoint
        ↓
DAC / headphones
```

The conventional 7.1 stream-SFX path is the production baseline. When Windows exposes richer authored spatial truth before headphone rendering, Omniphony must ingest that richer representation rather than intentionally collapsing it to stereo or reconstructing geometry that the source already supplied.

The exact system-wide Windows boundary that can receive another application's raw Spatial Audio objects is still an implementation question and must be proven from Microsoft-supported interfaces before deployment. `ISpatialAudioClient` is an application-side spatial render client/sink; opening a second client is not treated as evidence that Omniphony can intercept another application's object stream.

The previous virtual-device and process-loopback designs are migration history and diagnostic material, not the product architecture.

## Full Current renderer

The Windows APOs load `omniphony_realtime.dll`, which hosts the same Current renderer used by the portable engine. Packaging must not fork, simplify or replace Current.

The internal Current scene uses the canonical 17-anchor 8.1.4.4 vocabulary:

```text
L R C LFE Ls Rs Lb Rb Cb Tfl Tfr Tbl Tbr Bfl Bfr Bbl Bbr
```

**8.1.4.4 remains the ideal full fixed Windows scene vocabulary.** It is the canonical static coordinate frame, not a claim that every source contains seventeen authored channels.

Stereo-derived Current presently populates the evidence-backed lanes:

```text
L R Ls Rs Lb Rb Tfl Tfr Tbl Tbr
```

and leaves these EMPTY:

```text
C LFE Cb Bfl Bfr Bbl Bbr
```

That canonical scene feeds the Current System-H-derived spatial shell and binaural renderer before returning stereo to the physical headphone endpoint.

For authored Windows speaker beds, the realtime native-bed path maps the supplied `WAVEFORMATEXTENSIBLE` channel mask to authored source positions and bypasses stereo spatial inference. Missing canonical anchors remain empty. LFE remains non-directional source evidence and is handled separately from HRTF placement.

The accepted conventional 7.1 baseline therefore maps:

```text
FL FR C LFE SL SR BL BR = AUTHORED
BC TFL TFR TBL TBR BFL BFR BBL BBR = EMPTY unless separately earned
```

Windows Spatial Audio is the richer target. Static objects map directly to their supplied fixed roles, including the lower hemisphere when present, and dynamic objects retain their supplied 3-D positions over time. Omniphony may render those sources through its existing shell and binaural machinery, but it may not erase their authored spatial identity first.

Dynamic XYZ objects are richer source geometry than the fixed 8.1.4.4 skeleton when they are actually supplied. They remain continuous objects rather than being snapped to static anchors merely to fit the scene vocabulary.

The installer build runs the renderer, realtime ABI, DSP fixture, reference-bridge and engine tests before producing `OmniphonySetup.exe`.

## Current Windows ingress boundary

The physically accepted production path is now:

```text
stereo client
→ Omniphony stream SFX
→ Current
→ stereo endpoint

or

authored 7.1 client
→ Omniphony stream SFX
→ native-bed realtime ABI
→ authored source coordinates
→ Current shell / binaural
→ stereo endpoint
```

These are not two product modes. They are two source-authority states entering the same renderer. The stereo case uses bounded inference where the source lacks explicit geometry; the authored 7.1 case replaces those guesses with real speaker-channel authority and then receives the same Omniphony enhancement/rendering treatment.

A separate stereo endpoint EFX remains implemented as the safe rollback floor. It is not the promoted steady-state path after successful 7.1-capable installation.

Authored 7.1.4 processing is also implemented and regression-tested inside the stream APO/native-bed path. That fixture proves the renderer and APO can handle the richer bed; it does not yet prove that an ordinary Windows application will open that exact 12-channel shared stream on the current product host.

Raw Windows Spatial Audio object ingress is a **required Windows host capability**, not an optional research curiosity. The target representation is the source data available before the active Windows spatial renderer collapses it for the physical headphone endpoint:

```text
8.1.4.4 static spatial objects
→ preserve fixed role / identity

+ dynamic spatial objects
→ preserve per-object audio + supplied XYZ trajectory

→ Omniphony source-scene adapter
→ existing Current shell / binaural renderer
→ stereo headphones
```

This requirement does not authorize an undocumented hook. The host mechanism must first be demonstrated with an actual spatial-aware application and a Microsoft-supported boundary. Until that boundary is proven, the conventional 7.1 stream-SFX path is the production baseline.

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
2. verifies the realtime renderer;
3. repairs known Omniphony global APO registration before endpoint discovery when recovering an existing install;
4. resolves and persists the selected physical endpoint identity;
5. saves previous endpoint and AudioDG state;
6. establishes the stereo Current EFX rollback floor;
7. verifies stereo Current processing and the physical endpoint;
8. registers the native stream APO;
9. attaches the format-changing SFX and removes the temporary EFX;
10. restarts the Windows audio graph and waits for the exact endpoint to return ACTIVE;
11. requires the physical endpoint to remain 48 kHz / 32-bit / stereo;
12. requires a 48 kHz / float32 / 7.1 shared client format to report supported and successfully initialize;
13. keeps the stream SFX only after that real client-boundary proof;
14. restores the stereo Current EFX automatically if native-surround promotion fails;
15. starts the tray icon when setup succeeds.

The installed runtime is intentionally small:

```text
C:\Program Files\Omniphony\APO\OmniphonyAPO.dll
C:\Program Files\Omniphony\APO\OmniphonyStreamAPO.dll
C:\Program Files\Omniphony\APO\omniphony_realtime.dll
C:\Program Files\Omniphony\support\...
```

There is no resident `Omniphony.exe` audio host. Audio processing occurs in the Windows APO path.

If the eventual Microsoft-supported raw-object boundary requires an additional installed Windows component, that component must remain headless and subordinate to the same one-installer product law. It does not become a second user-facing audio host or a second renderer.

## Endpoint continuity

The physical endpoint may temporarily become inactive or disappear from Core Audio when a USB DAC is powered off, unplugged, restarted, or when Windows restarts its audio services. That must not erase Omniphony's installation state.

Omniphony persists the verified endpoint identity and uses it for recovery. Installation/recovery must never deregister a previously working global APO merely because endpoint discovery temporarily returns no active device. The endpoint must return ACTIVE before Omniphony mutates endpoint FX state or declares the path healthy.

Normal DAC power cycling is therefore treated as endpoint availability, not product installation state. A genuinely new Windows endpoint identity, such as one created by a driver/topology change, may require reattachment.

## Tray contract

The notification-area icon is the normal UI.

Current tray controls include listener EQ selection and right-ear compensation. The tray writes tiny preference state only. It does not carry the audio stream and exiting it does not stop Current.

Future controls can be added to the tray as capabilities mature, while preserving the headless renderer.

## Failure and uninstall law

Installation must leave ordinary Windows audio recoverable.

The accepted transaction is deliberately two-stage: establish a proven stereo Current floor, attempt native-surround promotion, and only remove that floor after the stream SFX is attached. If the richer client-stream proof fails, the installer removes the SFX and restores the stereo Current EFX.

A failure before endpoint discovery must not dismantle a previously known installation. Rollback is successful only when the restored endpoint state is verified, not merely because a cleanup command was attempted.

Uninstall removes Omniphony's stream/endpoint attachments and runtime files and restores the previous AudioDG state. It must not replace or uninstall the physical DAC/audio driver.

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

Build upward from the accepted baseline rather than sideways:

1. preserve the physically verified stereo and authored-7.1 paths as two source-authority states of one renderer, not separate product modes;
2. use real games such as Overwatch to compare the conventional 7.1 input against the game's richer spatial path and identify what source geometry 7.1 is missing;
3. treat Dolby Atmos for Headphones or another native game spatial renderer as a perceptual reference for height/directional information, not as the desired final architecture when it has already binauralized the scene;
4. prove the Microsoft-supported system boundary, if any, that exposes raw Windows Spatial Audio static/dynamic objects to an installed system-wide renderer before Sonic / Dolby headphone rendering;
5. add a Windows spatial-object adapter that preserves the full 8.1.4.4 static vocabulary plus dynamic XYZ trajectories into Omniphony's existing source-scene contract;
6. feed that richer source truth through the same Current enhancement / shell / binaural machinery rather than creating a second spatial renderer;
7. prove reboot, sleep/resume and DAC power-cycle/reconnect behavior around the accepted endpoint continuity contract;
8. improve the tray icon and controls;
9. add trustworthy already-binaural detection/bypass when host evidence exists;
10. add optional deepSTRF/research capabilities only when validated and without replacing the established Current sound.

The baseline remains deliberately simple for the user: **click install, use the selected physical Windows output, and let Omniphony enhance whatever source truth Windows supplies. Stereo is inferred where necessary; native surround is preserved where authored; richer spatial objects should be preserved directly when the host exposes them. All of it converges on one Omniphony renderer.**
