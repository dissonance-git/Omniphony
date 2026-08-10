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

    // Settle position/HRTF state while metadata gain is silent. Using the same
    // 40-sample priming sequence for every compared renderer removes HRTF
    // construction/crossfade history from the variable under test.
    let silent = fixed_event(-128);
    let mut reuse = Vec::new();
    for block in 0..16 {
        let pcm: Vec<f32> = (0..40)
            .map(|sample| pseudo((block * 40 + sample) as u64) * 0.25)
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
        let events = if block_start == 0 {
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

/// This is intentionally a **known-defect reproducer**, not the desired gate.
///
/// Current `SpatialRenderer` computes `(gain_start, gain_step)` correctly but
/// stores only the block-end value in `binaural_gain_buf`. `BinauralRenderer`
/// then multiplies every sample in that callback by the one scalar. A 40-sample
/// callback therefore creates a fine staircase; a 240- or 960-sample callback
/// creates a much coarser one.
///
/// Run manually while repairing the hot path:
///
/// ```text
/// cargo test -p dsp_fixtures known_defect_binaural_gain_is_block_quantized -- --ignored --nocapture
/// ```
///
/// Once the renderer consumes the `ChannelState` gain trajectory per sample,
/// replace this assertion with the inverse portability gate (fine/coarse outputs
/// equal within the calibrated binaural floating-point tolerance) and remove
/// `#[ignore]`.
#[test]
#[ignore = "known defect: binaural gain is currently quantized to caller blocks; invert this into a live equivalence gate after the hot-path fix"]
fn known_defect_binaural_gain_is_block_quantized() {
    let fine = render_gain_step(40);
    let medium = render_gain_step(240);
    let whole_slew = render_gain_step(TOTAL_SAMPLES);

    assert_eq!(fine.len(), medium.len());
    assert_eq!(fine.len(), whole_slew.len());

    let fine_vs_medium = peak_residual_dbfs(&fine, &medium);
    let fine_vs_whole = peak_residual_dbfs(&fine, &whole_slew);
    eprintln!(
        "known binaural gain block dependence: 40-vs-240={fine_vs_medium:.2} dBFS, 40-vs-960={fine_vs_whole:.2} dBFS"
    );

    // This is deliberately a defect-presence assertion. The difference should
    // be enormous compared with the ~-100 dBFS cross-host binaural null floor.
    // If it no longer is, the implementation improved and this reproducer must
    // be converted into the positive equivalence gate described above.
    assert!(
        fine_vs_medium > -60.0 || fine_vs_whole > -60.0,
        "the known block-quantized gain defect no longer reproduces; convert this into the positive invariance gate"
    );

    // Guard the fixture itself: the gain slew is exactly 20 ms at the declared
    // 48 kHz test rate, so the logical experiment duration tracks the product
    // contract rather than an accidental constant.
    assert_eq!(TOTAL_SAMPLES, (SAMPLE_RATE as f32 * 0.020) as usize);
}
