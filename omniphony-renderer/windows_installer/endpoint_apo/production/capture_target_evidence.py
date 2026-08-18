from __future__ import annotations

import argparse
import json
import os
from pathlib import Path
import re
from typing import Iterable

AUDIO_CATEGORY = "{6994ad04-93ef-11d0-a3cc-00a0c9223196}"
TOPOLOGY_CATEGORY = "{dda54a40-1e4c-11d1-a050-405705c10000}"


class EvidenceError(ValueError):
    pass


def _norm(value: object) -> str:
    return str(value or "").strip().lower()


def _strip_comment(line: str) -> str:
    quoted = False
    out: list[str] = []
    for char in line:
        if char == '"':
            quoted = not quoted
        if char == ";" and not quoted:
            break
        out.append(char)
    return "".join(out).strip()


class InfDocument:
    def __init__(self, path: Path):
        self.path = path.resolve()
        self.name = self.path.name
        self.sections: dict[str, tuple[str, list[str]]] = {}
        self.strings: dict[str, str] = {}
        current: str | None = None
        current_key: str | None = None

        for raw in self.path.read_text(encoding="utf-8-sig", errors="replace").splitlines():
            line = _strip_comment(raw)
            if not line:
                continue
            match = re.fullmatch(r"\[([^\]]+)\]", line)
            if match:
                current = match.group(1).strip()
                current_key = current.lower()
                self.sections.setdefault(current_key, (current, []))
                continue
            if not current_key:
                continue
            self.sections[current_key][1].append(line)
            if re.fullmatch(r"strings(?:\..+)?", current, re.IGNORECASE):
                if "=" in line:
                    key, value = line.split("=", 1)
                    self.strings[key.strip().lower()] = value.strip().strip('"')

    def resolve_token(self, value: str) -> str:
        value = value.strip().strip('"')
        match = re.fullmatch(r"%([^%]+)%", value)
        if match:
            return self.strings.get(match.group(1).lower(), value)
        return value

    def resolve_section(self, requested: str) -> str | None:
        exact = self.sections.get(requested.lower())
        if exact:
            return exact[0]
        prefix = requested.lower() + "."
        matches = [display for key, (display, _) in self.sections.items() if key.startswith(prefix)]
        return matches[0] if len(matches) == 1 else None

    def lines(self, section: str) -> list[str]:
        entry = self.sections.get(section.lower())
        return list(entry[1]) if entry else []


def resolved_install_section(base: str, extension: str) -> str:
    base = base.strip()
    extension = extension.strip()
    if not base:
        return ""
    if not extension:
        return base
    if not extension.startswith("."):
        extension = "." + extension
    return base if base.lower().endswith(extension.lower()) else base + extension


def _resolve_include(parent: Path, token: str, doc: InfDocument, inf_root: Path | None) -> Path | None:
    name = doc.resolve_token(token).strip().strip('"')
    if not name:
        return None
    candidate = Path(name)
    if candidate.is_absolute() and candidate.is_file():
        return candidate.resolve()
    beside = parent.parent / candidate.name
    if beside.is_file():
        return beside.resolve()
    if inf_root:
        rooted = inf_root / candidate.name
        if rooted.is_file():
            return rooted.resolve()
    windir = os.environ.get("WINDIR")
    if windir:
        rooted = Path(windir) / "INF" / candidate.name
        if rooted.is_file():
            return rooted.resolve()
    return None


