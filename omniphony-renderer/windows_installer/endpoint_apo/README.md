# Omniphony native APO Current path

This directory contains the normal Windows 0.1 endpoint-native product path for Omniphony Current.

It uses the selected physical render endpoint directly. It does **not** create an Omniphony playback device, require a virtual cable, or keep an audio-host application running.

## Accepted 0.1 deployment contract

The steady-state Windows baseline is now the stream-SFX path:

```text
OmniphonySetup.exe
        ↓
one UAC elevation
        ↓
recover / verify the selected physical endpoint
        ↓
install OmniphonyAPO.dll + OmniphonyStreamAPO.dll + omniphony_realtime.dll
        ↓
establish stereo Current EFX rollback floor
        ↓
register unsigned user-mode stream SFX
        ↓
attach SFX and remove temporary EFX
        ↓
restart Windows Audio
        ↓
prove 48 kHz / float32 / 7.1 shared client initialization
while endpoint remains 48 kHz / 32-bit / stereo
        ↓
tray icon appears
        ↓
Current renders headlessly
```

This unsigned APO route is the **supported quick-install product path** for 0.1. It is no longer classified as a temporary development-only bring-up harness.

The componentized/signed DriverStore machinery under `production/` remains an optional future deployment experiment. It is not required to install or use Omniphony 0.1.

## Physically verified baseline

The conventional Windows surround path was physically accepted on 2026-08-18 with the Noire X / FiiO endpoint.

The accepted machine evidence is:

```text
MIX_FORMAT_OK
RATE=48000 CHANNELS=2 BITS=32

SHARED_7_1_FORMAT_SUPPORTED
RATE=48000 CHANNELS=8 BITS=32 FORMAT=float32

SHARED_7_1_INITIALIZE_OK
RATE=48000 INPUT_CHANNELS=8 ENDPOINT_CHANNELS=2 BITS=32 FORMAT=float32

NATIVE_SURROUND_CLIENT_FORMAT_OK
INPUT_CHANNELS=8 ENDPOINT_CHANNELS=2 RATE=48000 BITS=32

NATIVE_SURROUND_SFX 1
NATIVE_SURROUND_EFX 0
OMNIPHONY_INSTALL_STAGE native-surround-active
```

The important topology is therefore:

```text
authored Windows client stream
stereo / 5.1 / 7.1
        ↓
OmniphonyStreamAPO.dll
        ↓
stereo sources → protected Current stereo path
multichannel beds → native-bed authored source path
        ↓
omniphony_realtime.dll
        ↓
canonical 8.1.4.4-capable source scene
        ↓
Current 22-direction shell
        ↓
cascaded binaural / measured HRTF
        ↓
listener correction + linked peak guard
        ↓
48 kHz / 32-bit / stereo physical endpoint
```

The endpoint remaining stereo is intentional. A stereo `GetMixFormat` result does not mean the SFX failed to receive richer source input. Production acceptance tests the client boundary directly by asking Windows to support and initialize an exact 48 kHz float32 7.1 shared stream while the DAC remains two-channel.

For conventional games, the intended configuration is:

```text
Windows Spatial Sound: OFF
game mix: Home Theater / surround
in-game Dolby/Sonic/headphone virtualization: OFF
```

That prevents double binaural rendering and leaves Omniphony as the only headphone renderer.

## Installed layout

```text
C:\Program Files\Omniphony\
├─ APO\
│  ├─ OmniphonyAPO.dll
│  ├─ OmniphonyStreamAPO.dll
│  └─ omniphony_realtime.dll
├─ support\
│  ├─ Install-OmniphonyAPO.ps1
│  ├─ Install-OmniphonyWindows.ps1
│  ├─ Uninstall-OmniphonyAPO.ps1
│  ├─ Uninstall-OmniphonyWindows.ps1
│  ├─ OmniphonyApoCtl.exe
│  ├─ OmniphonyMixProbe.exe
│  ├─ OmniphonyEndpointCtl.exe
│  ├─ OmniphonySpatialProbe.exe
│  ├─ OmniphonySpatialProviderProbe.exe
│  └─ OmniphonyTray.ps1
├─ LICENSE
└─ Inno Setup uninstaller files
```

