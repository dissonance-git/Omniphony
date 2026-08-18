# Omniphony native APO Current path

This directory contains the normal Windows 0.1 endpoint-native product path for Omniphony Current.

It uses the selected physical render endpoint directly. It does **not** create an Omniphony playback device, require a virtual cable, or keep an audio-host application running.

## 0.1 deployment contract

```text
OmniphonySetup.exe
        ↓
one UAC elevation
        ↓
install OmniphonyAPO.dll + omniphony_realtime.dll
        ↓
register unsigned user-mode EFX APO
        ↓
enable Windows' unprotected AudioDG compatibility mode
        ↓
attach to current render endpoint
        ↓
restart Windows Audio + probe endpoint
        ↓
tray icon appears
        ↓
Current renders headlessly
```

This unsigned APO route is the **supported quick-install product path** for 0.1. It is no longer classified as a temporary development-only bring-up harness.

The componentized/signed DriverStore machinery under `production/` remains an optional future deployment experiment. It is not required to install or use Omniphony 0.1.

## Installed layout

```text
C:\Program Files\Omniphony\
├─ APO\
│  ├─ OmniphonyAPO.dll
│  └─ omniphony_realtime.dll
├─ support\
│  ├─ Install-OmniphonyAPO.ps1
│  ├─ Uninstall-OmniphonyAPO.ps1
│  ├─ OmniphonyApoCtl.exe
│  ├─ OmniphonyMixProbe.exe
│  ├─ OmniphonyEndpointCtl.exe
│  └─ OmniphonyTray.ps1
├─ LICENSE
└─ Inno Setup uninstaller files
```

The old virtual-device `driver\` directory and loopback-host `Omniphony.exe` are migration history and are removed during upgrade.

## Processing path

```text
Windows audio engine / AudioDG
        ↓
OmniphonyAPO.dll
        ↓ cached realtime ABI call
omniphony_realtime.dll
        ↓
bounded delayed-dry safety lane
+ dedicated Current worker
        ↓
canonical 8.1.4.4 scene
        ↓
Current 22-direction shell
        ↓
cascaded binaural / measured HRTF
        ↓
protected master + foundation + spatial support
        ↓
listener-specific correction
        ↓
linked peak guard
        ↓
physical endpoint
```

Mode 0 remains exact identity inside the portable realtime ABI as a deterministic transport oracle. The endpoint APO selects **mode 1 / Current** for supported stereo float32 graphs.

The canonical scene remains the 17-lane 8.1.4.4 vocabulary. The 22-direction System-H-derived shell is downstream rendering geometry, not a replacement scene model.

## Tray contract

The notification-area icon is the only normal UI surface. It currently exposes listener EQ choices and right-ear compensation.

The tray does not host or transport audio. Closing it does not stop Current, because processing remains inside the Windows endpoint APO path.

## Unsigned AudioDG compatibility mode

The 0.1 installer intentionally uses the Windows compatibility path required by this unsigned user-mode APO:

```text
HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio
DisableProtectedAudioDG = 1
```

The installer snapshots the previous value before changing it. Rollback and uninstall restore that saved state. This is an explicit 0.1 product tradeoff in exchange for Equalizer-APO-style one-click deployment without a Microsoft driver-signing workflow.

## Install transaction

`OmniphonySetup.exe` performs the complete normal installation. The user should not need to run the PowerShell helper manually.

During install/upgrade it:

1. stops any obsolete Omniphony host/tray instance;
2. removes obsolete virtual-device/loopback files and autostart entries;
3. validates the realtime renderer before attachment;
4. resolves the current default render endpoint;
5. snapshots endpoint state and the prior AudioDG protection value;
6. stops AudioSrv before replacing loaded runtime DLLs;
7. installs and globally registers the user-mode APO;
8. enables the unsigned-APO AudioDG compatibility mode;
9. refuses to overwrite a different existing endpoint EFX;
10. attaches Omniphony while keeping endpoint enhancements enabled;
11. restarts the Windows audio graph;
12. runs COM/Current and real-endpoint WASAPI probes;
13. automatically detaches/unregisters/restores the previous state if a gate fails;
14. starts the tray icon after successful setup.

Stable APO CLSID:

```text
{A9333BFE-39C1-40FD-B4B0-ECC591410B47}
```

## Personal output correction

The Omniphony foundation EQ and listener-specific headphone correction are separate layers. The current personal build includes the Noire X correction after the Current master/spatial sum and before the final linked peak guard.

Equalizer APO is **not** a runtime dependency. Omniphony only adopts the same broad unsigned-APO deployment tradeoff.

## Fixed-latency safety lane

The current native path reports a fixed **40 ms / 1920-frame** host delay at 48 kHz. The same timeline is maintained for a delayed-dry safety lane. Worker underruns substitute the matching delayed dry frame rather than jumping forward in time. Late Current frames are discarded before Current resumes.

## Diagnostics

The normal product is the EXE installer, but the retained support helpers can diagnose a machine when needed:

```powershell
OmniphonyApoCtl.exe status
OmniphonyMixProbe.exe "Dan Clark Noire X" FiiO Noire
```

Expected successful evidence includes:

```text
EFX     {A9333BFE-39C1-40FD-B4B0-ECC591410B47}
ENHANCEMENTS_DISABLED   0
MIX_FORMAT_OK   ...
```

## Optional signed DriverStore experiment

`production/` retains the componentized Windows APO work: target capture, generated extension INF, DriverStore component package, catalog/signing hooks, transactional install/rollback and protected-AudioDG probes.

That work is useful if Omniphony later wants a signed/protected distribution route, but it must not be described as unfinished work blocking the 0.1 product.

See `production/README.md` for that optional track.

## Evidence states

Keep engineering evidence distinct:

```text
APO source builds
≠ canonical Current DSP contracts pass
≠ realtime ABI tests pass
≠ COM activation succeeds
≠ endpoint association succeeds
≠ post-restart GetMixFormat succeeds
≠ ordinary application PCM is stable on the physical machine
≠ audible Current behavior is preferred
```

For the optional signed path there are additional DriverStore/protected-AudioDG gates, but those are not prerequisites for the normal 0.1 installer.
