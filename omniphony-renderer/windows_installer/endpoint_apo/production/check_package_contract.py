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
            f"production APO package must not contain development registration token: {forbidden}"
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

# Production is the inverse contract. It may inspect the bypass in order to
# refuse it, but it must never set the bypass or use the raw MMDevices attach.
production_install = (HERE / "Install-ProductionApoPackages.ps1").read_text(encoding="utf-8")
production_uninstall = (HERE / "Uninstall-ProductionApoPackages.ps1").read_text(encoding="utf-8")
production_combined = (production_install + "\n" + production_uninstall).lower()
if "disableprotectedaudiodg=1 is still active" not in production_combined:
    raise SystemExit("production installer must refuse the unprotected AudioDG development state")
for forbidden in (
    "set-regdword 'software\\microsoft\\windows\\currentversion\\audio' 'disableprotectedaudiodg' 1",
    "attach-id",
    "fxproperties",
    "software\\classes\\audioengine\\audioprocessingobjects",
):
    if forbidden in production_combined:
        raise SystemExit(f"production lifecycle leaked development attach behavior: {forbidden}")

print("PRODUCTION_APO_PACKAGE_CONTRACT_OK 1")
print("WINDOWS_DEPLOYMENT_SEPARATION_CONTRACT_OK 1")
