# Production Windows APO packaging

This directory is the production packaging boundary for the retained Omniphony Current renderer.

It is deliberately separate from the current Inno Setup development installer. The development path proves endpoint attachment and physical listening quickly; the production path must satisfy the Windows protected-AudioDG and driver-package-isolation model without global test switches or direct ownership edits to another driver's endpoint registry tree.

## What is complete here

`OmniphonyApoComponent.inx` defines the Omniphony APO software component itself:

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
PETrust signature attributes:
  both runtime DLLs
```

The runtime DLL remains beside the APO DLL because `OmniphonyAPO.dll` deliberately resolves `omniphony_realtime.dll` relative to its own module path.

`check_package_contract.py` locks the source-level isolation contract. Windows CI additionally copies the template to an `.inf` and runs WDK `InfVerif /w` before auditing the PE import tables of both AudioDG-loaded DLLs.

## Capture the real target driver

`Capture-TargetAudioDriver.ps1` turns the remaining device-specific association into a machine witness instead of a guessed HWID.

With an installed development build:

```powershell
Set-ExecutionPolicy -Scope Process Bypass
.\Capture-TargetAudioDriver.ps1
```

Or point it at a freshly built helper:

```powershell
.\Capture-TargetAudioDriver.ps1 `
  -EndpointCtl C:\path\to\OmniphonyEndpointCtl.exe `
  -OutputPath .\omniphony-audio-target.json
```

The script is read-only apart from writing the JSON result. It resolves the Windows default render MMDevice, maps it to the matching `AudioEndpoint` PnP node by endpoint identity rather than friendly-name guessing, walks its parent chain, and records available hardware IDs, compatible IDs, driver INF, service, provider, version, class and manufacturer.

The resulting `AssociationCandidates` are evidence for choosing the production extension-INF target. The capture is not permission to blindly use the first string returned; the chosen node still needs to correspond to the actual audio driver that can own the APO software component.

## What must remain explicit

A component INF does not decide which physical audio driver owns the APO. The missing production piece is an extension/package association for the captured audio driver that creates the Omniphony software component, conceptually:

```text
[target audio driver extension install]
        ↓
AddComponent = OmniphonyCurrent,,OmniphonyCurrent_AddComponent
        ↓
ComponentIDs = VEN_OMNI&CID_CURRENT
        ↓
SWC\VEN_OMNI&CID_CURRENT
        ↓
OmniphonyApoComponent.inf
```

The target audio hardware/driver identity must come from the real installed endpoint. Do not guess a FiiO, USB-audio, or other HWID in source control merely to make the INF look complete.

## Production acceptance gates

A package is not production-ready until all of these are true:

1. `Capture-TargetAudioDriver.ps1` records the real target physical audio-driver identity;
2. the extension association is defined against the correct captured identity using a supported Windows driver-package path;
3. the component INF passes current WDK `InfVerif /w` validation;
4. the catalog and both PE payload DLLs are signed for the intended deployment mode;
5. both AudioDG-loaded DLL import tables satisfy the package/runtime dependency contract;
6. the APO loads with protected AudioDG enabled, with no `DisableProtectedAudioDG` override;
7. no global HKCR/HKLM APO registration or MMDevices ACL takeover is needed;
8. the real endpoint survives attach, audio-service restart, `GetMixFormat`, ordinary application playback, sleep/resume, upgrade, rollback and uninstall;
9. the Current DSP path still passes the canonical 8.1.4.4 -> 22-direction -> binaural contract tests.

## Development installer relationship

`../Install-OmniphonyAPO.ps1` and `../OmniphonyApoCtl*.cpp` remain useful bring-up machinery. They should be treated as a development harness while the production component package is completed.

The production package must converge on the same DSP binaries and the same Current renderer. Packaging changes are not permission to fork the sound or create a second renderer.