The old virtual-device `driver\` directory and loopback-host `Omniphony.exe` are migration history and are removed during upgrade.

## Two APO roles

Omniphony intentionally retains two Windows APOs with different responsibilities.

### `OmniphonyStreamAPO.dll`

This is the promoted steady-state 0.1 path after successful installation.

It:

- implements `IAudioProcessingObjectPreferredFormatSupport`;
- prefers 7.1 input for a stereo-rendering headphone endpoint;
- preserves stereo Current when the client is stereo;
- routes authored multichannel beds through the native-bed realtime ABI;
- accepts differing input/output channel counts;
- reduces richer source input to stereo before the physical endpoint;
- keeps the physical endpoint at its normal two-channel format.

Stable stream APO CLSID:

```text
{07D403D9-8A98-43EF-8C28-8651756D83BE}
```

### `OmniphonyAPO.dll`

This is the proven stereo Current EFX and rollback floor.

It:

- processes supported stereo float32 graphs;
- provides the recovery path if native-surround promotion fails;
- is attached first during installation so audio has a known-good floor;
- is removed after the stream SFX passes real client-boundary acceptance.

Stable endpoint APO CLSID:

```text
{A9333BFE-39C1-40FD-B4B0-ECC591410B47}
```

The steady-state invariant after a successful native-surround install is:

```text
SFX = OmniphonyStreamAPO
EFX = absent
```

Current must not run in both APOs simultaneously.

## Source authority

The canonical scene remains the 17-lane 8.1.4.4 vocabulary. The 22-direction System-H-derived shell is downstream rendering geometry, not a replacement scene model.

For conventional authored 7.1:

```text
FL FR C LFE SL SR BL BR = AUTHORED
BC TFL TFR TBL TBR BFL BFR BBL BBR = EMPTY unless separately earned
```

For stereo, Current retains its evidence-bounded derived scene support and protected finished-master path.

For authored 7.1.4, the stream APO/native-bed path is already regression-tested with twelve input channels. That remains implementation evidence until a real Windows application is shown opening that exact richer shared stream.

The ideal full static Windows spatial vocabulary remains **8.1.4.4 / 17 positions**. Raw Windows Spatial Audio dynamic objects are richer still because they carry continuous XYZ source positions rather than fixed speaker anchors. Neither raw 8.1.4.4 object ingress nor dynamic-object interception is claimed solved by this conventional SFX baseline.

## Tray contract

The notification-area icon is the only normal UI surface. It currently exposes listener EQ choices and right-ear compensation.

The tray does not host or transport audio. Closing it does not stop Current, because processing remains inside the Windows APO path.

## Unsigned AudioDG compatibility mode

The 0.1 installer intentionally uses the Windows compatibility path required by these unsigned user-mode APOs:

```text
HKLM\SOFTWARE\Microsoft\Windows\CurrentVersion\Audio
DisableProtectedAudioDG = 1
```

The installer snapshots the previous value before changing it. Rollback and uninstall restore that saved state. This is an explicit 0.1 product tradeoff in exchange for Equalizer-APO-style one-click deployment without a Microsoft driver-signing workflow.

## Install transaction

`OmniphonySetup.exe` performs the complete normal installation. The user should not need to run the PowerShell helpers manually.

During install/upgrade it:

1. validates the realtime renderer before endpoint mutation;
2. stops any obsolete Omniphony host/tray instance;
3. repairs known Omniphony global APO registration if recovering an existing endpoint;
4. resolves the current render endpoint and persists its stable identity;
5. snapshots endpoint state and the previous AudioDG protection value;
6. establishes and proves the stereo Current EFX rollback floor;
7. registers the native stream APO;
8. waits for the exact physical endpoint to be ACTIVE;
9. cleans interrupted older SFX state;
10. attaches the Omniphony stream SFX;
11. removes the temporary stereo EFX before graph restart;
12. restarts the Windows audio graph;
13. proves the endpoint remains 48 kHz / 32-bit / stereo;
14. proves an exact 48 kHz / float32 / 7.1 shared client format is supported;
15. proves that 7.1 shared client can actually initialize;
16. keeps the SFX only after those facts are true;
17. otherwise restores the stereo Current EFX and verifies rollback;
18. starts the tray icon after successful setup.

A failed preflight must not deregister a previously working global APO while an endpoint still references it. Rollback success is based on verified restored state, not merely on attempted cleanup commands.

## Endpoint continuity

A USB DAC being powered off, unplugged, or temporarily absent must not be treated as uninstalling Omniphony.

Omniphony persists the verified endpoint identity. When Core Audio temporarily reports no active endpoint, recovery may repair project-owned global APO registration, reassert the known endpoint when appropriate, restart the Windows audio graph, and require the exact endpoint to become ACTIVE before FX mutation continues.

Normal power cycling of the same DAC should therefore preserve the installation. A genuinely new endpoint identity after a driver/topology change may require reattachment.

## Personal output correction

The Omniphony foundation EQ and listener-specific headphone correction are separate layers. The current personal build includes the Noire X correction after the Current master/spatial sum and before the final linked peak guard.

Equalizer APO is **not** a runtime dependency. Omniphony only adopts the same broad unsigned-APO deployment tradeoff.

## Fixed-latency safety lane

The Current realtime path reports a fixed **40 ms / 1920-frame** host delay at 48 kHz. The same timeline is maintained for a delayed-dry safety lane. Worker underruns substitute the matching delayed dry frame rather than jumping forward in time. Late Current frames are discarded before Current resumes.

## Diagnostics

The normal product is the EXE installer, but the retained support helpers can diagnose a machine when needed:

```powershell
OmniphonyApoCtl.exe status
OmniphonyMixProbe.exe "Dan Clark Noire X" FiiO Noire
OmniphonyMixProbe.exe --shared-7.1 "Dan Clark Noire X"
```

Successful promoted native-surround evidence includes:

```text
FX_REGISTRY_VERIFY_OK   SFX   {07D403D9-8A98-43EF-8C28-8651756D83BE}
FX_REGISTRY_VERIFY_OK   EFX   <absent>
SHARED_7_1_FORMAT_SUPPORTED
SHARED_7_1_INITIALIZE_OK
NATIVE_SURROUND_CLIENT_FORMAT_OK INPUT_CHANNELS=8 ENDPOINT_CHANNELS=2
NATIVE_SURROUND_SFX 1
NATIVE_SURROUND_EFX 0
```

The CI/test payload also contains two read-only Spatial Audio research probes:

```powershell
OmniphonySpatialProbe.exe
OmniphonySpatialProviderProbe.exe
```

`OmniphonySpatialProbe.exe` interrogates the active endpoint's public `ISpatialAudioClient` capability: static-object mask/positions, dynamic-object capacity, and supported object format. It does not open another application's stream.

`OmniphonySpatialProviderProbe.exe` observes the currently installed spatial-provider registry surfaces without writing them. Its provider-registry output is experimental evidence only because Microsoft does not document that registry surface as a public third-party provider contract. See `docs/windows-spatial-provider-experiment.md` for the falsifiable experiment ladder.

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
≠ SFX registry attachment succeeds
≠ exact 7.1 shared format reports supported
≠ exact 7.1 shared client Initialize succeeds
≠ a particular game actually opens/populates that 7.1 stream
≠ physical listening confirms the authored-surround result
```

The accepted 2026-08-18 Windows baseline has crossed through the **real 7.1 shared-client Initialize** gate. Game-specific authored-stream behavior and raw Windows Spatial Audio object ingress remain separate evidence layers.
