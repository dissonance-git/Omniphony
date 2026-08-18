# Optional signed DriverStore APO experiment

This directory contains Omniphony's **optional** componentized/signed Windows APO deployment work.

It is **not** the normal Windows 0.1 product path and it does not block the one-click tray-only installer.

The normal product lives one directory up and uses:

```text
OmniphonySetup.exe
→ unsigned user-mode endpoint EFX APO
→ Windows unprotected-AudioDG compatibility mode
→ current physical render endpoint
→ tray icon
→ headless Current renderer
```

See `../README.md` and `../../../../docs/omniphony-for-windows.md` for the user-facing product contract.

This directory remains useful if Omniphony later wants a Microsoft-trusted DriverStore distribution route with protected AudioDG kept enabled.

## Experimental topology

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

The experiment deliberately avoids direct MMDevices ownership, keeps the physical audio driver intact, and confines rollback/uninstall to Omniphony packages.

## Target capture and FiiO evidence

`Capture-ProductionTarget.ps1` records schema `omniphony.windows.apo-target.v3` with exact endpoint, physical MEDIA-class target, hardware IDs, installed INF/decorated section, interface provenance and read-only endpoint-effect state.

The strongest topology evidence is one reference exposed under both `KSCATEGORY_AUDIO` and `KSCATEGORY_TOPOLOGY`. The observed FiiO K7/Q-series driver also established one supported fallback: literal `Topology` under `KSCATEGORY_AUDIO` only. Omniphony accepts that shape only when captured from the exact installed driver section and never fabricates an unevidenced topology registration.

## Package generation

`generate_extension_inf.py` consumes finalized target evidence and emits the target-specific extension INF. It refuses ambiguous targets, uncaptured IDs/references, unresolved INF warnings, disabled effects, foreign endpoint EFX, and a normal 0.1 Omniphony legacy-slot attachment that could otherwise instantiate Current twice.

`OmniphonyApoComponent.inx` defines the componentized APO as:

```text
Class=AudioProcessingObject
ClassGuid={5989fce8-9cd0-467d-8a6a-5419e31529d4}
component ID: SWC\VEN_OMNI&CID_CURRENT
```

## Signing boundary

`Build-ProductionApoPackages.ps1` builds component and extension packages, validates INFs, creates catalogs, supports local candidate signing and writes a SHA-256 manifest.

Local Authenticode validity is not treated as Microsoft production trust. If externally signed packages are ever used, `Finalize-SignedProductionPackages.ps1` verifies the returned files and regenerates the manifest from those exact bytes.

## Experimental handoff

`Finish-ProductionInstall.ps1` coordinates the optional protected route. It can clean the normal unsigned attachment, capture fresh target evidence and later accept a returned signed package through:

```powershell
.\Finish-ProductionInstall.ps1 -SignedPackageRoot C:\path\to\signed-package
```

This is research/future deployment machinery, **not** the recommended 0.1 installation procedure.

## Readiness and acceptance

`Test-ProductionMachineReadiness.ps1` validates target identity, topology evidence, hashes/signatures, endpoint-effect state and baseline WASAPI behavior. `OmniphonyProductionProbe.exe` performs read-only WASAPI checks on the captured endpoint.

The historical protected-host `0x80070005` failure is relevant to this optional route only. It does not invalidate the normal unsigned 0.1 compatibility-mode product.

## CI value

This optional path remains useful for high-integrity tests of:

- exact Windows target/topology evidence;
- FiiO-style legacy topology support;
- component/extension INF isolation;
- package manifests and signatures;
- protected-AudioDG experimentation;
- transactional PnP rollback/uninstall.

None of those gates should be allowed to reclassify the simple tray-only installer as unfinished.