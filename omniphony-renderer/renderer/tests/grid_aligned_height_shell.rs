use renderer::speaker_layout::SpeakerLayout;

const GRID_ALIGNED_22: &str =
    include_str!("../../../layouts/system-h-derived-22.0-upper60-grid10.yaml");

fn angles(speaker: &renderer::speaker_layout::Speaker) -> (f32, f32) {
    let az = speaker.x.atan2(speaker.y).to_degrees();
    let horizontal = (speaker.x * speaker.x + speaker.y * speaker.y).sqrt();
    let el = speaker.z.atan2(horizontal).to_degrees();
    (az, el)
}

#[test]
fn upper_ring_lands_on_ten_degree_hrtf_nodes() {
    let layout = SpeakerLayout::from_yaml_str(GRID_ALIGNED_22)
        .expect("grid-aligned headphone shell must parse");
    assert_eq!(layout.num_speakers(), 22);

    let expected = [
        ("TpFC", 0.0_f32),
        ("TpFL", -40.0),
        ("TpFR", 40.0),
        ("TpSiL", -90.0),
        ("TpSiR", 90.0),
        ("TpBL", -140.0),
        ("TpBR", 140.0),
        ("TpBC", 180.0),
    ];
    for (name, expected_az) in expected {
        let speaker = layout
            .speakers
            .iter()
            .find(|speaker| speaker.name == name)
            .unwrap_or_else(|| panic!("missing upper speaker {name}"));
        let (az, el) = angles(speaker);
        let az_error = if expected_az.abs() == 180.0 {
            (az.abs() - 180.0).abs()
        } else {
            (az - expected_az).abs()
        };
        assert!(az_error < 0.05, "{name}: azimuth {az} != {expected_az}");
        assert!((el - 60.0).abs() < 0.05, "{name}: elevation {el} != 60");
        assert!((az / 10.0 - (az / 10.0).round()).abs() < 0.01);
        assert!((el / 10.0 - (el / 10.0).round()).abs() < 0.01);
    }
}
