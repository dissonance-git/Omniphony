use omniphony_source::{
    OmniphonySourceConfig, OmniphonySourceEvidenceV1, SOURCE_FLAG_AUTHORED_POSITION,
    SOURCE_FLAG_PERSISTENT_PART, SOURCE_HRIR_SYNTHETIC, SOURCE_LANE_DRY,
    SOURCE_LANE_SHARED_WET, SOURCE_SPATIAL_FULL_SPHERE, SOURCE_SPATIAL_NATIVE_ROUTING,
    omniphony_source_create, omniphony_source_destroy, omniphony_source_process_f32,
};

const SAMPLE_RATE: u32 = 48_000;
const FRAMES: usize = 1_024;

fn config(spatial_mode: u32) -> OmniphonySourceConfig {
    OmniphonySourceConfig {
        sample_rate_hz: SAMPLE_RATE,
        spatial_mode,
        externalization: 0,
        hrir_source: SOURCE_HRIR_SYNTHETIC,
        unit_scale_m: 1.0,
        reflection_level: 0.0,
    }
}

fn dry(source_id: u64) -> OmniphonySourceEvidenceV1 {
    OmniphonySourceEvidenceV1 {
        lane_kind: SOURCE_LANE_DRY,
        flags: SOURCE_FLAG_AUTHORED_POSITION,
        source_id,
        authored_x: 0.45,
        authored_y: 0.85,
        authored_z: 0.20,
        width: 1.0,
        diffuse: 1.0,
        confidence: 1.0,
        ..OmniphonySourceEvidenceV1::default()
    }
}

fn shared_wet(source_id: u64) -> OmniphonySourceEvidenceV1 {
    OmniphonySourceEvidenceV1 {
        lane_kind: SOURCE_LANE_SHARED_WET,
        // Hold the field's presentation identity fixed while varying only the
        // bounded source-episode token. Shared-wet placement legitimately uses
        // persistent/source identity when native pan does not choose a side.
        flags: SOURCE_FLAG_PERSISTENT_PART,
        source_id,
        persistent_part_id: 0x5745_5446, // "WETF"
        width: 1.0,
        diffuse: 1.0,
        confidence: 1.0,
        ..OmniphonySourceEvidenceV1::default()
    }
}

fn signal() -> Vec<f32> {
    (0..FRAMES)
        .map(|frame| {
            let t = frame as f32 / SAMPLE_RATE as f32;
            0.06 * (std::f32::consts::TAU * 911.0 * t).sin()
                + 0.025 * (std::f32::consts::TAU * 2_173.0 * t).sin()
        })
        .collect()
}

fn render(spatial_mode: u32, evidence: OmniphonySourceEvidenceV1) -> Vec<f32> {
    let cfg = config(spatial_mode);
    let processor = unsafe { omniphony_source_create(&cfg) };
    assert!(!processor.is_null());
    let input = signal();
    let mut output = vec![0.0f32; FRAMES * 2];
    let status = unsafe {
        omniphony_source_process_f32(
            processor,
            input.as_ptr(),
            &evidence,
            1,
            FRAMES,
            0,
            96,
            output.as_mut_ptr(),
        )
    };
    unsafe { omniphony_source_destroy(processor) };
    assert_eq!(status, 0);
    assert!(output.iter().all(|sample| sample.is_finite()));
    output
}

fn delta_rms(left: &[f32], right: &[f32]) -> f32 {
    assert_eq!(left.len(), right.len());
    (left
        .iter()
        .zip(right)
        .map(|(left, right)| (left - right) * (left - right))
        .sum::<f32>()
        / left.len() as f32)
        .sqrt()
}

#[test]
fn bounded_dry_episode_changes_only_fullsphere_extent_at_fixed_authored_center() {
    // source_id=0 deliberately means "no usable bounded episode token" to the
    // renderer-local attack guard. Both cases use the same authored position,
    // PCM and presentation evidence, isolating compact attack extent.
    let unguarded = render(SOURCE_SPATIAL_FULL_SPHERE, dry(0));
    let guarded = render(SOURCE_SPATIAL_FULL_SPHERE, dry(1));
    let difference = delta_rms(&unguarded, &guarded);
    assert!(
        difference > 1.0e-5,
        "bounded dry episode must alter FullSphere extent treatment; delta_rms={difference}"
    );
}

#[test]
fn native_routing_is_unchanged_by_episode_attack_guard() {
    let no_episode = render(SOURCE_SPATIAL_NATIVE_ROUTING, dry(0));
    let bounded_episode = render(SOURCE_SPATIAL_NATIVE_ROUTING, dry(1));
    let difference = delta_rms(&no_episode, &bounded_episode);
    assert!(
        difference < 1.0e-6,
        "NativeRouting already closes extent, so attack guard must be inaudible; delta_rms={difference}"
    );
}

#[test]
fn shared_wet_never_receives_dry_episode_attack_compactness() {
    let no_episode = render(SOURCE_SPATIAL_FULL_SPHERE, shared_wet(0));
    let bounded_id = render(SOURCE_SPATIAL_FULL_SPHERE, shared_wet(1));
    let difference = delta_rms(&no_episode, &bounded_id);
    assert!(
        difference < 1.0e-6,
        "shared wet is an environmental field, not a point attack; delta_rms={difference}"
    );
}
