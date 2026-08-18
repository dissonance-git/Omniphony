# Production Windows APO packaging

This directory is the production packaging boundary for the retained Omniphony Current renderer.

The goal is deliberately boring installation: Windows DriverStore packages, a device-specific extension generated from machine evidence, protected AudioDG left enabled, transactional rollback, and no ownership edits to another driver's MMDevices registry tree.

The existing Inno Setup path remains a **development bring-up harness**. It is not the production deployment model.

## Production topology

```text
physical audio driver
        │
        ├─ captured hardware ID + topology interface evidence
        │
        ▼
OmniphonyApoExtension.inf
        │
        ├─ AddComponent → VEN_OMNI&CID_CURRENT
        └─ extends the captured audio/topology interface
             └─ registers Omniphony as endpoint EFX
        │
        ▼
SWC\VEN_OMNI&CID_CURRENT
        │
        ▼
OmniphonyApoComponent.inf
        │
        ├─ isolated HKR COM registration
        ├─ isolated HKR AudioEngine registration
        ├─ OmniphonyAPO.dll
        └─ omniphony_realtime.dll
```

`AddComponent` is necessary but is not treated as sufficient. The extension also has to associate the Omniphony CLSID with the correct topology interface so the endpoint builder can actually select it as EFX.

## Files

### `OmniphonyApoComponent.inx`

Defines the APO software-component package:

```text
Class=AudioProcessingObject
ClassGuid={5989fce8-9cd0-467d-8a6a-5419e31529d4}
component ID: SWC\VEN_OMNI&CID_CURRENT
DriverStore payload:
  OmniphonyAPO.dll
  omniphony_realtime.dll
registration:
  HKR\Classes\CLSID\...
  HKR\AudioEngine\AudioProcessingObjects\...
PETrust:
  both AudioDG-loaded DLLs
```

The realtime DLL stays beside the APO DLL because the APO resolves `omniphony_realtime.dll` relative to its own module path.

### `Capture-TargetAudioDriver.ps1`

Creates `omniphony.windows.apo-target.v2` evidence from the real default render endpoint. It:

- resolves the default MMDevice by exact endpoint identity;
- maps it to the `AudioEndpoint` PnP node without friendly-name guessing;
- walks the physical parent chain;
- narrows association candidates to MEDIA-class driver nodes with hardware IDs;
- records driver INF path and installed section;
- reads the installed INF for `AddInterface` evidence;
- records captured audio/topology reference strings.

It is read-only apart from writing the JSON witness.

```powershell
.\Capture-TargetAudioDriver.ps1 `
  -EndpointCtl C:\path\to\OmniphonyEndpointCtl.exe `
  -OutputPath .\omniphony-audio-target.json
```

Do not guess a FiiO, USB-audio, or other HWID if capture is ambiguous. Ambiguity is a stop condition, not a prompt to make the INF look complete.

### `generate_extension_inf.py`

Consumes the machine witness and emits `OmniphonyApoExtension.inf`.

```powershell
python .\generate_extension_inf.py `
  .\omniphony-audio-target.json `
  .\OmniphonyApoExtension.inf
```

The generator refuses:

- zero or multiple physical MEDIA association candidates unless a value already present in the capture is selected explicitly;
- MMDevice software-endpoint hardware IDs;
- hardware IDs not present in the capture;
- invented topology reference strings;
- ambiguous topology references unless the chosen reference was actually captured as both `KSCATEGORY_AUDIO` and `KSCATEGORY_TOPOLOGY`.

The generated extension has a deterministic target-specific `ExtensionId`, creates `VEN_OMNI&CID_CURRENT`, and writes only interface-relative `HKR` FX association values. It does not write global `HKLM`/`HKCR` APO state and does not edit MMDevices directly.

### `Build-ProductionApoPackages.ps1`

Builds the two independent DriverStore packages around already-built runtime DLLs:

