//! Reproducers for host-callback-size dependence in the binaural path.
//!
//! The portable-core contract says the audio timeline owns audible trajectories;
//! callback boundaries are transport details. These fixtures deliberately feed
//! the same continuous PCM and metadata timeline through different caller block
//! sizes so violations can be isolated before touching the realtime code.

use crate::residual::peak_residual_dbfs;
use crate::scene::{SAMPLE_RATE, build_renderer_binaural, pseudo};
use renderer::live_params::{OutputMode, RampMode};
use renderer::spatial_renderer::{SpatialChannelEvent, SpatialRenderer};
use renderer::speaker_layout::SpeakerLayout;

const TOTAL_SAMPLES: usize = 960; // 20 ms at 48 kHz = GAIN_SLEW_SECS.
const PRIME_BLOCK_SAMPLES: usize = 40;
// Prime beyond one complete 20 ms gain slew, not merely beyond a few host
// callbacks. This is intentionally expressed in samples because callback count
// is exactly the quantity under test.
const PRIME_SAMPLES: usize = 1_280;
const POSITION: [f64; 3] = [0.0, 1.0, 0.0]; // fixed front-centre object.

fn fixed_event(gain_db: i8) -> SpatialChannelEvent {
    SpatialChannelEvent {
        channel_idx: 0,
        is_bed: false,
        gain_db: Some(gain_db),
        ramp_length: Some(0),
        size: Some([0.0, 0.0, 0.0]),
        position: Some(POSITION),
        sample_pos: Some(0),
    }
}

fn renderer_primed_at_silence() -> SpatialRenderer {
    let mut renderer = build_renderer_binaural(
        SpeakerLayout::preset("7.1.4").expect("known preset"),
        true,
        false,
    );
    {
        let control = renderer.renderer_control();
        control.set_requested_ramp_mode(RampMode::Sample);
        let mut live = control.live.write();
        live.ramp_mode = RampMode::Sample;
        live.binaural.output_mode = OutputMode::Binaural;
        // Keep the default SAF KEMAR source. No async source switch is involved
        // in this reproducer, and the object never moves after priming.
    }

    // Settle position/HRTF state and, crucially, the entire 20 ms metadata-gain
    // slew while gain is silent. Every compared renderer receives the exact same
    // 40-sample priming sequence so construction/crossfade history is controlled.
    assert_eq!(PRIME_SAMPLES % PRIME_BLOCK_SAMPLES, 0);
    assert!(PRIME_SAMPLES > TOTAL_SAMPLES);
    let silent = fixed_event(-128);
    let mut reuse = Vec::new();
    for block_start in (0..PRIME_SAMPLES).step_by(PRIME_BLOCK_SAMPLES) {
        let pcm: Vec<f32> = (0..PRIME_BLOCK_SAMPLES)
            .map(|sample| pseudo((block_start + sample) as u64) * 0.25)
            .collect();
        let frame = renderer
            .render_frame(&pcm, 1, std::slice::from_ref(&silent), reuse, false)
            .expect("prime fixed binaural object");
        reuse = frame.samples;
        reuse.clear();
    }
    renderer
}

fn render_gain_step(block_samples: usize) -> Vec<f32> {
    assert!(block_samples > 0);
    assert_eq!(TOTAL_SAMPLES % block_samples, 0);

    let mut renderer = renderer_primed_at_silence();
    let step = fixed_event(0);
    let mut output = Vec::with_capacity(TOTAL_SAMPLES * 2);
    let mut reuse = Vec::new();

    for block_start in (0..TOTAL_SAMPLES).step_by(block_samples) {
        // The PCM is indexed by absolute timeline sample, not by callback. Every
        // block partition therefore sees the exact same continuous excitation.
        let pcm: Vec<f32> = (0..block_samples)
            .map(|sample| pseudo((10_000 + block_start + sample) as u64) * 0.25)
            .collect();
        let events: &[SpatialChannelEvent] = if block_start == 0 {
            std::slice::from_ref(&step)
        } else {
            &[]
        };
        let frame = renderer
            .render_frame(&pcm, 1, events, reuse, false)
            .expect("render binaural gain-step fixture");
        output.extend_from_slice(&frame.samples);
        reuse = frame.samples;
        reuse.clear();
    }

    output
}

/// Metadata/mute gain belongs to the audio sample timeline, not the host's
/// callback partition. The same 20 ms 0→unity slew must therefore render the
/// same headphone waveform whether the caller supplies 40-, 240-, or 960-sample
/// blocks.
///
/// `SpatialRenderer::ChannelState::slew_gain` remains the sole authority for the
/// gain trajectory. It publishes the gain at each callback's ending sample
/// boundary; `BinauralRenderer` retains the previous boundary and consumes the
/// linear segment per sample instead of applying the endpoint to the whole block.
#[test]
fn binaural_gain_is_invariant_to_host_block_size() {
    let fine = render_gain_step(40);
    let medium = render_gain_step(240);
    let whole_slew = render_gain_step(TOTAL_SAMPLES);

    assert_eq!(fine.len(), medium.len());
    assert_eq!(fine.len(), whole_slew.len());

    let fine_vs_medium = peak_residual_dbfs(&fine, &medium);
    let fine_vs_whole = peak_residual_dbfs(&fine, &whole_slew);
    eprintln!(
        "binaural gain callback invariance: 40-vs-240={fine_vs_medium:.2} dBFS, 40-vs-960={fine_vs_whole:.2} dBFS"
    );

    // The renderer's already-established deterministic binaural null floor is
    // around -100 dBFS. Keep a small margin for f32 endpoint accumulation across
    // the fine partition while still making any audible staircase a hard failure.
    const MAX_RESIDUAL_DBFS: f32 = -90.0;
    assert!(
        fine_vs_medium <= MAX_RESIDUAL_DBFS && fine_vs_whole <= MAX_RESIDUAL_DBFS,
        "binaural metadata gain depends on host callback size: 40-vs-240={fine_vs_medium:.2} dBFS, 40-vs-960={fine_vs_whole:.2} dBFS (required <= {MAX_RESIDUAL_DBFS:.1})"
    );

    // Guard the fixture itself: the measured gain transition is exactly 20 ms
    // at the declared 48 kHz rate and priming lasts strictly longer than it.
    assert_eq!(TOTAL_SAMPLES, (SAMPLE_RATE as f32 * 0.020) as usize);
    assert!(PRIME_SAMPLES > TOTAL_SAMPLES);
}
