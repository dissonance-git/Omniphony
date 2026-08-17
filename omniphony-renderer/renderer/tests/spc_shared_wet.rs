use renderer::source_scene::{
    NativeStereoRoute, SourceLaneKind, SourcePresentationPolicy, SourceSceneEvidence,
    present_source,
};

fn echo_half(
    source_id: u64,
    persistent_part_id: u64,
    left_gain: f32,
    right_gain: f32,
) -> SourceSceneEvidence {
    SourceSceneEvidence {
        lane_kind: SourceLaneKind::SharedWetReturn,
        source_id,
        persistent_part_id: Some(persistent_part_id),
        native_stereo_route: Some(NativeStereoRoute {
            left_gain,
            right_gain,
        }),
        diffuse: 1.0,
        width: 1.0,
        confidence: 1.0,
        ..SourceSceneEvidence::default()
    }
}

#[test]
fn linked_spc_echo_halves_keep_opposite_authored_sides() {
    let field_id = 0x5344_5350_4543_4809;
    let left = present_source(
        echo_half(field_id ^ 0x4c, field_id, -1.0, 0.0),
        SourcePresentationPolicy::default(),
    );
    let right = present_source(
        echo_half(field_id ^ 0x52, field_id, 0.0, 64.0 / 127.0),
        SourcePresentationPolicy::default(),
    );

    // Signed S-DSP polarity is source truth, but side comes from gain magnitude.
    assert_eq!(left.route_pan, -1.0);
    assert_eq!(right.route_pan, 1.0);

    // One persistent environmental identity must not collapse its stereo halves.
    assert!(left.azimuth_deg < -90.0);
    assert!(right.azimuth_deg > 90.0);
    assert!((left.azimuth_deg.abs() - right.azimuth_deg.abs()).abs() < 1.0e-5);
    assert!((left.elevation_deg - right.elevation_deg).abs() < 1.0e-5);
    assert!((left.distance - right.distance).abs() < 1.0e-5);
    assert_eq!(left.size, [1.0, 1.0, 1.0]);
    assert_eq!(right.size, [1.0, 1.0, 1.0]);
}

#[test]
fn spc_echo_native_mode_preserves_side_without_inventing_room_depth() {
    let field_id = 0x5344_5350_4543_480a;
    let policy = SourcePresentationPolicy {
        sphere_strength: 0.0,
        max_elevation_deg: 0.0,
        max_distance: 1.0,
        ..SourcePresentationPolicy::default()
    };

    let left = present_source(echo_half(field_id ^ 0x4c, field_id, 1.0, 0.0), policy);
    let right = present_source(echo_half(field_id ^ 0x52, field_id, 0.0, 1.0), policy);

    assert_eq!(left.azimuth_deg, -70.0);
    assert_eq!(right.azimuth_deg, 70.0);
    assert_eq!(left.elevation_deg, 0.0);
    assert_eq!(right.elevation_deg, 0.0);
    assert_eq!(left.distance, 1.0);
    assert_eq!(right.distance, 1.0);
}
