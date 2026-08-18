from __future__ import annotations

import argparse
import json
from pathlib import Path
import re
import uuid

MEDIA_CLASS_GUID = "{4d36e96c-e325-11ce-bfc1-08002be10318}"
KSCATEGORY_AUDIO = "{6994ad04-93ef-11d0-a3cc-00a0c9223196}"
KSCATEGORY_TOPOLOGY = "{dda54a40-1e4c-11d1-a050-405705c10000}"
EXTENSION_NAMESPACE = uuid.UUID("b95f0c5a-51cf-45af-94c5-0feeaec3f108")
APO_CLSID = "{A9333BFE-39C1-40FD-B4B0-ECC591410B47}"

HARDWARE_ID_RE = re.compile(r"^[A-Za-z0-9_&\\.\-{}]+$")
REFERENCE_RE = re.compile(r"^[A-Za-z0-9_{}.&\-]+$")


class ContractError(ValueError):
    pass


def _norm(value: object) -> str:
    return str(value or "").strip().lower()


def _candidate_key(candidate: dict) -> str:
    return _norm(candidate.get("InstanceId"))


def _select_candidate(capture: dict, instance_id: str | None, hardware_id: str | None) -> dict:
    candidates = list(capture.get("AssociationCandidates") or [])
    media = [
        c for c in candidates
        if _norm(c.get("ClassGuid")) == MEDIA_CLASS_GUID or _norm(c.get("Class")) == "media"
    ]
    if media:
        candidates = media
    if instance_id:
        wanted = _norm(instance_id)
        candidates = [c for c in candidates if _candidate_key(c) == wanted]
    if hardware_id:
        wanted = _norm(hardware_id)
        candidates = [c for c in candidates if wanted in {_norm(x) for x in (c.get("HardwareIds") or [])}]
    if len(candidates) != 1:
        details = ", ".join(str(c.get("InstanceId") or "<unknown>") for c in candidates) or "<none>"
        raise ContractError(
            "capture must resolve to exactly one physical MEDIA-class association "
            f"candidate; got {len(candidates)}: {details}. "
            "Use --instance-id or --hardware-id only with values present in the capture."
        )
    return candidates[0]


def _select_hardware_id(candidate: dict, explicit: str | None) -> str:
    ids = [str(x).strip() for x in (candidate.get("HardwareIds") or []) if str(x).strip()]
    if explicit:
        if _norm(explicit) not in {_norm(x) for x in ids}:
            raise ContractError("--hardware-id is not present on the selected captured driver")
        selected = explicit.strip()
    elif ids:
        selected = ids[0]
    else:
        raise ContractError("selected driver has no captured hardware IDs")
    if selected.upper().startswith("SWD\\MMDEVAPI\\"):
        raise ContractError("refusing to target an MMDevice software endpoint")
    if not HARDWARE_ID_RE.fullmatch(selected):
        raise ContractError(f"hardware ID contains unsupported INF characters: {selected!r}")
    return selected


def _reference_categories(candidate: dict) -> dict[str, tuple[str, set[str]]]:
    by_ref: dict[str, tuple[str, set[str]]] = {}
    for item in candidate.get("DriverInterfaces") or []:
        ref = str(item.get("ReferenceResolved") or "").strip()
        category = _norm(item.get("CategoryResolved"))
        if not ref or category not in {KSCATEGORY_AUDIO, KSCATEGORY_TOPOLOGY}:
            continue
        key = _norm(ref)
        if key not in by_ref:
            by_ref[key] = (ref, set())
        by_ref[key][1].add(category)
    return by_ref


def _safe_reference_candidates(candidate: dict) -> list[tuple[str, set[str], str]]:
    refs = _reference_categories(candidate)
    paired = [
        (display, categories, "paired-audio-topology")
        for display, categories in refs.values()
        if KSCATEGORY_AUDIO in categories and KSCATEGORY_TOPOLOGY in categories
    ]
    if paired:
        return sorted(paired, key=lambda row: row[0].lower())

    # Legacy WDM audio drivers can register their topology miniport only under
    # KSCATEGORY_AUDIO. Accept only the standard literal Topology reference,
    # never a guessed wave/render reference. This matches Microsoft's WDM INF
    # example and is independently observable through the device interface list.
    legacy = [
        (display, categories, "legacy-kscategory-audio-topology")
        for display, categories in refs.values()
        if KSCATEGORY_AUDIO in categories and display.lower() == "topology"
    ]
    return sorted(legacy, key=lambda row: row[0].lower())