```powershell
.\Build-ProductionApoPackages.ps1 `
  -CaptureJson .\omniphony-audio-target.json `
  -ApoDll C:\path\to\OmniphonyAPO.dll `
  -RealtimeDll C:\path\to\omniphony_realtime.dll `
  -OutputRoot .\omniphony-production-packages
```

The builder:

1. generates the target-specific extension INF;
2. runs WDK `InfVerif /w /v` on both packages;
3. optionally signs both PE payloads first when `-CertificateThumbprint` is supplied;
4. runs `Inf2Cat` independently for the component and extension packages;
5. optionally signs both catalogs after they are generated;
6. writes a SHA-256 `package-manifest.json` covering the staged payload.

The default Inf2Cat target list covers Windows 11 x64 21H2, 22H2, 24H2 and 25H2 identifiers supported by current WDK tooling. A different explicit list can be supplied with `-Inf2CatOs`.

A locally signed package is still a **candidate**, not proof that protected AudioDG will accept it. Physical protected-mode loading remains an acceptance gate.

### `Install-ProductionApoPackages.ps1`

Installs through Windows PnP/DriverStore machinery rather than direct endpoint registry mutation:

```powershell
.\Install-ProductionApoPackages.ps1 `
  -PackageRoot .\omniphony-production-packages
```

The installer:

- requires elevation;
- verifies every staged file against `package-manifest.json`;
- refuses to run while `DisableProtectedAudioDG=1` is active;
- snapshots existing Omniphony driver packages;
- installs the APO component and target extension with `pnputil /add-driver ... /install`;
- rescans PnP and restarts the audio graph;
- verifies that `SWC\VEN_OMNI&CID_CURRENT` exists;
- records installed package state under `%ProgramData%\Omniphony\production`;
- removes only newly added Omniphony packages if installation fails.

It never removes or replaces the physical audio driver.

### `Uninstall-ProductionApoPackages.ps1`

Enumerates Omniphony's own DriverStore packages using structured PnPUtil output, removes the extension before the component, rescans devices, restarts AudioSrv and verifies that the Omniphony packages are gone.

It does **not** modify the physical audio driver or `DisableProtectedAudioDG`.

## CI contract

Windows CI now guards several layers independently:

- source-level isolated component-package contract;
- Python unit tests for extension selection and anti-guessing behavior;
- PowerShell AST parsing for capture/build/install/uninstall tooling;
- synthetic machine witness → generated extension INF;
- WDK `InfVerif /w /v` for both component and extension;
- synthetic full package staging and `Inf2Cat` catalog generation;
- APO and realtime DLL builds;
- AudioDG import-table audit.

The synthetic witness exists only to exercise structure in CI. It is not a substitute for the physical machine capture.

## What still requires the real machine

Repository-side packaging can remove guesswork, but it cannot manufacture evidence about the actual installed endpoint. Before the production path is considered physically ready:

1. run `Capture-TargetAudioDriver.ps1` against the real default output;
2. confirm the generator resolves one real MEDIA driver and one captured topology reference;
3. build the two package candidates with the intended signing method;
4. install with protected AudioDG enabled;
5. prove APO activation and Current processing on the real endpoint;
6. prove `GetMixFormat` and ordinary application playback;
7. prove AudioSrv restart and reboot/sleep-resume;
8. prove upgrade, failed-install rollback and uninstall;
9. retain the canonical Current 8.1.4.4 → 22-direction → binaural DSP contract.

The previous `0x80070005`/`GetMixFormat` development failure is not considered solved merely because the new package structure is cleaner. It is solved only when the protected production path passes the physical endpoint test.

## Development installer relationship

`../Install-OmniphonyAPO.ps1`, `../OmniphonyApoCtl*.cpp` and the current `0.0.4-dev` Inno package remain bring-up tools. Their global registration, endpoint ACL repair and unprotected-AudioDG measures must not leak into this production path.

Both paths converge on the same `OmniphonyAPO.dll`, `omniphony_realtime.dll` and Current renderer. Packaging work is not permission to fork or retune the sound.
