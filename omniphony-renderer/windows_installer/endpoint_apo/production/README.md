# Production Windows APO packaging

This directory is the production deployment boundary for the retained Omniphony Current renderer.

The product contract is intentionally small:

- Current DSP is unchanged by packaging work;
- Windows DriverStore/componentized APO packaging owns deployment;
- protected AudioDG stays enabled;
- the physical audio driver is never replaced or edited;
- endpoint registry state is observed read-only in production;
- rollback/uninstall remove only Omniphony packages;
- user UI remains the small Omniphony tray icon, with no required taskbar window.

The existing Inno/`Install-OmniphonyAPO.ps1` path is a **development bring-up harness**. It is not the production deployment model.

## Production topology

```text
physical MEDIA driver
  ├─ hardware ID
  ├─ installed DDInstall section
  ├─ exact topology interface evidence
  └─ endpoint-effects snapshot
        ↓
target-capture.json (v3)
        ↓
OmniphonyApoExtension.inf
  ├─ AddComponent → VEN_OMNI&CID_CURRENT
  └─ interface-relative EFX association
        ↓
SWC\VEN_OMNI&CID_CURRENT
        ↓
OmniphonyApoComponent.inf
  ├─ Class=AudioProcessingObject
  ├─ isolated HKR COM/AudioEngine registration
  ├─ PETrust
  ├─ OmniphonyAPO.dll
  └─ omniphony_realtime.dll
        ↓
protected AudioDG
        ↓
Current renderer
```

## Real target capture

`Capture-ProductionTarget.ps1` is the production-facing capture command. It wraps `Capture-TargetAudioDriver.ps1`, reads `DEVPKEY_Device_DriverInfSectionExt`, snapshots endpoint effects read-only, and finalizes evidence through the pure-PowerShell `Finalize-TargetEvidence.ps1`.

The capture is schema:

```text
omniphony.windows.apo-target.v3
```

It records the exact default endpoint, physical MEDIA-class candidate, hardware IDs, installed INF and decorated section, `Include=`/`Needs=` traversal, `AddInterface` provenance, interface warnings, and endpoint-effect state.

Capture itself has **no Python dependency**. `capture_target_evidence.py` remains only as a CI/test oracle.

### Safe topology evidence

The strongest case is one reference exposed under both `KSCATEGORY_AUDIO` and `KSCATEGORY_TOPOLOGY`.

A real FiiO K7/Q-series driver established one additional supported WDM pattern: the installed section can expose the literal reference `Topology` under `KSCATEGORY_AUDIO` only. Omniphony accepts that fallback only when it is captured from the exact installed driver section. It does not generalize arbitrary AUDIO references into topology guesses.

For such a legacy layout, the generated extension attaches only to the interface class actually evidenced by the driver. It does not invent a second `KSCATEGORY_TOPOLOGY` registration.

Ambiguity or unresolved INF traversal remains a hard stop.

## Extension generation

`generate_extension_inf.py` consumes finalized v3 evidence and emits `OmniphonyApoExtension.inf`.

It refuses:

- old/unfinalized capture schemas;
- zero or multiple physical MEDIA targets unless disambiguated using values already captured;
- MMDevice software endpoints as hardware targets;
- uncaptured hardware IDs or topology references;
- unresolved INF warnings;
- disabled system effects;
- foreign endpoint EFX;
- a legacy Omniphony development EFX attachment.

The last rule prevents the old dev EFX slot and the production composite EFX from instantiating Current twice.

The extension uses a deterministic target-specific `ExtensionId`, `AddComponent`, and interface-relative `HKR` properties. Production never edits the MMDevices tree directly.

## Component package

`OmniphonyApoComponent.inx` defines the Windows 11 APO component package:

```text
Class=AudioProcessingObject
ClassGuid={5989fce8-9cd0-467d-8a6a-5419e31529d4}
component ID: SWC\VEN_OMNI&CID_CURRENT
```

The APO and realtime DLL are installed together because `OmniphonyAPO.dll` resolves `omniphony_realtime.dll` beside itself. Both AudioDG-loaded binaries are marked with PETrust. The APO is built without an embedded manifest because an embedded manifest can make an APO fail in protected AudioDG.

## Build and signing boundary

`Build-ProductionApoPackages.ps1` creates the component and extension driver packages from a bound v3 target capture. It runs `InfVerif`, generates catalogs with `Inf2Cat`, optionally signs local PEs/catalogs, verifies those signatures, and writes a SHA-256 package manifest.

A locally Valid Authenticode signature is **not** promoted to production trust. Local signing prepares a submission candidate. Microsoft driver signing/certification remains an external trust boundary.

