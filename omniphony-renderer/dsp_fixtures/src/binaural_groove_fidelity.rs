//! Musical timing fidelity for the stationary binaural path.
//!
//! libaural AUD-RHYTHM-001 established a useful representation obligation:
//! tempo and beat-relative microtiming are distinct. Omniphony does not need a
//! human groove model to enforce the corresponding renderer invariant: a fixed
//! HRTF may color and delay a transient, but it must not change the relative
//! timing between musical events.

use crate::scene::{SAMPLE_RATE, build_renderer_binaural};
use renderer::live_params::RampMode;
use renderer::spatial_renderer::SpatialChannelEvent;
use renderer::speaker_layout::SpeakerLayout;

const PRIME_SAMPLES: usize = 1_280;
const PRIME_BLOCK: usize = 40;
const BEAT_SAMPLES: usize = SAMPLE_RATE as usize / 2; // 120 BPM.
const BEATS: usize = 4;
const SEARCH_TAIL: usize = 512;

fn front_event() -> SpatialChannelEvent {
    SpatialChannelEvent {
        channel_idx: 0,
        is_bed: false,
        gain_db: Some(0),
        ramp_length: Some(0),
        size: Some([0.0, 0.0, 0.0]),
        position: Some([0.0, 1.0, 0.0]),
        sample_pos: Some(0),
    }
}

fn event_samples(offbeat_phase: f64) -> Vec<usize> {
    let mut events = Vec::with_capacity(BEATS * 2);
    for beat in 0..BEATS {
        let base = beat * BEAT_SAMPLES;
        events.push(base);
        events.push(base + (offbeat_phase * BEAT_SAMPLES as f64).round() as usize);
    }
    events
}

fn render_click_train(offbeat_phase: f64, block_samples: usize) -> (Vec<f32>, Vec<usize>) {
    let mut renderer = build_renderer_binaural(
        SpeakerLayout::preset("7.1.4").expect("known preset"),
        true,
        false,
    );
    {
        let control = renderer.renderer_control();
        control.set_requested_ramp_mode(RampMode::Sample);
        control.live.write().ramp_mode = RampMode::Sample;
    }

    // Settle metadata gain, position and the initial measured-HRTF kernel before
    // measuring musical timing. The listener hears none of this priming audio.
    let event = front_event();
    let mut reuse = Vec::new();
    for offset in (0..PRIME_SAMPLES).step_by(PRIME_BLOCK) {
        let n = PRIME_BLOCK.min(PRIME_SAMPLES - offset);
        let silence = vec![0.0f32; n];
        let events: &[SpatialChannelEvent] = if offset == 0 {
            std::slice::from_ref(&event)
        } else {
            &[]
        };
        let frame = renderer
            .render_frame(&silence, 1, events, reuse, false)
            .expect("prime stationary binaural object");
        reuse = frame.samples;
        reuse.clear();
    }

    let positions = event_samples(offbeat_phase);
    let total_samples = BEATS * BEAT_SAMPLES + SEARCH_TAIL;
    let mut input = vec![0.0f32; total_samples];
    for &sample in &positions {
        input[sample] = 1.0;
    }

    let mut output = Vec::with_capacity(total_samples * 2);
    let mut cursor = 0usize;
    while cursor < total_samples {
        let n = block_samples.min(total_samples - cursor);
        let frame = renderer
            .render_frame(&input[cursor..cursor + n], 1, &[], reuse, false)
            .expect("render binaural groove fixture");
        output.extend_from_slice(&frame.samples);
        reuse = frame.samples;
        reuse.clear();
        cursor += n;
    }

    (output, positions)
}

fn stereo_energy(output: &[f32], sample: usize) -> f32 {
    let l = output[sample * 2];
    let r = output[sample * 2 + 1];
    l * l + r * r
}

fn detected_peaks(output: &[f32], expected: &[usize]) -> Vec<usize> {
    let output_samples = output.len() / 2;
    expected
        .iter()
        .map(|&event| {
            // A measured HRIR may add a fixed direct-arrival/filter offset. Search
            // only forward from the authored event so one transient cannot steal
            // the previous event's window.
            let end = (event + SEARCH_TAIL).min(output_samples);
            (event..end)
                .max_by(|&a, &b| {
                    stereo_energy(output, a)
                        .partial_cmp(&stereo_energy(output, b))
                        .unwrap()
                })
                .expect("non-empty transient search window")
        })
        .collect()
}

fn assert_timing_preserved(offbeat_phase: f64, block_samples: usize) {
    let (output, authored) = render_click_train(offbeat_phase, block_samples);
    let heard = detected_peaks(&output, &authored);
    assert_eq!(authored.len(), heard.len());

    // HRTF/ITD may add a common latency. Relative timing is the musical
    // obligation, so remove the first-event offset before comparing trajectories.
    let authored_origin = authored[0] as isize;
    let heard_origin = heard[0] as isize;
    let mut max_relative_error = 0isize;
    for (&a, &h) in authored.iter().zip(&heard) {
        let expected = a as isize - authored_origin;
        let observed = h as isize - heard_origin;
        max_relative_error = max_relative_error.max((observed - expected).abs());
    }

    let beat0 = heard[0];
    let offbeat0 = heard[1];
    let beat1 = heard[2];
    let heard_phase = (offbeat0 - beat0) as f64 / (beat1 - beat0) as f64;

    eprintln!(
        "binaural groove fidelity: requested phase={offbeat_phase:.3}, heard phase={heard_phase:.6}, max relative event error={max_relative_error} sample(s), callback={block_samples}"
    );

    assert!(
        max_relative_error <= 1,
        "stationary binaural rendering changed relative event timing by {max_relative_error} samples"
    );
    let phase_tolerance = 1.0 / BEAT_SAMPLES as f64;
    assert!(
        (heard_phase - offbeat_phase).abs() <= phase_tolerance,
        "beat-relative timing changed: requested={offbeat_phase:.6}, heard={heard_phase:.6}"
    );
}

#[test]
fn straight_groove_timing_survives_stationary_binaural_render() {
    assert_timing_preserved(0.50, 240);
}

#[test]
fn swung_microtiming_survives_stationary_binaural_render() {
    assert_timing_preserved(0.62, 240);
}

#[test]
fn groove_timing_is_not_a_host_callback_artifact() {
    // Same straight groove through very different caller partitions. This is not
    // a waveform-null test; it protects the musical timing relation itself.
    assert_timing_preserved(0.50, 40);
    assert_timing_preserved(0.50, 960);
}
