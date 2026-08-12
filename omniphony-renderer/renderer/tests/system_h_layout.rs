use renderer::speaker_layout::SpeakerLayout;
use renderer::spatial_vbap::VbapPanner;

const SYSTEM_H_22: &str = include_str!("../../../layouts/itu-r-bs2051-system-h-22.0.yaml");

#[test]
fn system_h_support_shell_is_a_real_three_layer_unit_sphere() {
    let layout = SpeakerLayout::from_yaml_str(SYSTEM_H_22).expect("System H layout must parse");

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
    assert_eq!(lower, 3, "System H support shell must retain its bottom layer");

    let zenith = layout
        .speakers
        .iter()
        .find(|speaker| speaker.name == "TpC")
        .expect("System H support shell must retain top centre");
    assert!(zenith.x.abs() < 1.0e-6);
    assert!(zenith.y.abs() < 1.0e-6);
    assert!((zenith.z - 1.0).abs() < 1.0e-6);
}

#[test]
fn system_h_support_shell_can_pan_into_the_lower_hemisphere() {
    let layout = SpeakerLayout::from_yaml_str(SYSTEM_H_22).expect("System H layout must parse");
    let positions = layout.positions();
    let panner = VbapPanner::new(&positions, 5, 5, 0.0, Default::default())
        .expect("System H directions must triangulate")
        .with_negative_z(true);

    let gains = panner.get_gains_cartesian(0.0, 0.866025, -0.5, 0.025);
    assert_eq!(gains.len(), 22);
    assert!(gains.iter().all(|gain| gain.is_finite()));
    let energy: f32 = gains.iter().map(|gain| gain * gain).sum();
    assert!(energy > 0.5, "lower-hemisphere pan lost energy: {energy}");
}
