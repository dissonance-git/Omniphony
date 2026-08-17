# Omniphony native APO Current candidate

This is the APO-native Windows path for the retained Omniphony Current renderer. It uses the physical FiiO / Noire X render endpoint directly. It does **not** create an Omniphony playback device and it does not require a virtual cable.

## Installed layout

The product installation is intentionally small:

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
│  └─ OmniphonyEndpointCtl.exe
├─ LICENSE
└─ Inno Setup uninstaller files
```

The old `driver\` directory and legacy loopback/tray `Omniphony.exe` are not part of this product path. A future tray/settings GUI may occupy the root later, but the old loopback host is deliberately removed during migration.

## Processing path

```text
Windows audio engine / audiodg
        ↓
OmniphonyAPO.dll
        ↓ cached realtime ABI call
omniphony_realtime.dll
        ↓
bounded delayed-dry safety lane
+ dedicated Current worker
        ↓
protected master + foundation + spatial support
        ↓
primary Noire X personal output correction
        ↓
retained final linked peak guard
        ↓
physical FiiO / Noire X endpoint
```

Mode 0 remains exact identity inside the portable realtime ABI as a deterministic transport oracle. The endpoint APO selects **mode 1 / Current** for supported stereo float32 graphs.

## Personal output correction

The Omniphony foundation EQ and the listener-specific headphone correction are separate layers. This personal build includes the primary Noire X correction after the Current master/spatial sum and before the final linked peak guard:

```text
shared preamp      -4.0 dB
15 Hz high-pass    Q 0.6
45 Hz low shelf    +3.5 dB Q 0.5
30 Hz peak         +1.2 dB Q 0.8
85 Hz peak         +2.0 dB Q 0.65
155 Hz peak        +1.3 dB Q 0.75
240 Hz peak        -0.2 dB Q 0.9
420 Hz peak        +0.8 dB Q 0.7
700 Hz peak        +0.8 dB Q 0.8
1.2 kHz peak       +0.9 dB Q 0.7
1.9 kHz peak       +0.5 dB Q 0.8
2.8 kHz peak       -0.6 dB Q 0.6
3.8 kHz peak       -2.2 dB Q 0.9
4.8 kHz peak       -2.6 dB Q 1.1
6.2 kHz peak       -0.9 dB Q 1.3
7.2 kHz high shelf -1.8 dB Q 0.7

right only
preamp             -0.4 dB
180 Hz peak         -0.3 dB Q 0.9
3.0 kHz peak        -1.1 dB Q 1.0
6.2 kHz high shelf  -0.3 dB Q 0.7
delay                0.02 ms
```

Equalizer APO is not a runtime dependency.

## Fixed-latency safety lane

The first physical Current candidate reports a fixed **40 ms / 1920-frame** host delay at 48 kHz. The same timeline is maintained for a delayed-dry safety lane. Worker underruns therefore substitute the matching delayed dry frame rather than jumping forward in time. Late Current frames are discarded before Current resumes. The final output remains bounded by the retained linked -1.0 dBFS ceiling.

## Protected AudioDG deployment

The real Windows audio engine is a stronger gate than an ordinary COM smoke process. The APO deployment therefore enforces:

- `OmniphonyAPO.dll` is linked with `/MANIFEST:NO`;
- the two runtime DLLs are installed under `C:\Program Files\Omniphony\APO` before COM registration;
- AudioSrv is stopped before runtime replacement so future upgrades can replace an APO currently loaded by AudioDG;
- the legacy `Omniphony.exe` process is forcibly terminated before migration;
- old `Omniphony` / `Spatial` Run-key autostart entries are removed by the single-EXE installer;
- the old product `driver\` directory and `EndpointAPO` staging name are deleted during migration;
- after AudioSrv restarts, `OmniphonyMixProbe.exe` activates the real physical endpoint and calls `IAudioClient::GetMixFormat`;
- a failed post-attach WASAPI probe triggers automatic APO detach/rollback instead of leaving the endpoint unusable.

Stable APO CLSID:

```text
{A9333BFE-39C1-40FD-B4B0-ECC591410B47}
```

## Single-EXE installer

The intended installation path is `OmniphonySetup.exe`. The installer carries build/smoke files only as temporary setup payload, then keeps only the small installed layout above.

During an in-place upgrade it:

1. kills any running legacy `Omniphony.exe` process;
2. removes obsolete autostart entries and old virtual-driver product files;
3. validates the realtime ABI and Current worker from the temporary setup payload;
4. stops AudioSrv, replaces `C:\Program Files\Omniphony\APO\OmniphonyAPO.dll` and `omniphony_realtime.dll`, and registers that installed APO;
5. restarts AudioSrv and runs the COM/Current smoke test;
6. attaches Omniphony to the physical FiiO / Noire endpoint;
7. restores the physical endpoint as Windows default;
8. restarts Windows Audio and verifies EFX association;
9. calls real endpoint `GetMixFormat` as the final install gate;
10. automatically rolls back the APO association if the Windows audio gate fails.

No legacy Omniphony playback device is selected or required.

## Development artifact

The standalone APO artifact can still be installed from elevated PowerShell:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\Install-OmniphonyAPO.ps1
```

For diagnostics:

```powershell
.\OmniphonyApoCtl.exe status
.\OmniphonyMixProbe.exe "Dan Clark Noire X" FiiO Noire
```

Expected successful evidence:

```text
EFX     {A9333BFE-39C1-40FD-B4B0-ECC591410B47}
ENHANCEMENTS_DISABLED   0
MIX_FORMAT_OK   Dan Clark Noire X (FiiO Q series)   ...
```

## Evidence states

Keep these separate:

```text
APO builds
≠ realtime ABI Current tests pass
≠ COM registration/activation succeeds
≠ physical endpoint accepts the EFX association
≠ post-restart GetMixFormat succeeds through the real Windows endpoint
≠ Current PCM is stable on the physical machine
≠ Current + personal EQ is preferred in physical listening
```

The first audible Current package reached endpoint association on the primary FiiO / Noire machine but real application playback failed with `IAudioClient::GetMixFormat` returning `0x80070005`. The protected-host deployment and post-restart WASAPI gate exist specifically to close that evidence gap before another physical listening claim is made.
