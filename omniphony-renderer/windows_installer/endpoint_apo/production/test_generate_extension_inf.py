import copy
import unittest

import generate_extension_inf as gen


def fixture():
    return {
        "Schema": "omniphony.windows.apo-target.v3",
        "CapturedEndpointEffects": {
            "Readable": True,
            "LegacyEndpointEffects": [],
            "CompositeEndpointEffects": [],
            "EnhancementsDisabled": 0,
        },
        "AssociationCandidates": [
            {
                "InstanceId": r"USB\VID_F00D&PID_BEEF&MI_00\123",
                "Class": "MEDIA",
                "ClassGuid": "{4D36E96C-E325-11CE-BFC1-08002BE10318}",
                "HardwareIds": [
                    r"USB\VID_F00D&PID_BEEF&MI_00",
                    r"USB\VID_F00D&PID_BEEF",
                ],
                "DriverInfResolvedSection": "SyntheticAudio.NTamd64",
                "InterfaceResolutionWarnings": [],
                "DriverInterfaces": [
                    {
                        "CategoryResolved": "{6994AD04-93EF-11D0-A3CC-00A0C9223196}",
                        "ReferenceResolved": "TopologyRender",
                    },
                    {
                        "CategoryResolved": "{DDA54A40-1E4C-11D1-A050-405705C10000}",
                        "ReferenceResolved": "TopologyRender",
                    },
                ],
            }
        ],
    }


class ExtensionInfTests(unittest.TestCase):
    def test_generates_component_and_efx_association(self):
        text = gen.render_extension_inf(fixture())
        self.assertIn("Class       = Extension", text)
        self.assertIn("USB\\VID_F00D&PID_BEEF&MI_00", text)
        self.assertIn("AddComponent = OmniphonyCurrent", text)
        self.assertIn("ComponentIDs = VEN_OMNI&CID_CURRENT", text)
        self.assertIn("PKEY_CompositeFX_EndpointEffectClsid", text)
        self.assertIn("PKEY_EFX_ProcessingModes_Supported_For_Streaming", text)
        self.assertIn('TARGET_TOPOLOGY_REFERENCE = "TopologyRender"', text)
        self.assertIn(gen.APO_CLSID, text)
        normalized = text.lower()
        for forbidden in ("hklm,", "hkcr,", "disableprotectedaudiodg", "mmdevices\\audio\\render"):
            self.assertNotIn(forbidden, normalized)

    def test_extension_id_is_deterministic(self):
        one = gen.render_extension_inf(fixture())
        two = gen.render_extension_inf(fixture())
        line_one = next(x for x in one.splitlines() if x.startswith("ExtensionId"))
        line_two = next(x for x in two.splitlines() if x.startswith("ExtensionId"))
        self.assertEqual(line_one, line_two)

    def test_rejects_multiple_media_candidates_without_witness(self):
        data = fixture()
        data["AssociationCandidates"].append(copy.deepcopy(data["AssociationCandidates"][0]))
        data["AssociationCandidates"][1]["InstanceId"] = r"USB\VID_F00D&PID_CAFE&MI_00\456"
        data["AssociationCandidates"][1]["HardwareIds"] = [r"USB\VID_F00D&PID_CAFE&MI_00"]
        with self.assertRaises(gen.ContractError):
            gen.render_extension_inf(data)

    def test_hardware_id_can_disambiguate_captured_candidates(self):
        data = fixture()
        second = copy.deepcopy(data["AssociationCandidates"][0])
        second["InstanceId"] = r"USB\VID_F00D&PID_CAFE&MI_00\456"
        second["HardwareIds"] = [r"USB\VID_F00D&PID_CAFE&MI_00"]
        data["AssociationCandidates"].append(second)
        text = gen.render_extension_inf(
            data, hardware_id=r"USB\VID_F00D&PID_CAFE&MI_00"
        )
        self.assertIn(r"USB\VID_F00D&PID_CAFE&MI_00", text)

    def test_rejects_unpaired_or_ambiguous_topology(self):
        data = fixture()
        data["AssociationCandidates"][0]["DriverInterfaces"].append(
            {
                "CategoryResolved": "{6994AD04-93EF-11D0-A3CC-00A0C9223196}",
                "ReferenceResolved": "TopologyOther",
            }
        )
        data["AssociationCandidates"][0]["DriverInterfaces"].append(
            {
                "CategoryResolved": "{DDA54A40-1E4C-11D1-A050-405705C10000}",
                "ReferenceResolved": "TopologyOther",
            }
        )
        with self.assertRaises(gen.ContractError):
            gen.render_extension_inf(data)

        text = gen.render_extension_inf(data, topology_reference="TopologyOther")
        self.assertIn('TARGET_TOPOLOGY_REFERENCE = "TopologyOther"', text)

    def test_rejects_uncaptured_topology_override(self):
        with self.assertRaises(gen.ContractError):
            gen.render_extension_inf(fixture(), topology_reference="MadeUpTopology")

    def test_rejects_mmdevice_target(self):
        data = fixture()
        data["AssociationCandidates"][0]["HardwareIds"] = [
            r"SWD\MMDEVAPI\{00000000-0000-0000-0000-000000000000}"
        ]
        with self.assertRaises(gen.ContractError):
            gen.render_extension_inf(data)

    def test_rejects_foreign_existing_endpoint_effect(self):
        data = fixture()
        data["CapturedEndpointEffects"]["CompositeEndpointEffects"] = [
            "{11111111-1111-1111-1111-111111111111}"
        ]
        with self.assertRaises(gen.ContractError):
            gen.render_extension_inf(data)

    def test_allows_existing_omniphony_effect_for_upgrade(self):
        data = fixture()
        data["CapturedEndpointEffects"]["CompositeEndpointEffects"] = [gen.APO_CLSID]
        self.assertIn(gen.APO_CLSID, gen.render_extension_inf(data))

    def test_rejects_disabled_system_effects(self):
        data = fixture()
        data["CapturedEndpointEffects"]["EnhancementsDisabled"] = 1
        with self.assertRaises(gen.ContractError):
            gen.render_extension_inf(data)

    def test_rejects_unfinalized_schema_or_inf_warning(self):
        data = fixture()
        data["Schema"] = "omniphony.windows.apo-target.v2"
        with self.assertRaises(gen.ContractError):
            gen.render_extension_inf(data)

        data = fixture()
        data["AssociationCandidates"][0]["InterfaceResolutionWarnings"] = ["unresolved include"]
        with self.assertRaises(gen.ContractError):
            gen.render_extension_inf(data)


if __name__ == "__main__":
    unittest.main()
