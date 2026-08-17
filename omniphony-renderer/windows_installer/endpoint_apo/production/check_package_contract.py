from pathlib import Path
import re

INF = Path(__file__).with_name("OmniphonyApoComponent.inx")
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
        raise SystemExit(f"production APO package must not contain development registration token: {forbidden}")

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

print("PRODUCTION_APO_PACKAGE_CONTRACT_OK 1")
