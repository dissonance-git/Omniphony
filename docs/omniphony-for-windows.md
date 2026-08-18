# Omniphony for Windows

This document defines the Windows product boundary for Omniphony.

Omniphony for Windows is **not** a second renderer, a virtual-cable product or a loopback host. It is the Windows operating-system integration of the same portable Omniphony scene and binaural engine used elsewhere.

---

## Product topology

The mature Windows target is endpoint-native:

```text
applications / games / browsers / players
        ↓
Windows Audio Engine
        ↓
Omniphony endpoint effect (EFX APO)
        ↓
physical endpoint driver
        ↓
DAC / headphones
```

Windows owns the system mix. Omniphony owns the final endpoint processing graph. The physical driver owns playback.

The old virtual-device and process-loopback routes are migration history and bounded diagnostics only.

---

## Current product scene

The portable static scene contract is **8.1.4.4 with 17 semantic anchors**:

```text
L R C LFE Ls Rs Lb Rb Cb Tfl Tfr Tbl Tbr Bfl Bfr Bbl Bbr
```

Each anchor may be:

```text
AUTHORED
DERIVED
EMPTY
```

The canonical scene is a semantic vocabulary, not a promise that every source contains seventeen channels.

For stereo-derived Current, only these lanes are populated by evidence:

```text
L R Ls Rs Lb Rb Tfl Tfr Tbl Tbr
```

These remain EMPTY:

```text
C LFE Cb Bfl Bfr Bbl Bbr
```

The canonical scene then feeds the Current **22-direction System-H-derived render shell**, followed by cascaded binaural output.

```text
source / evidence
→ canonical 8.1.4.4 scene
→ 22-direction Current shell
→ binaural renderer
→ stereo physical endpoint
```

The 17-lane scene and 22-direction shell must never be described as the same layer.

---

## Current Windows implementation

The present endpoint APO path is deliberately narrower than the scene model:

```text
CURRENT INTERNAL SCENE
17-lane 8.1.4.4 vocabulary

CURRENT WINDOWS APO INPUT
stereo float32 only

CURRENT WINDOWS APO OUTPUT
stereo float32
```

That means native authored 5.1/7.1 input through the APO is **not yet implemented**, even though the internal scene already has places for richer authored geometry.

Do not describe the current APO as if it already receives 7.1 or 7.1.4 from games.

---

## Realtime architecture

The endpoint effect loads `omniphony_realtime.dll` and talks to it through a narrow ABI.

Current does not run its allocating graph directly inside the AudioDG callback. The callback-facing path uses bounded, preallocated PCM transfer while a dedicated worker owns the Current DSP graph.

Current safety behavior includes:

- preallocated callback-facing rings;
- dedicated worker processing;
- aligned dry fallback;
- non-finite sanitization;
- linked peak safety;
- failure fallback rather than blocking AudioDG;
- explicit create/destroy lifecycle tests;
- import-table, manifest and ABI checks in CI.

The callback must never become a place for filesystem I/O, network activity, device discovery, unbounded locks or research-time analysis.

---

## Development attach versus production package

The repository intentionally contains two different Windows deployment stages.

### Development / bring-up path

The `0.0.4-dev` installer remains a physical bring-up harness for raw endpoint attachment. It uses global development registration and may temporarily disable protected AudioDG.

That behavior is now explicit rather than ambient:

```text
Install-OmniphonyAPO.ps1
→ refuses the bypass by default
→ requires -AllowUnprotectedAudioDG

OmniphonySetup.exe 0.0.4-dev
→ passes -AllowUnprotectedAudioDG explicitly
→ therefore remains visibly classified as development-only
```

Rollback/uninstall state handling still restores the prior AudioDG-protection value. The development path is useful for diagnosis, but it is not production evidence.

### Production path implemented in the repository

Production uses the componentized driver model and protected Windows audio path.

The repository now contains an end-to-end production candidate toolchain:

```text
windows_installer/endpoint_apo/production/
  OmniphonyApoComponent.inx
  Capture-TargetAudioDriver.ps1
  generate_extension_inf.py
  Build-ProductionApoPackages.ps1
  Install-ProductionApoPackages.ps1
  Uninstall-ProductionApoPackages.ps1
  check_package_contract.py
  test_generate_extension_inf.py
  testdata/target-audio-synthetic.json
  README.md
```

The production component package establishes:

- `Class=AudioProcessingObject`;
- `SWC\VEN_OMNI&CID_CURRENT` component identity;
- DriverStore payload placement;
- HKR-local COM/APO registration;
- PETrust declarations;
- no global `HKLM`/`HKCR` APO registration;
- no raw MMDevices attachment in the production INF.

The production extension generator now establishes the device-specific side from captured evidence:

- exact physical MEDIA-class hardware ID;
- exact captured topology reference;
- `AddComponent` for `VEN_OMNI&CID_CURRENT`;
- paired `KSCATEGORY_AUDIO` / `KSCATEGORY_TOPOLOGY` interface extension;
- interface-relative EFX association to the Omniphony CLSID;
- default EFX processing-mode declaration;
- deterministic target-specific `ExtensionId`;
- refusal of ambiguous or invented target data.

The package builder validates both INFs, stages independent component and extension packages, generates catalogs, optionally signs PE/catalog payloads, and records SHA-256 package manifests.

The production installer and uninstaller use Windows PnP/DriverStore servicing rather than direct endpoint registry ownership. The installer refuses to run while `DisableProtectedAudioDG=1`, verifies its staged manifest, installs only Omniphony packages, checks that `SWC\VEN_OMNI&CID_CURRENT` appears, and rolls back newly-added Omniphony packages on failure.

### What still separates repository completeness from physical readiness

The remaining boundary is **machine evidence and protected-mode proof**, not another speculative installer architecture:

```text
capture actual target-driver + topology identity
→ generate exact extension from that witness
→ build/sign the two DriverStore packages
→ install with protected AudioDG
→ prove APO activation and Current processing
→ prove playback / restart / sleep / upgrade / rollback / uninstall
```

Do not invent a hardware ID or topology reference in source control. The target identity must come from the real machine.

---

## Target-driver capture

`production/Capture-TargetAudioDriver.ps1` is a read-only discovery tool for the target physical render device.

It now writes schema `omniphony.windows.apo-target.v2` and records more than a parent HWID. It:

- resolves the default MMDevice;
- maps it into the PnP tree by endpoint identity rather than friendly name;
- walks parent devices;
- narrows association candidates to MEDIA-class nodes with hardware IDs;
- records installed driver INF path and section;
- parses captured `AddInterface` evidence from that INF;
- records audio/topology reference candidates.

The JSON is the source of truth for `generate_extension_inf.py`.

The generator refuses ambiguous candidates. If a real driver inherits interface declarations through an INF include/needs chain that the current capture parser cannot resolve, the correct response is to improve capture resolution, not guess the topology string.

---

## Production package lifecycle

The repository-side candidate flow is now:

```text
Capture-TargetAudioDriver.ps1
        ↓
omniphony-audio-target.json
        ↓
generate_extension_inf.py
        ↓
OmniphonyApoExtension.inf
        ↓
Build-ProductionApoPackages.ps1
        ├→ component package + catalog
        ├→ extension package + catalog
        └→ SHA-256 package-manifest.json
        ↓
Install-ProductionApoPackages.ps1
        ↓
Windows DriverStore / PnP
        ↓
SWC\VEN_OMNI&CID_CURRENT
        ↓
protected AudioDG physical proof
```

Uninstall reverses only Omniphony's own extension/component packages. It must never remove the physical audio driver.

Signing support in the builder creates a candidate package. It does not, by itself, prove that the intended certificate/trust route is accepted by protected AudioDG on the physical system.

---

## Native multichannel frontier

A stereo physical headphone endpoint should not force Omniphony to lose authored 5.1/7.1 geometry before rendering.

The target architecture is:

```text
authored 5.1 / 7.1 PCM
        ↓
Windows host exposes richer bed to Omniphony
        ↓
matching channels become AUTHORED canonical anchors
        ↓
remaining anchors stay EMPTY or bounded DERIVED
        ↓
Current / future renderer
        ↓
stereo headphones
```

The implementation must preserve real channel truth and avoid this anti-pattern:

```text
7.1 source
→ Windows/downstream stereo collapse
→ stereo inference
→ attempt to reconstruct the discarded 7.1
```

Native multichannel ingress is a host/API problem, not a reason to change the canonical scene.

---

## Windows Spatial Audio and object frontier

The endpoint EFX path and the Windows Spatial Audio object path are different host layers.

The repository does **not** currently prove that an arbitrary third-party endpoint EFX can receive another application's raw static/dynamic object identities and positions before the platform spatial renderer consumes them.

Therefore keep the states separate:

```text
IMPLEMENTED NOW
system-wide endpoint EFX
stereo Current input
canonical internal scene
own binaural renderer

NEXT CONVENTIONAL INGEST FRONTIER
native authored 5.1 / 7.1 PCM preservation

RICH INGRESS FRONTIER
static height/object geometry
dynamic x/y/z objects
platform-supported scene seam still to prove
```

Do not hook or inject into games to recover spatial metadata. Anti-cheat and Windows integrity boundaries outrank convenience.

---

## Dolby and already-binaural behavior

There are two distinct compatibility cases.

### Omniphony owns rendering

If a supported host seam exposes authored bed/object truth before final binauralization:

```text
source spatial truth
→ Omniphony canonical scene
→ Omniphony binaural renderer
```

### Another renderer already owns rendering

If Dolby/Windows/another system has already produced binaural stereo:

```text
already-binaural stereo
→ Omniphony spatial bypass
or explicitly validated non-spatial correction only
```

Do not double-virtualize an already-binaural headphone render.

Stereo channel count alone is insufficient to identify this case. Automatic bypass requires trustworthy host state or an explicit mode signal.

---

## Source authority

Moving into an EFX does not change the fidelity laws.

```text
stereo
→ preserve master + bounded DERIVED support

5.1 / 7.1
→ preserve AUTHORED directional channels when host support exists

height / objects
→ preserve supplied geometry when a supported seam exposes it

already-binaural
→ no blind second HRTF pass
```

A classifier may provide evidence. It does not create authorship.

---

## Headless product law

Normal use should feel like ordinary Windows audio with a better endpoint renderer.

```text
physical output remains selected
Omniphony is automatically present in the endpoint graph
no virtual-device routing
no loopback capture
no console
no helper that must stay open
no daily device setup
```

The optional tray is preference/configuration surface only. Exiting it must not stop the endpoint renderer.

---

## Health and validation

A useful Windows health check should be able to prove independent layers:

```text
package present
→ APO registered in the intended scope
→ endpoint association is the expected one
→ Windows loaded the APO
→ expected format was negotiated
→ realtime callbacks occurred
→ non-silent input reached Current
→ finite non-silent output left Current
```

For production, additionally prove:

```text
protected AudioDG remains enabled
component + extension packages are trusted
no global registry takeover
physical playback survives AudioSrv restart
physical playback survives reboot / sleep-resume
upgrade replaces only Omniphony packages
failed install rolls back only newly-added Omniphony packages
uninstall leaves the physical driver intact
```

For future rich ingress, additionally prove the actual source/scene seam and whether Omniphony or another renderer owns final binauralization.

Installation success is not listening success. Physical listening remains the final audible gate.

---

## Research-informed Windows priorities

Binaural research suggests that Windows integration should preserve the cues required to test externalization rather than flattening them prematurely:

- frontal/rear externalization can remain difficult even with nominal HRTF rendering;
- binaural room cues and interaural structure can affect externalization;
- timbral coloration and localization should be tested together;
- head-tracked world stability is a meaningful future capability;
- richer authored source truth should reduce inference rather than increase it.

These findings support the product architecture, but they do not convert unimplemented Windows host capabilities into claims of support.

---

## Installer contract

The user-facing goal remains one boring installer executable:

```text
OmniphonySetup.exe
→ one elevation
→ capture/resolve the selected physical render device
→ install trusted component + extension packages
→ verify protected-mode health
→ rollback on failure
```

The repository now has the production capture, generation, package-build, install and uninstall machinery underneath that future EXE. The current `0.0.4-dev` EXE is still a development artifact because the **real target capture, final trust/signing path, and protected physical endpoint proof have not yet been completed together**.

The production EXE is earned only after those physical gates pass. Wrapping unproven packages in a nicer wizard would make the failure prettier, not make the deployment ready.

---

## Portable core versus Windows adapter

```text
PORTABLE CORE
source authority
canonical scene
stereo evidence
Current support mapping
HRTF / ITD / distance / room
binaural rendering
validation

WINDOWS ADAPTER
EFX APO host
format negotiation
device association
configuration
installer / servicing
production driver package
rich source ingress when supported
```

The host seam may evolve. The scene and renderer should not fork into a Windows-only audio engine.

---

## Definition of Windows success

The Windows build is finished only when ordinary system audio can use Omniphony headlessly, safely and repeatably on the physical endpoint while preserving source truth, surviving normal OS/device lifecycle events and producing the same Current spatial contract the portable tests validate.