def _select_topology_reference(candidate: dict, explicit: str | None) -> tuple[str, set[str], str]:
    resolved_section = str(candidate.get("DriverInfResolvedSection") or "").strip()
    if not resolved_section:
        raise ContractError("target evidence has no DriverInfResolvedSection; re-run Capture-ProductionTarget.ps1")
    warnings = [str(x).strip() for x in (candidate.get("InterfaceResolutionWarnings") or []) if str(x).strip()]
    if warnings:
        raise ContractError(
            "target INF evidence contains unresolved warnings and is not safe to package: " + " | ".join(warnings)
        )

    candidates = _safe_reference_candidates(candidate)
    if explicit:
        wanted = _norm(explicit)
        matches = [row for row in candidates if _norm(row[0]) == wanted]
        if len(matches) != 1:
            raise ContractError(
                "--topology-reference is not a safely captured topology association. "
                "It must be either an AUDIO+TOPOLOGY pair or the exact legacy KSCATEGORY_AUDIO reference 'Topology'."
            )
        selected, categories, mode = matches[0]
    elif len(candidates) == 1:
        selected, categories, mode = candidates[0]
    else:
        summary = ", ".join(row[0] for row in candidates) or "<none>"
        raise ContractError(
            "capture must expose exactly one safe topology association reference; "
            f"got {len(candidates)}: {summary}. Do not guess."
        )

    if not REFERENCE_RE.fullmatch(selected):
        raise ContractError(f"topology reference contains unsupported INF characters: {selected!r}")
    if KSCATEGORY_AUDIO not in categories:
        raise ContractError("selected topology reference is not exposed under KSCATEGORY_AUDIO")
    return selected, categories, mode


def _validate_endpoint_effect_snapshot(capture: dict) -> None:
    snapshot = capture.get("CapturedEndpointEffects")
    if not isinstance(snapshot, dict) or not bool(snapshot.get("Readable")):
        detail = str((snapshot or {}).get("Error") or "no readable snapshot") if isinstance(snapshot, dict) else "snapshot missing"
        raise ContractError(
            "target capture does not contain a readable endpoint-effects snapshot: "
            f"{detail}. Re-run Capture-ProductionTarget.ps1."
        )
    if int(snapshot.get("EnhancementsDisabled") or 0) == 1:
        raise ContractError(
            "system effects were disabled on the captured endpoint; refusing to generate an APO extension that could not run"
        )
    existing: list[str] = []
    for key in ("LegacyEndpointEffects", "CompositeEndpointEffects"):
        existing.extend(str(x).strip() for x in (snapshot.get(key) or []) if str(x).strip())
    foreign = sorted({effect for effect in existing if _norm(effect) != _norm(APO_CLSID)}, key=str.lower)
    if foreign:
        raise ContractError(
            "captured endpoint already has non-Omniphony EFX registered: "
            + ", ".join(foreign)
            + ". Windows supports composite EFX, but Omniphony will not guess a safe ordering or overwrite a vendor effect."
        )


def _extension_id(hardware_id: str, topology_reference: str) -> str:
    name = f"{hardware_id.lower()}|{topology_reference.lower()}"
    return "{" + str(uuid.uuid5(EXTENSION_NAMESPACE, name)).upper() + "}"


