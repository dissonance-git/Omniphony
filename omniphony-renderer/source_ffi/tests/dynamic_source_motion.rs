use omniphony_source::{
    OmniphonySourceConfig, OmniphonySourceEvidenceEventV1, OmniphonySourceEvidenceV1,
    SOURCE_FLAG_AUTHORED_POSITION, SOURCE_HRIR_SYNTHETIC, SOURCE_LANE_DRY,
    SOURCE_SPATIAL_FULL_SPHERE, omniphony_source_create, omniphony_source_destroy,
    omniphony_source_process_events_f32,
};

const SAMPLE_RATE: u32 = 48_000;
const FRAMES: usize = 4_096;
const MOVE_FRAME: usize = FRAMES / 2;
const SOURCE_ID: u64 = 0xD1A0_0001;

fn signal() -> Vec<f32> {
    (0..FRAMES)
        .map(|i| {
            let t = i as f32 / SAMPLE_RATE as f32;
            0.05 * (std::f32::consts::TAU * 701.0 * t).sin()
                + 0.02 * (std::f32::consts::TAU * 1_933.0 * t).sin()
        })
        .collect()
}

fn evidence_at(x: f32, y: f32, z: f32) -> OmniphonySourceEvidenceV1 {
    OmniphonySourceEvidenceV1 {
        lane_kind: SOURCE_LANE_DRY,
        flags: SOURCE_FLAG_AUTHORED_POSITION,
        source_id: SOURCE_ID,
        authored_x: x,
        authored_y: y,
        authored_z: z,
        confidence: 1.0,
        ..OmniphonySourceEvidenceV1::default()
    }
}

fn render(
    initial: OmniphonySourceEvidenceV1,
    events: &[OmniphonySourceEvidenceEventV1],
) -> Vec<f32> {
    let config = OmniphonySourceConfig {
        sample_rate_hz: SAMPLE_RATE,
        spatial_mode: SOURCE_SPATIAL_FULL_SPHERE,
        externalization: 0,
        hrir_source: SOURCE_HRIR_SYNTHETIC,
        unit_scale_m: 1.0,
        reflection_level: 0.0,
    };
    let processor = unsafe { omniphony_source_create(&config) };
    assert!(!processor.is_null());

    let input = signal();
    let mut output = vec![0.0f32; FRAMES * 2];
    let event_ptr = if events.is_empty() {
        std::ptr::null()
    } else {
        events.as_ptr()
    };
    let status = unsafe {
        omniphony_source_process_events_f32(
            processor,
            input.as_ptr(),
            &initial,
            1,
            event_ptr,
            events.len(),
            FRAMES,
            0,
            0,
            output.as_mut_ptr(),
        )
    };
    assert_eq!(status, 0);
    assert!(output.iter().all(|sample| sample.is_finite()));

    unsafe { omniphony_source_destroy(processor) };
    output
}

fn rms_delta(a: &[f32], b: &[f32]) -> f32 {
    (a.iter()
        .zip(b)
        .map(|(x, y)| {
            let d = x - y;
            d * d
        })
        .sum::<f32>()
        / a.len() as f32)
        .sqrt()
}

#[test]
fn stable_source_id_can_move_at_sample_offset_without_bed_quantization() {
    let left = evidence_at(-0.8, 1.0, 0.15);
    let right = evidence_at(0.8, 1.0, -0.2);
    assert_eq!(left.source_id, right.source_id);

    let moving = render(
        left,
        &[OmniphonySourceEvidenceEventV1 {
            frame_offset: MOVE_FRAME as u32,
            lane_index: 0,
            evidence: right,
        }],
    );
    let fixed_left = render(left, &[]);
    let fixed_right = render(right, &[]);

    let after_move = MOVE_FRAME * 2;
    let moving_vs_left = rms_delta(&moving[after_move..], &fixed_left[after_move..]);
    let moving_vs_right = rms_delta(&moving[after_move..], &fixed_right[after_move..]);

    assert!(
        moving_vs_left > 1.0e-6,
        "an authored position event must change the rendered scene; delta_rms={moving_vs_left}"
    );
    assert!(
        moving_vs_right < moving_vs_left,
        "after the event, the moving source should converge toward the new authored position; moving_vs_right={moving_vs_right}, moving_vs_left={moving_vs_left}"
    );
}
