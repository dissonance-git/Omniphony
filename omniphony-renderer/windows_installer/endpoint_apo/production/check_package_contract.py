from pathlib import Path
import re

HERE = Path(__file__).resolve().parent
ENDPOINT_APO = HERE.parent
WINDOWS_INSTALLER = ENDPOINT_APO.parent

INF = HERE / "OmniphonyApoComponent.inx"
text = INF.read_text(encoding="utf-8")
normalized = text.replace(" ", "").lower()

required = {
    "windows 11 APO class": "class=audioprocessingobject",
    "Windows 11 APO class GUID": "classguid={5989fce8-9cd0-467d-8a6a-5419e31529d4}",
    "Omniphony component ID": r"swc\ven_omni&cid_current",
    "APO payload": "omniphonyapo.dll",
    "Current realtime payload": "omniphony_realtime.dll",
    "driver-store destination": "apo_copyfiles=13",
    "isolated COM registration": r"hkr,classes\clsid\%omniphony_apo_clsid%",
    "isolated audio-engine registration": r"hkr,audioengine\audioprocessingobjects\%omniphony_apo_clsid%",
    "system-effects interface": "{fd7f2b29-24d0-4b5c-b177-592c39f9ca10}",
    "PETrust declaration": "petrust=true",
}

missing = [label for label, needle in required.items() if needle.lower() not in normalized]
if missing:
    raise SystemExit("production APO package is missing: " + ", ".join(missing))

for forbidden in ("hklm,", "hkcr,", "disableprotectedaudiodg", "fxproperties"):
    if forbidden in normalized:
        raise SystemExit(
            f"production APO component INF must not contain development registration token: {forbidden}"
        )

copy_section = re.search(r"\[apo_copyfiles\](.*?)(?:\n\[|\Z)", text, re.IGNORECASE | re.DOTALL)
if not copy_section:
    raise SystemExit("Apo_CopyFiles section missing")
files = {
    line.strip().lower()
    for line in copy_section.group(1).splitlines()
    if line.strip() and not line.lstrip().startswith(";")
}
if files != {"omniphonyapo.dll", "omniphony_realtime.dll"}:
    raise SystemExit(f"unexpected production APO payload: {sorted(files)}")

# The old raw endpoint attach remains a development harness, but weakening
# AudioDG protection must never be an invisible side effect of invoking the
# PowerShell script by hand. The packaged dev EXE opts in explicitly by name.
dev_install = (ENDPOINT_APO / "Install-OmniphonyAPO.ps1").read_text(encoding="utf-8")
dev_setup = (WINDOWS_INSTALLER / "OmniphonySetup.iss").read_text(encoding="utf-8")
if "[switch]$AllowUnprotectedAudioDG" not in dev_install:
    raise SystemExit("development installer lost explicit AllowUnprotectedAudioDG switch")
if "if (-not $AllowUnprotectedAudioDG)" not in dev_install:
    raise SystemExit("development installer no longer refuses implicit AudioDG bypass")
if "DisableProtectedAudioDG' 1" not in dev_install:
    raise SystemExit("development installer contract changed unexpectedly; review bypass handling")
if "-AllowUnprotectedAudioDG" not in dev_setup:
    raise SystemExit("0.0.4-dev Inno package must explicitly opt into its bring-up AudioDG bypass")
if "status-id $EndpointId.Id" in dev_install:
    raise SystemExit("development status helper dereferences a string endpoint ID")

# Production is the inverse contract. It may READ MMDevices/FxProperties to
# prove that it will not overwrite another EFX, but direct endpoint writes and
# raw attach remain forbidden. DriverStore/PnP owns all production changes.
production_install = (HERE / "Install-ProductionApoPackages.ps1").read_text(encoding="utf-8")
production_uninstall = (HERE / "Uninstall-ProductionApoPackages.ps1").read_text(encoding="utf-8")
production_build = (HERE / "Build-ProductionApoPackages.ps1").read_text(encoding="utf-8")
production_readiness = (HERE / "Test-ProductionMachineReadiness.ps1").read_text(encoding="utf-8")
production_generator = (HERE / "generate_extension_inf.py").read_text(encoding="utf-8")
production_capture = (HERE / "Capture-ProductionTarget.ps1").read_text(encoding="utf-8")
production_probe = (ENDPOINT_APO / "OmniphonyProductionProbe.cpp").read_text(encoding="utf-8")
cmake = (ENDPOINT_APO / "CMakeLists.txt").read_text(encoding="utf-8")
production_combined = (production_install + "\n" + production_uninstall).lower()