def render_extension_inf(
    capture: dict,
    *,
    instance_id: str | None = None,
    hardware_id: str | None = None,
    topology_reference: str | None = None,
) -> str:
    schema = str(capture.get("Schema") or "")
    if schema != "omniphony.windows.apo-target.v3":
        raise ContractError(
            f"unsupported capture schema: {schema!r}; production packaging requires omniphony.windows.apo-target.v3"
        )
    _validate_endpoint_effect_snapshot(capture)
    candidate = _select_candidate(capture, instance_id, hardware_id)
    selected_hwid = _select_hardware_id(candidate, hardware_id)
    topology_ref, categories, evidence_mode = _select_topology_reference(candidate, topology_reference)
    extension_id = _extension_id(selected_hwid, topology_ref)

    interface_lines = [
        "AddInterface = %KSCATEGORY_AUDIO%,%TARGET_TOPOLOGY_REFERENCE%,OmniphonyCurrent_Interface"
    ]
    if KSCATEGORY_TOPOLOGY in categories:
        interface_lines.append(
            "AddInterface = %KSCATEGORY_TOPOLOGY%,%TARGET_TOPOLOGY_REFERENCE%,OmniphonyCurrent_Interface"
        )
    interface_block = "\n".join(interface_lines)

    return f'''; Generated by generate_extension_inf.py from machine-captured target evidence.
; Do not hand-edit the hardware ID, topology reference, or ExtensionId.
; TopologyEvidenceMode={evidence_mode}

[Version]
Signature   = "$WINDOWS NT$"
Class       = Extension
ClassGuid   = {{e2f84ce7-8efa-411c-aa69-97454ca4cb57}}
Provider    = %ProviderName%
ExtensionId = {extension_id}
DriverVer   = 08/17/2026,0.0.4.0
CatalogFile = OmniphonyApoExtension.cat
PnpLockDown = 1

[Manufacturer]
%MfgName% = DeviceExtensions,NTamd64.10.0...22000

[DeviceExtensions.NTamd64.10.0...22000]
%Device.ExtensionDesc% = OmniphonyCurrent_Install,{selected_hwid}

[OmniphonyCurrent_Install]

[OmniphonyCurrent_Install.Components]
AddComponent = OmniphonyCurrent,,OmniphonyCurrent_AddComponent

[OmniphonyCurrent_AddComponent]
ComponentIDs = VEN_OMNI&CID_CURRENT
Description  = "Omniphony Current Audio Processing Object"

[OmniphonyCurrent_Install.Interfaces]
{interface_block}

[OmniphonyCurrent_Interface]
AddReg = OmniphonyCurrent_Interface.AddReg

[OmniphonyCurrent_Interface.AddReg]
HKR,FX\\0,%PKEY_FX_Association%,,%KSNODETYPE_ANY%
HKR,FX\\0,%PKEY_CompositeFX_EndpointEffectClsid%,0x00010000,%OMNIPHONY_APO_CLSID%
HKR,FX\\0,%PKEY_EFX_ProcessingModes_Supported_For_Streaming%,0x00010000,%AUDIO_SIGNALPROCESSINGMODE_DEFAULT%

[Strings]
ProviderName = "Omniphony downstream fork"
MfgName = "Omniphony"
Device.ExtensionDesc = "Omniphony Current APO extension"
TARGET_TOPOLOGY_REFERENCE = "{topology_ref}"
OMNIPHONY_APO_CLSID = "{APO_CLSID}"

KSCATEGORY_AUDIO = "{{6994AD04-93EF-11D0-A3CC-00A0C9223196}}"
KSCATEGORY_TOPOLOGY = "{{DDA54A40-1E4C-11D1-A050-405705C10000}}"
KSNODETYPE_ANY = "{{00000000-0000-0000-0000-000000000000}}"
PKEY_FX_Association = "{{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D}},0"
PKEY_CompositeFX_EndpointEffectClsid = "{{D04E05A6-594B-4FB6-A80D-01AF5EED7D1D}},15"
PKEY_EFX_ProcessingModes_Supported_For_Streaming = "{{D3993A3F-99C2-4402-B5EC-A92A0367664B}},7"
AUDIO_SIGNALPROCESSINGMODE_DEFAULT = "{{C18E2F7E-933D-4965-B7D1-1EEF228D2AF3}}"
'''


def main() -> int:
    parser = argparse.ArgumentParser(
        description="Generate a production Omniphony audio-driver extension INF from finalized machine evidence."
    )
    parser.add_argument("capture_json", type=Path)
    parser.add_argument("output_inf", type=Path)
    parser.add_argument("--instance-id")
    parser.add_argument("--hardware-id")
    parser.add_argument("--topology-reference")
    args = parser.parse_args()
    capture = json.loads(args.capture_json.read_text(encoding="utf-8-sig"))
    try:
        text = render_extension_inf(
            capture,
            instance_id=args.instance_id,
            hardware_id=args.hardware_id,
            topology_reference=args.topology_reference,
        )
    except ContractError as exc:
        parser.error(str(exc))
    args.output_inf.parent.mkdir(parents=True, exist_ok=True)
    args.output_inf.write_text(text, encoding="utf-8", newline="\n")
    print(f"OMNIPHONY_EXTENSION_INF_OK\t{args.output_inf}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
