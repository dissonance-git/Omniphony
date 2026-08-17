# Omniphony native APO Current path

This is the endpoint-native Windows path for the retained Omniphony Current renderer. It uses the physical render endpoint directly. It does **not** create an Omniphony playback device and it does not require a virtual cable.

Two deployment states must stay distinct:

```text
DEVELOPMENT / PHYSICAL BRING-UP
Inno Setup + direct endpoint association
+ temporary protected-AudioDG test override

PRODUCTION TARGET
signed componentized AudioProcessingObject package
+ driver extension association
+ protected AudioDG left enabled
```

Both states use the same `OmniphonyAPO.dll`, `omniphony_realtime.dll`, and Current DSP. Packaging is not a second renderer.

## Installed development layout

The current development installation is intentionally small:

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

The old `driver\` directory and legacy loopback-host `Omniphony.exe` are not part of this path. The old host is deliberately removed during migration.

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
listener-specific headphone correction
        ↓
retained final linked peak guard
        ↓
physical endpoint
```

Mode 0 remains exact identity inside the portable realtime ABI as a deterministic transport oracle. The endpoint APO selects **mode 1 / Current** for supported stereo float32 graphs.

The canonical scene remains 17-lane 8.1.4.4. The 22-direction System-H-derived shell is downstream rendering geometry, not the scene vocabulary.

## Personal output correction

The Omniphony foundation EQ and the listener-specific headphone correction are separate layers. The current personal build includes the primary Noire X correction after the Current master/spatial sum and before the final linked peak guard:

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

The current native path reports a fixed **40 ms / 1920-frame** host delay at 48 kHz. The same timeline is maintained for a delayed-dry safety lane. Worker underruns therefore substitute the matching delayed dry frame rather than jumping forward in time. Late Current frames are discarded before Current resumes. The final output remains bounded by the retained linked -1.0 dBFS ceiling.

## Development endpoint attach

The real Windows audio engine is a stronger gate than an ordinary COM smoke process. The current `0.0.4-dev` installer therefore keeps a deliberately explicit bring-up path:

- `OmniphonyAPO.dll` is linked with `/MANIFEST:NO`;
- the APO and realtime DLL are placed together before registration;
- AudioSrv is stopped before replacing a runtime that AudioDG may have loaded;
- the legacy loopback host and old autostart entries are removed during migration;
- the installer snapshots the prior endpoint and AudioDG protection state before changing it;
- the development path sets `DisableProtectedAudioDG=1` while this unsigned/manual package is active;
- the endpoint EFX association is written only after checking that another EFX is not being displaced;
- post-attach COM and real-endpoint WASAPI probes are mandatory;
- failure detaches/bypasses the APO and restores the prior protection state;
- uninstall restores the saved protection state and endpoint settings.

That `DisableProtectedAudioDG` override is **development scaffolding, not a production invariant**. A release package must not require it.

Stable APO CLSID:

```text
{A9333BFE-39C1-40FD-B4B0-ECC591410B47}
```

## Production package

The production package scaffold lives in `production/`.

`production/OmniphonyApoComponent.inx` defines a Windows 11 `AudioProcessingObject` software component with:

```text
SWC\VEN_OMNI&CID_CURRENT
        ↓
DriverStore
  OmniphonyAPO.dll
  omniphony_realtime.dll
        ↓
HKR-isolated COM registration
+ HKR-isolated AudioEngine APO registration
```

`production/check_package_contract.py` prevents the production component from drifting back toward global HKLM/HKCR registration, endpoint `FxProperties` surgery, or the protected-AudioDG development switch. Windows CI also runs the WDK `InfVerif /w` isolation verifier against the template.

The remaining production association is intentionally not guessed: the real target audio-driver identity must be captured, then an extension/package association must create `VEN_OMNI&CID_CURRENT` for that driver. See `production/README.md`.

## Single-EXE development installer

`OmniphonySetup.exe` remains the convenient physical bring-up artifact while production driver packaging is completed. It carries build/smoke files only as temporary setup payload, then keeps the small development layout above.

During an in-place development upgrade it:

1. kills any running legacy `Omniphony.exe` process;
2. removes obsolete autostart entries and old virtual-driver product files;
3. validates the realtime ABI and Current worker from the temporary setup payload;
4. stops AudioSrv, replaces `OmniphonyAPO.dll` and `omniphony_realtime.dll`, and registers the development APO;
5. applies the protected-AudioDG development override after saving the prior value;
6. attaches Omniphony to the current physical endpoint without overwriting a different EFX;
7. restarts Windows Audio and runs the COM/Current smoke test;
8. calls real endpoint `GetMixFormat` as the final install gate;
9. automatically rolls back the association and protection state if the Windows audio gate fails.

No legacy Omniphony playback device is selected or required.

## Development artifact

The standalone bring-up package can still be installed from elevated PowerShell:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\Install-OmniphonyAPO.ps1
```

For diagnostics:

```powershell
.\OmniphonyApoCtl.exe status
.\OmniphonyMixProbe.exe "Dan Clark Noire X" FiiO Noire
```

Expected successful evidence includes:

```text
EFX     {A9333BFE-39C1-40FD-B4B0-ECC591410B47}
ENHANCEMENTS_DISABLED   0
MIX_FORMAT_OK   ...
```

## Evidence states

Keep these separate:

```text
APO source builds
≠ canonical Current DSP contracts pass
≠ realtime ABI tests pass
≠ development COM activation succeeds
≠ development endpoint association succeeds
≠ post-restart GetMixFormat succeeds
≠ ordinary application PCM is stable on the physical machine
≠ component INF passes current WDK validation
≠ signed production component is associated with the target audio driver
≠ production APO loads with protected AudioDG enabled
≠ production Current is physically preferred
```

The earlier physical package reached endpoint association but real application playback failed with `IAudioClient::GetMixFormat` returning `0x80070005`. The current development repair path and post-restart WASAPI gate exist to test that failure directly. It must not be promoted to a production claim until the protected, componentized package crosses the same physical gates.
