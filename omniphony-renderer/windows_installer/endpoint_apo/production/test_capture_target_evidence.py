import json
from pathlib import Path
import tempfile
import unittest

import capture_target_evidence as evidence


class TargetEvidenceTests(unittest.TestCase):
    def test_platform_extension_and_include_needs_resolve_exact_topology(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            base_inf = root / "synthetic.inf"
            shared_inf = root / "shared.inf"
            base_inf.write_text(
                """
[Strings]
SharedInf = "shared.inf"

[SyntheticAudio.NTamd64.Interfaces]
Include = %SharedInf%
Needs = Shared.Audio.Interfaces
""".strip()
                + "\n",
                encoding="utf-8",
            )
            shared_inf.write_text(
                """
[Strings]
KSCATEGORY_AUDIO = "{6994AD04-93EF-11D0-A3CC-00A0C9223196}"
KSCATEGORY_TOPOLOGY = "{DDA54A40-1E4C-11D1-A050-405705C10000}"
KSNAME_Render = "TopologyRender"

[Shared.Audio.Interfaces]
AddInterface = %KSCATEGORY_AUDIO%,%KSNAME_Render%,Shared.Render
AddInterface = %KSCATEGORY_TOPOLOGY%,%KSNAME_Render%,Shared.Render
""".strip()
                + "\n",
                encoding="utf-8",
            )
            capture = {
                "Schema": "omniphony.windows.apo-target.v2",
                "DefaultEndpoint": {"FriendlyName": "Synthetic"},
                "AssociationCandidates": [
                    {
                        "DriverInfFullPath": str(base_inf),
                        "DriverInfSection": "SyntheticAudio",
                        "DriverInfSectionExt": ".NTamd64",
                        "HardwareIds": [r"USB\VID_F00D&PID_BEEF"],
                    }
                ],
            }
            result = evidence.finalize_capture(capture, inf_root=root)
            candidate = result["AssociationCandidates"][0]
            self.assertEqual(result["Schema"], "omniphony.windows.apo-target.v3")
            self.assertEqual(candidate["DriverInfResolvedSection"], "SyntheticAudio.NTamd64")
            self.assertEqual(candidate["PairedTopologyReferenceCandidates"], ["TopologyRender"])
            self.assertEqual(len(candidate["DriverInterfaces"]), 2)
            self.assertTrue(any(row["ResolutionDepth"] == 1 for row in candidate["DriverInterfaces"]))

    def test_missing_or_ambiguous_needs_does_not_guess(self):
        with tempfile.TemporaryDirectory() as tmp:
            root = Path(tmp)
            base_inf = root / "synthetic.inf"
            a_inf = root / "a.inf"
            b_inf = root / "b.inf"
            base_inf.write_text(
                """
[SyntheticAudio.NT.Interfaces]
Include=a.inf,b.inf
Needs=Shared.Interfaces
""".strip()
                + "\n",
                encoding="utf-8",
            )
            for path in (a_inf, b_inf):
                path.write_text("[Shared.Interfaces]\nAddInterface={6994AD04-93EF-11D0-A3CC-00A0C9223196},Render,X\n", encoding="utf-8")
            interfaces, warnings, _ = evidence.collect_interface_evidence(
                base_inf, "SyntheticAudio.NT", inf_root=root
            )
            self.assertEqual(interfaces, [])
            self.assertTrue(any("ambiguous" in warning.lower() for warning in warnings))

    def test_resolved_section_does_not_double_append_extension(self):
        self.assertEqual(
            evidence.resolved_install_section("SyntheticAudio.NTamd64", ".NTamd64"),
            "SyntheticAudio.NTamd64",
        )
        self.assertEqual(
            evidence.resolved_install_section("SyntheticAudio", ".NTamd64"),
            "SyntheticAudio.NTamd64",
        )

    def test_rejects_non_v2_low_level_input(self):
        with self.assertRaises(evidence.EvidenceError):
            evidence.finalize_capture({"Schema": "omniphony.windows.apo-target.v1"})


if __name__ == "__main__":
    unittest.main()