if "disableprotectedaudiodg=1 is still active" not in production_combined:
    raise SystemExit("production installer must refuse the unprotected AudioDG development state")
if "fxproperties" not in production_install.lower():
    raise SystemExit("production installer must retain read-only endpoint EFX collision observation")
if "already has non-omniphony efx registered" not in production_install.lower():
    raise SystemExit("production installer must refuse foreign endpoint EFX collision")
if "endpoint_efx_association_ok" not in production_install.lower():
    raise SystemExit("production installer must verify endpoint EFX association after PnP install")
if "target-capture.json" not in production_build.lower():
    raise SystemExit("production package builder must bind the exact target capture into the package")
if "omniphonyproductionprobe.exe" not in production_build.lower():
    raise SystemExit("production package builder must carry the read-only WASAPI acceptance probe")
if "omniphony.windows.apo-package-build.v2" not in production_build:
    raise SystemExit("production package builder must emit the probe-aware v2 manifest")
if "signaturesverified" not in production_build.lower() or "manifest.signaturesverified" not in production_install.lower():
    raise SystemExit("production package build/install must preserve the verified-signature gate")
if "omniphony.windows.apo-target.v3" not in production_generator:
    raise SystemExit("production extension generator must require finalized v3 target evidence")
if "DEVPKEY_Device_DriverInfSectionExt" not in production_capture:
    raise SystemExit("production target capture must record the installed INF platform section extension")

# Production success now requires a real WASAPI transaction on the exact
# captured endpoint. The historical 0x80070005 failure happened at
# IAudioClient::GetMixFormat, so registration-only success is not sufficient.
probe_required = (
    "GetMixFormat",
    "IAudioClient::Initialize",
    "IAudioClient::Start",
    "OMNIPHONY_PRODUCTION_WASAPI_PROBE_OK",
    "CURRENT_MIX_CONTRACT_OK",
)
for token in probe_required:
    if token not in production_probe:
        raise SystemExit(f"production WASAPI probe lost acceptance stage: {token}")
for forbidden in (
    "RegSetValue",
    "RegCreateKey",
    "SetNamedSecurityInfo",
    "FxProperties",
    "RepairEndpointApo",
):
    if forbidden.lower() in production_probe.lower():
        raise SystemExit(f"production WASAPI probe gained repair/mutation surface: {forbidden}")
if "add_executable(OmniphonyProductionProbe" not in cmake:
    raise SystemExit("CMake no longer builds OmniphonyProductionProbe")
if "OmniphonyMixProbe" in production_install:
    raise SystemExit("production installer must not invoke the development repair-capable MixProbe")
if "OMNIPHONY_PRODUCTION_WASAPI_PROBE_OK" not in production_install:
    raise SystemExit("production installer must gate success on the safe WASAPI probe")
if "BaselineWasapiProbe" not in production_readiness:
    raise SystemExit("production readiness must prove the same endpoint works before installation")

for forbidden in (
    "set-regdword 'software\\microsoft\\windows\\currentversion\\audio' 'disableprotectedaudiodg' 1",
    "attach-id",
    "software\\classes\\audioengine\\audioprocessingobjects",
    "new-itemproperty",
    "set-itemproperty",
    "remove-itemproperty",
    ".setvalue(",
    ".deletevalue(",
):
    if forbidden in production_combined:
        raise SystemExit(f"production lifecycle leaked direct registry/attach behavior: {forbidden}")

print("PRODUCTION_APO_PACKAGE_CONTRACT_OK 1")
print("WINDOWS_DEPLOYMENT_SEPARATION_CONTRACT_OK 1")
print("PRODUCTION_READ_ONLY_ENDPOINT_OBSERVATION_OK 1")
print("PRODUCTION_WASAPI_ACCEPTANCE_CONTRACT_OK 1")
