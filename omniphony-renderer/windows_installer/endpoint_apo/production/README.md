# Production Windows APO packaging

This directory is the production packaging boundary for the retained Omniphony Current renderer.

The goal is deliberately boring installation: Windows DriverStore packages, a device-specific extension generated from machine evidence, protected AudioDG left enabled, transactional rollback, and no ownership edits to another driver's MMDevices registry tree.

The existing Inno Setup path remains a **development bring-up harness**. It is not the production deployment model.

## Production topology

```text
physical audio driver
        │
        ├─ captured hardware ID
        ├─ exact decorated DDInstall section
        ├─ captured topology interface evidence
        └─ read-only existing-EFX snapshot
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

## Capture contract

### `Capture-TargetAudioDriver.ps1`

This is the low-level Windows probe. It resolves the default render MMDevice, walks the PnP parent chain and records MEDIA-class driver candidates, installed INF information and hardware IDs. It remains useful as a diagnostic primitive.

### `Capture-ProductionTarget.ps1`

This is the **production-facing one-command capture**.

```powershell
.\Capture-ProductionTarget.ps1 `
  -EndpointCtl C:\path\to\OmniphonyEndpointCtl.exe `
  -OutputPath .\omniphony-audio-target.json
```

It produces schema:

```text
omniphony.windows.apo-target.v3
```

The wrapper deliberately adds evidence that an ordinary parent/HWID walk cannot safely infer:

- `DEVPKEY_Device_DriverInfSectionExt`, kept separate from the base DDInstall section;
- the exact resolved decorated install section;
- deterministic INF `Include=` / `Needs=` traversal;
- source-INF and section provenance for every captured `AddInterface`;
- paired `KSCATEGORY_AUDIO` + `KSCATEGORY_TOPOLOGY` reference candidates;
- a **read-only** snapshot of legacy/composite endpoint EFX and the enhancements-disabled state.

`capture_target_evidence.py` performs the deterministic INF finalization and is covered by unit tests, including platform decoration and cross-INF `Include/Needs` resolution.

The current development capture wrapper uses Python 3 for that finalization step. The eventual user-facing production EXE must internalize/bundle this logic so Python is not a product dependency.

Do not guess a FiiO, USB-audio, or other HWID or topology reference if capture is ambiguous. Ambiguity is a stop condition.

## Extension generation

### `generate_extension_inf.py`

Consumes only finalized v3 machine evidence and emits `OmniphonyApoExtension.inf`.

```powershell
python .\generate_extension_inf.py `
  .\omniphony-audio-target.json `
  .\OmniphonyApoExtension.inf
```

The generator refuses:

- old v1/v2 captures;
- zero or multiple physical MEDIA association candidates unless a value already present in the capture is selected explicitly;
- MMDevice software-endpoint hardware IDs;
- hardware IDs not present in the capture;
- missing decorated-driver-section evidence;
- unresolved INF evidence warnings;
- invented or ambiguous topology reference strings;
- a topology reference not captured under both `KSCATEGORY_AUDIO` and `KSCATEGORY_TOPOLOGY`;
- an unreadable endpoint-effects snapshot;
- an endpoint with system effects disabled;
- a pre-existing **non-Omniphony EFX**.

Windows supports composite endpoint effects, but Omniphony does not currently guess ordering or silently replace an unknown vendor effect. A collision is therefore a hard blocker until coexistence can be proved for that exact stack.

The generated extension has a deterministic target-specific `ExtensionId`, creates `VEN_OMNI&CID_CURRENT`, and writes only interface-relative `HKR` FX association values. It does not write global `HKLM`/`HKCR` APO state and does not edit MMDevices directly.

## Component package

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

## Package build

### `Build-ProductionApoPackages.ps1`

Builds two independent DriverStore packages around already-built runtime DLLs:

```powershell
.\Build-ProductionApoPackages.ps1 `
  -CaptureJson .\omniphony-audio-target.json `
  -ApoDll C:\path\to\OmniphonyAPO.dll `
  -RealtimeDll C:\path\to\omniphony_realtime.dll `
  -OutputRoot .\omniphony-production-packages
```

The builder:

1. copies the exact v3 witness into the package as `target-capture.json`;
2. generates the target-specific extension from **that bound copy**;
3. runs WDK `InfVerif /w /v` on both packages;
4. optionally signs both PE payloads before catalog generation;
5. immediately verifies the PE signatures;
6. runs `Inf2Cat` independently for the component and extension packages;
7. optionally signs and immediately verifies both catalogs;
8. writes a SHA-256 `package-manifest.json` covering the staged payload, including `target-capture.json`.

The package therefore cannot quietly drift away from the machine witness used to generate its extension.

A locally Valid Authenticode signature is necessary evidence, not proof that protected AudioDG or the intended Windows driver-trust path will accept the candidate. Physical protected-mode loading remains an acceptance gate.

## Read-only machine preflight

### `Test-ProductionMachineReadiness.ps1`

This runs before any production installation attempt:

```powershell
.\Test-ProductionMachineReadiness.ps1 `
  -PackageRoot .\omniphony-production-packages `
  -OutputPath .\omniphony-readiness.json
