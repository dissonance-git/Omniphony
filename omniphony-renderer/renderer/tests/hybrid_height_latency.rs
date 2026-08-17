use renderer::live_params::{
    BinauralMode, LiveEvaluationMode, OutputMode, PreferredEvaluationMode,
};
use renderer::spatial_renderer::{SpatialChannelEvent, SpatialRenderer};
use renderer::spatial_vbap::{DistanceModel, VbapTableMode};
use renderer::speaker_layout::SpeakerLayout;

const BLOCK: usize = 40;
const SETTLE_BLOCKS: usize = 30;
const CAPTURE_BLOCKS: usize = 8;
const ARRIVAL_THRESHOLD: f32 = 1.0e-6;

fn build_renderer() -> SpatialRenderer {
    let layout = SpeakerLayout::preset("7.1.4").expect("7.1.4 preset");
    SpatialRenderer::new(
        layout,
        48_000,
        1,
        1,
        0.0,
        2.0,
        VbapTableMode::Cartesian {
            x_size: 21,
            y_size: 21,
            z_size: 9,
            z_neg_size: 9,
        },
        false,
        true,
        DistanceModel::None,
        false,
        1.0,
        1.0,
        0.0,
        1.0,
        false,
        [1.0, 1.0, 1.0],
        1.0,
        1.0,
        0.0,
        0.0,
        false,
        false,
        false,
        1.0,
        1.0,
        PreferredEvaluationMode::PrecomputedCartesian,
        LiveEvaluationMode::Realtime,
        21,
        21,
        9,
        9,
    )
    .expect("renderer")
}

fn render_impulse(cascaded: bool) -> Vec<f32> {
    // Mirrors the music field's top-front geometry closely: about -44 degrees
    // azimuth and +57 degrees elevation. This deliberately does not sit exactly
    // on the stock 7.1.4 upper speaker, so cascade VBAP really participates.
    let position = [-0.96, 1.0, 2.15];
    let mut renderer = build_renderer();
    {
        let control = renderer.renderer_control();
        let mut live = control.live.write();
        live.binaural.output_mode = OutputMode::Binaural;
        live.binaural.mode = if cascaded {
            BinauralMode::Cascaded
        } else {
            BinauralMode::Direct
        };
        live.binaural.reflections.enabled = false;
        live.binaural.reverb.enabled = false;
        live.binaural.air_absorption = false;
    }

    let event = [SpatialChannelEvent {
        channel_idx: 0,
        is_bed: false,
        gain_db: Some(0),
        ramp_length: Some(0),
        size: Some([0.0, 0.0, 0.0]),
        position: Some(position),
        sample_pos: Some(0),
    }];

    let silence = [0.0_f32; BLOCK];
    for block in 0..SETTLE_BLOCKS {
        let events: &[SpatialChannelEvent] = if block == 0 { &event } else { &[] };
        renderer
            .render_frame(&silence, 1, events, Vec::new(), false)
            .expect("settle render");
    }

    let mut captured = Vec::with_capacity(CAPTURE_BLOCKS * BLOCK * 2);
    for block in 0..CAPTURE_BLOCKS {
        let mut input = [0.0_f32; BLOCK];
        if block == 0 {
            input[0] = 1.0;
        }
        let out = renderer
            .render_frame(&input, 1, &[], Vec::new(), false)
            .expect("impulse render");
        captured.extend_from_slice(&out.samples);
    }
    captured
}

fn first_arrival_frame(stereo: &[f32]) -> Option<usize> {
    stereo
        .chunks_exact(2)
        .position(|frame| frame[0].abs().max(frame[1].abs()) >= ARRIVAL_THRESHOLD)
}

#[test]
fn direct_and_cascade_height_arrivals_are_sample_aligned() {
    let direct = render_impulse(false);
    let cascade = render_impulse(true);
    let direct_frame = first_arrival_frame(&direct).expect("direct impulse arrival");
    let cascade_frame = first_arrival_frame(&cascade).expect("cascade impulse arrival");
    let lag = direct_frame.abs_diff(cascade_frame);

    assert!(
        lag <= 1,
        "direct/cascade height arrival mismatch is {lag} frames: direct={direct_frame}, cascade={cascade_frame}"
    );
}
