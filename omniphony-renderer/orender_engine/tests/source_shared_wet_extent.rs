use orender_engine::{
    SourceRendererOptions, SourceSpatialMode, build_source_frame_renderer,
    source_presentation_policy,
};
use renderer::binaural::HrirSource;
use renderer::source_scene::{SourceLaneKind, SourceSceneEvidence, present_source};

const SAMPLE_RATE: u32 = 48_000;
const FRAMES: usize = 2_048;

fn test_signal() -> Vec<f32> {
    (0..FRAMES)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            0.055 * (std::f32::consts::TAU * 743.0 * t).sin()
                + 0.025 * (std::f32::consts::TAU * 1_619.0 * t).sin()
        })
        .collect()
}

fn shared_wet_source() -> SourceSceneEvidence {
    SourceSceneEvidence {
        lane_kind: SourceLaneKind::SharedWetReturn,
        source_id: 0x5344_5350, // "SDSP"-like stable test token, not source provenance.
        diffuse: 1.0,
        width: 1.0,
        confidence: 1.0,
        ..SourceSceneEvidence::default()
    }
}

fn delta_rms(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len());
    (a.iter()
        .zip(b)
        .map(|(a, b)| (a - b) * (a - b))
        .sum::<f32>()
        / a.len() as f32)
        .sqrt()
}

#[test]
fn shared_wet_extent_changes_headphone_field_without_moving_its_center() {
    let source = shared_wet_source();
    let mut point_policy = source_presentation_policy(SourceSpatialMode::FullSphere);
    point_policy.shared_wet.extent = [0.0, 0.0, 0.0];
    let field_policy = source_presentation_policy(SourceSpatialMode::FullSphere);

    // Extent is an independent production dimension. Changing it must not move
    // the historical shared field's chosen center, rear bias, height or radius.
    let point_presentation = present_source(source, point_policy);
    let field_presentation = present_source(source, field_policy);
    assert_eq!(point_presentation.position, field_presentation.position);
    assert_eq!(point_presentation.azimuth_deg, field_presentation.azimuth_deg);
    assert_eq!(point_presentation.elevation_deg, field_presentation.elevation_deg);
    assert_eq!(point_presentation.distance, field_presentation.distance);
    assert_eq!(point_presentation.size, [0.0, 0.0, 0.0]);
    assert!(field_presentation.size[0] > 0.9);
    assert!(field_presentation.size[1] > 0.8);
    assert!(field_presentation.size[2] > 0.7);

    let mut renderer = build_source_frame_renderer(
        SAMPLE_RATE,
        None,
        SourceRendererOptions {
            mode: SourceSpatialMode::FullSphere,
            hrir_source: HrirSource::Synthetic,
            externalization: false,
            ..SourceRendererOptions::default()
        },
    )
    .expect("FullSphere source renderer");
    let input = test_signal();

    renderer.set_policy(point_policy);
    let point = renderer
        .render_source_frame(&input, &[source], 0, 0, Vec::new(), false)
        .expect("point shared-wet render")
        .samples;

    renderer.reset_runtime_state();
    renderer.set_policy(field_policy);
    let field = renderer
        .render_source_frame(&input, &[source], 0, 0, Vec::new(), false)
        .expect("extended shared-wet render")
        .samples;

    assert_eq!(point.len(), FRAMES * 2);
    assert_eq!(field.len(), point.len());
    assert!(point.iter().chain(&field).all(|sample| sample.is_finite()));

    let difference = delta_rms(&point, &field);
    assert!(
        difference > 1.0e-5,
        "shared wet extent must alter the rendered headphone field; delta_rms={difference}"
    );
}