```

When `-PackageRoot` is supplied, the preflight automatically uses its bound `target-capture.json`.

It checks without installing anything:

- Windows build eligibility;
- whether the old `DisableProtectedAudioDG=1` development bypass is still active;
- Secure Boot / Device Guard observations;
- existing Omniphony DriverStore packages;
- whether the captured endpoint and physical driver are still present;
- whether the live `DriverInfSectionExt` still matches the witness;
- whether the v3 capture has exactly one safe paired topology reference;
- whether endpoint effects are readable and enabled;
- whether a foreign endpoint EFX now exists;
- package-manifest hashes;
- all four required Authenticode signatures;
- whether the package manifest records completed signature verification.

A zero-blocker report is permission to **attempt** the physical test. It is not a claim that AudioDG has loaded Current.

## Production installation

### `Install-ProductionApoPackages.ps1`

Installs through Windows PnP/DriverStore machinery rather than direct endpoint registry mutation:

```powershell
.\Install-ProductionApoPackages.ps1 `
  -PackageRoot .\omniphony-production-packages
```

Before PnPUtil is allowed to change anything, the installer rechecks:

- package hashes;
- the bound target-capture hash;
- all four signatures;
- protected AudioDG bypass state;
- captured endpoint presence;
- captured physical driver presence;
- live hardware-ID overlap with the witness;
- endpoint system-effects state;
- legacy + composite existing EFX collision state.

The endpoint-effects registry is **observed read-only** for these checks. Production never takes ownership of or directly writes the MMDevices tree.

Only after those gates pass does the installer:

- snapshot existing Omniphony driver packages;
- install the APO component and target extension with `pnputil /add-driver ... /install`;
- rescan PnP and restart the audio graph;
- verify that `SWC\VEN_OMNI&CID_CURRENT` exists;
- verify that the endpoint effects property store now exposes the Omniphony EFX CLSID;
- verify that system effects are still enabled;
- record installed package and endpoint state under `%ProgramData%\Omniphony\production`.

If any post-install gate fails, rollback removes only newly-added Omniphony packages, rescans PnP and restarts AudioSrv.

It never removes or replaces the physical audio driver.

### `Uninstall-ProductionApoPackages.ps1`

Enumerates Omniphony's own DriverStore packages using structured PnPUtil output, removes the extension before the component, rescans devices, restarts AudioSrv and verifies that the Omniphony packages are gone.

It does **not** modify the physical audio driver or `DisableProtectedAudioDG`.

## CI contract

Windows CI guards several layers independently:

- source-level isolated component-package contract;
- development/production AudioDG separation;
- read-only production endpoint observation contract;
- Python unit tests for decorated INF evidence and `Include/Needs` traversal;
- Python unit tests for extension selection, anti-guessing and EFX collision behavior;
- PowerShell AST parsing for capture/preflight/build/install/uninstall tooling;
- synthetic finalized v3 witness → generated extension INF;
- WDK `InfVerif /w /v` for both component and extension;
- synthetic full package staging and `Inf2Cat` catalog generation;
- bound-capture manifest hashing;
- APO and realtime DLL builds;
- AudioDG import-table audit.

The synthetic witness exists only to exercise structure in CI. It is not a substitute for physical machine evidence.

## What still requires the real machine

Repository-side packaging can remove guesswork, but it cannot manufacture evidence about the actual installed endpoint. Before the production path is considered physically ready:

1. run `Capture-ProductionTarget.ps1` against the real default output;
2. obtain a clean v3 witness with one physical MEDIA driver and one paired topology reference;
3. build the two package candidates with the intended signing/trust method;
4. obtain a zero-blocker read-only readiness report;
5. install with protected AudioDG enabled;
6. prove APO activation and **Current processing**, not merely CLSID registration;
7. prove `GetMixFormat` and ordinary application playback;
8. prove AudioSrv restart and reboot/sleep-resume;
9. prove upgrade, failed-install rollback and uninstall;
10. retain the canonical Current 8.1.4.4 → 22-direction → binaural DSP contract.

The previous `0x80070005`/`GetMixFormat` development failure is not considered solved merely because the new package structure is cleaner. It is solved only when the protected production path passes the physical endpoint test.

## Development installer relationship

`../Install-OmniphonyAPO.ps1`, `../OmniphonyApoCtl*.cpp` and the current `0.0.4-dev` Inno package remain bring-up tools. Their global registration, endpoint ACL repair and explicitly opted-in unprotected-AudioDG measures must not leak into this production path.

Both paths converge on the same `OmniphonyAPO.dll`, `omniphony_realtime.dll` and Current renderer. Packaging work is not permission to fork or retune the sound.