After externally signed packages are returned, `Finalize-SignedProductionPackages.ps1`:

1. verifies the required APO/runtime/catalog/probe signatures;
2. can require a Microsoft identity on both driver catalogs;
3. recomputes hashes from the returned files;
4. rewrites `package-manifest.json` with `SignaturesVerified=true`.

This matters because an external signing service can replace or modify catalogs. A pre-submission manifest must not be reused blindly.

## One-command handoff

`Finish-ProductionInstall.ps1` is the migration coordinator.

### Preparation phase

Run it from an elevated shell with the current artifact/repo available. It:

1. stages the current audible APO/runtime/probe before touching the dev install;
2. runs the existing dev uninstaller when dev state is present;
3. verifies `DisableProtectedAudioDG=1` is gone;
4. captures a fresh clean v3 target witness;
5. with a signing certificate, builds a **submission candidate**;
6. stops at the Microsoft signing boundary instead of pretending local signing is production.

The user's FiiO-specific hardware/INF archaeology does not need to be repeated manually.

### Installation phase

After the externally signed component/extension packages have been reassembled into a package root, run:

```powershell
.\Finish-ProductionInstall.ps1 -SignedPackageRoot C:\path\to\signed-package
```

The script then:

1. finalizes the externally signed package;
2. runs read-only machine readiness;
3. runs the baseline WASAPI/GetMixFormat/shared-render probe;
4. installs with `Install-ProductionApoPackages.ps1` only if all gates pass;
5. runs the production installer's post-install acceptance/rollback logic;
6. relaunches the tray icon if the installed app support files are present.

## Read-only readiness

`Test-ProductionMachineReadiness.ps1` checks only facts needed to attempt the protected transaction:

- Windows 11 APO-class eligibility;
- elevated execution;
- `DisableProtectedAudioDG` development bypass absent;
- live v3 target evidence;
- exactly one safe topology association, including the verified legacy `Topology` pattern;
- hardware-ID overlap and driver-section-extension stability;
- readable/enabled endpoint effects;
- no legacy dev Omniphony EFX and no foreign EFX;
- package hashes;
- **five** locally Valid signatures: APO DLL, realtime DLL, component catalog, extension catalog, production probe;
- manifest signature-verification state;
- baseline read-only WASAPI probe.

Zero blockers means permission to attempt installation. It does not by itself prove that protected AudioDG has loaded Current.

## Production acceptance probe

`OmniphonyProductionProbe.exe` is deliberately separate from `OmniphonyMixProbe.exe`.

The production probe is read-only and tests the exact captured MMDevice through WASAPI, including `GetMixFormat`, shared render initialization/start, buffer progress, and clean stop/reset. It owns no registry/ACL repair path.

The old development `0x80070005`/`GetMixFormat` failure is considered solved only when this probe passes on the real endpoint under the protected production installation.

## Install, rollback, uninstall

`Install-ProductionApoPackages.ps1` uses PnPUtil/DriverStore, revalidates the bound target and package before changing anything, installs component then extension, rescans/restarts the audio graph, verifies the software component and endpoint association, runs the production WASAPI probe, and rolls back newly added Omniphony packages on failure.

`Rollback-ProductionApoPackages.ps1` restores the prior Omniphony package generation when one exists.

`Uninstall-ProductionApoPackages.ps1` removes the extension before the component and never removes the physical audio driver or enables the development AudioDG bypass.

## Tray-only UI

Packaging does not require a new desktop window. The existing `OmniphonyTray.ps1` remains the small user-facing control surface. The production handoff relaunches it after a successful install; normal application packaging can continue to register it at user startup.

## CI contract

Windows CI now protects the path by:

- parsing **all** PowerShell under `windows_installer`;
- unit-testing INF evidence and extension generation;
- covering the observed FiiO-style legacy topology shape;
- rejecting a legacy dev EFX in production capture;
- validating component/extension INF isolation;
- checking the manifest-free APO contract;
- building the read-only production probe;
- packaging the production handoff scripts and probe in the endpoint artifact.

Synthetic evidence exists only for CI. Physical proof still comes from the real machine.

## Final external boundary

Repository-side implementation is complete up to Windows' driver trust boundary. The remaining non-repository step is obtaining the appropriate Microsoft-signed/certified driver packages and feeding the returned files back through `Finalize-SignedProductionPackages.ps1` / `Finish-ProductionInstall.ps1 -SignedPackageRoot ...`.

That boundary must remain explicit. Omniphony must not re-enable `DisableProtectedAudioDG=1` or call a locally signed package “production” merely to make the install appear finished.
