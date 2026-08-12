use renderer::speaker_layout::SpeakerLayout;
use renderer::spatial_vbap::VbapPanner;

// Historical runtime compatibility path: currently the controlled headphone
// height A/B with the upper ring moved to +60 degrees.
const ACTIVE_HEADPHONE_22: &str = include_str!("../../../layouts/itu-r-bs2051-system-h-22.0.yaml");
// Normative/reference System H geometry remains frozen separately.
const SYSTEM_H_REFERENCE_22: &str =
    include_str!("../../../layouts/reference/itu-r-bs2051-system-h-22.0.yaml");

fn elevation_deg(speaker: &renderer::speaker_layout::Speaker) -> f32 {
    let horizontal = (speaker.x * speaker.x + speaker.y * speaker.y).sqrt();
    speaker.z.atan2(horizontal).to_degrees()
}

#[test]
fn canonical_system_h_reference_keeps_30_degree_upper_ring() {
    let layout = SpeakerLayout::from_yaml_str(SYSTEM_H_REFERENCE_22)
        .expect("canonical System H reference layout must parse");
    assert_eq!(layout.num_speakers(), 22);

    for name in [
        "TpFC", "TpFL", "TpFR", "TpSiL", "TpSiR", "TpBL", "TpBR", "TpBC",
    ] {
        let speaker = layout
            .speakers
            .iter()
            .find(|speaker| speaker.name == name)
            .unwrap_or_else(|| panic!("missing canonical upper speaker {name}"));
        let elevation = elevation_deg(speaker);
        assert!(
            (elevation - 30.0).abs() < 0.05,
            "canonical {name} moved away from +30 degrees: {elevation}"
        );
    }
}

#[test]
fn active_headphone_shell_is_a_real_three_layer_unit_sphere() {
    let layout = SpeakerLayout::from_yaml_str(ACTIVE_HEADPHONE_22)
        .expect("active headphone layout must parse");

    assert_eq!(layout.num_speakers(), 22);
    assert!(layout.speakers.iter().all(|speaker| speaker.spatialize));
    assert!(
        layout
            .speakers
            .iter()
            .all(|speaker| !speaker.name.to_ascii_uppercase().contains("LFE"))
    );

    for speaker in &layout.speakers {
        assert!(speaker.x.is_finite());
        assert!(speaker.y.is_finite());
        assert!(speaker.z.is_finite());
        let radius = (speaker.x * speaker.x
            + speaker.y * speaker.y
            + speaker.z * speaker.z)
            .sqrt();
        assert!(
            (radius - 1.0).abs() < 2.0e-4,
            "{} is off the unit sphere: radius={radius}",
            speaker.name
        );
    }

    let lower = layout
        .speakers
        .iter()
        .filter(|speaker| speaker.z < -0.45)
        .count();
    assert_eq!(lower, 3, "headphone shell must retain the bottom layer");

    let zenith = layout
        .speakers
        .iter()
        .find(|speaker| speaker.name == "TpC")
        .expect("headphone shell must retain top centre");
    assert!(zenith.x.abs() < 1.0e-6);
    assert!(zenith.y.abs() < 1.0e-6);
    assert!((zenith.z - 1.0).abs() < 1.0e-6);
}

#[test]
fn active_headphone_upper_ring_is_60_degrees() {
    let layout = SpeakerLayout::from_yaml_str(ACTIVE_HEADPHONE_22)
        .expect("active headphone layout must parse");

    for name in [
        "TpFC", "TpFL", "TpFR", "TpSiL", "TpSiR", "TpBL", "TpBR", "TpBC",
    ] {
        let speaker = layout
            .speakers
            .iter()
            .find(|speaker| speaker.name == name)
            .unwrap_or_else(|| panic!("missing active upper speaker {name}"));
        let elevation = elevation_deg(speaker);
        assert!(
            (elevation - 60.0).abs() < 0.05,
            "active {name} is not near +60 degrees: {elevation}"
        );
    }
}

#[test]
fn active_headphone_shell_can_pan_into_the_lower_hemisphere() {
    let layout = SpeakerLayout::from_yaml_str(ACTIVE_HEADPHONE_22)
        .expect("active headphone layout must parse");
    let positions = layout.positions();
    let panner = VbapPanner::new(&positions, 5, 5, 0.0, Default::default())
        .expect("active headphone directions must triangulate")
        .with_negative_z(true);

    let gains = panner.get_gains_cartesian(0.0, 0.866025, -0.5, 0.025);
    assert_eq!(gains.len(), 22);
    assert!(gains.iter().all(|gain| gain.is_finite()));
    let energy: f32 = gains.iter().map(|gain| gain * gain).sum();
    assert!(energy > 0.5, "lower-hemisphere pan lost energy: {energy}");
}