def collect_interface_evidence(
    inf_path: Path,
    install_section: str,
    *,
    inf_root: Path | None = None,
) -> tuple[list[dict], list[str], list[dict]]:
    if not install_section:
        return [], ["resolved installed INF section is empty"], []
    cache: dict[Path, InfDocument] = {}
    visited: set[tuple[Path, str]] = set()
    evidence: list[dict] = []
    warnings: list[str] = []
    visited_rows: list[dict] = []

    def document(path: Path) -> InfDocument:
        resolved = path.resolve()
        if resolved not in cache:
            cache[resolved] = InfDocument(resolved)
        return cache[resolved]

    def visit(path: Path, requested_section: str, via: str, depth: int) -> None:
        if depth > 12:
            warnings.append(f"INF Include/Needs traversal exceeded depth 12 at {path} [{requested_section}]")
            return
        doc = document(path)
        resolved = doc.resolve_section(requested_section)
        if not resolved:
            warnings.append(f"INF section was not found unambiguously: {doc.name} [{requested_section}]")
            return
        key = (doc.path, resolved.lower())
        if key in visited:
            return
        visited.add(key)
        visited_rows.append(
            {
                "InfPath": str(doc.path),
                "InfName": doc.name,
                "Section": resolved,
                "Via": via,
                "Depth": depth,
            }
        )

        includes: list[str] = []
        needs: list[str] = []
        for line in doc.lines(resolved):
            if "=" not in line:
                continue
            directive, value = line.split("=", 1)
            directive = directive.strip().lower()
            if directive == "addinterface":
                parts = [part.strip() for part in value.split(",", 3)]
                if len(parts) < 2:
                    continue
                category_token = parts[0]
                reference_token = parts[1]
                install_token = parts[2] if len(parts) >= 3 else ""
                category = doc.resolve_token(category_token)
                reference = doc.resolve_token(reference_token)
                install = doc.resolve_token(install_token)
                evidence.append(
                    {
                        "Section": resolved,
                        "SectionRelevant": True,
                        "SourceInfPath": str(doc.path),
                        "SourceInfName": doc.name,
                        "ResolutionVia": via,
                        "ResolutionDepth": depth,
                        "CategoryToken": category_token,
                        "CategoryResolved": category,
                        "ReferenceToken": reference_token,
                        "ReferenceResolved": reference,
                        "InstallSectionToken": install_token,
                        "InstallSectionResolved": install,
                        "IsAudio": _norm(category) == AUDIO_CATEGORY,
                        "IsTopology": _norm(category) == TOPOLOGY_CATEGORY,
                    }
                )
            elif directive == "include":
                includes.extend(part.strip() for part in value.split(",") if part.strip())
            elif directive == "needs":
                needs.extend(part.strip() for part in value.split(",") if part.strip())

        if not needs:
            return
        if not includes:
            warnings.append(f"{doc.name} [{resolved}] has Needs= without Include=; refusing cross-INF guessing")
            return

        included: list[InfDocument] = []
        for token in includes:
            include_path = _resolve_include(doc.path, token, doc, inf_root)
            if not include_path:
                warnings.append(f"Included INF not found from {doc.name} [{resolved}]: {token}")
                continue
            try:
                included.append(document(include_path))
            except OSError as exc:
                warnings.append(f"Could not parse included INF {include_path}: {exc}")

        for token in needs:
            need = doc.resolve_token(token)
            matches: list[tuple[InfDocument, str]] = []
            for included_doc in included:
                matched = included_doc.resolve_section(need)
                if matched:
                    matches.append((included_doc, matched))
            if len(matches) == 0:
                warnings.append(f"Needs section not found in included INFs from {doc.name} [{resolved}]: {need}")
                continue
            if len(matches) > 1:
                locations = ", ".join(f"{item.name}[{section}]" for item, section in matches)
                warnings.append(f"Needs section is ambiguous across included INFs: {need} -> {locations}")
                continue
            included_doc, matched = matches[0]
            visit(included_doc.path, matched, f"Include/Needs from {doc.name}[{resolved}]", depth + 1)

    primary = document(inf_path)
    visit(primary.path, install_section + ".Interfaces", "installed-driver-section+platform-extension", 0)
    return evidence, sorted(set(warnings)), visited_rows


def paired_topology_refs(interfaces: Iterable[dict]) -> list[str]:
    categories: dict[str, set[str]] = {}
    display: dict[str, str] = {}
    for item in interfaces:
        ref = str(item.get("ReferenceResolved") or "").strip()
        category = _norm(item.get("CategoryResolved"))
        if not ref or not category:
            continue
        key = ref.lower()
        display.setdefault(key, ref)
        categories.setdefault(key, set()).add(category)
    return sorted(
        [display[key] for key, values in categories.items() if AUDIO_CATEGORY in values and TOPOLOGY_CATEGORY in values],
        key=str.lower,
    )


def finalize_capture(base: dict, *, inf_root: Path | None = None) -> dict:
    schema = str(base.get("Schema") or "")
    if schema != "omniphony.windows.apo-target.v2":
        raise EvidenceError(f"expected low-level v2 capture, got {schema!r}")

    out = json.loads(json.dumps(base))
    out["Schema"] = "omniphony.windows.apo-target.v3"
    out["EvidenceFinalizer"] = "capture_target_evidence.py"

    for candidate in out.get("AssociationCandidates") or []:
        base_section = str(candidate.get("DriverInfSection") or "")
        section_ext = str(candidate.get("DriverInfSectionExt") or "")
        resolved = resolved_install_section(base_section, section_ext)
        candidate["DriverInfSectionBase"] = base_section
        candidate["DriverInfSectionExt"] = section_ext
        candidate["DriverInfResolvedSection"] = resolved

        inf_path = Path(str(candidate.get("DriverInfFullPath") or ""))
        if not inf_path.is_file():
            candidate["DriverInterfaces"] = []
            candidate["InterfaceResolutionWarnings"] = [f"installed INF is not readable: {inf_path}"]
            candidate["InterfaceResolutionVisitedSections"] = []
            candidate["PairedTopologyReferenceCandidates"] = []
            continue

        interfaces, warnings, visited = collect_interface_evidence(inf_path, resolved, inf_root=inf_root)
        candidate["DriverInterfaces"] = interfaces
        candidate["InterfaceResolutionWarnings"] = warnings
        candidate["InterfaceResolutionVisitedSections"] = visited
        candidate["TopologyReferenceCandidates"] = sorted(
            {str(item.get("ReferenceResolved")) for item in interfaces if item.get("IsTopology") and item.get("ReferenceResolved")},
            key=str.lower,
        )
        candidate["PairedTopologyReferenceCandidates"] = paired_topology_refs(interfaces)

    return out


def main() -> int:
    parser = argparse.ArgumentParser(description="Finalize a low-level Omniphony Windows target capture into deterministic v3 INF evidence.")
    parser.add_argument("input_json", type=Path)
    parser.add_argument("output_json", type=Path)
    parser.add_argument("--inf-root", type=Path)
    args = parser.parse_args()

    base = json.loads(args.input_json.read_text(encoding="utf-8-sig"))
    try:
        final = finalize_capture(base, inf_root=args.inf_root)
    except EvidenceError as exc:
        parser.error(str(exc))
    args.output_json.parent.mkdir(parents=True, exist_ok=True)
    args.output_json.write_text(json.dumps(final, indent=2) + "\n", encoding="utf-8")
    print(f"OMNIPHONY_TARGET_EVIDENCE_V3_OK\t{args.output_json}")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())
