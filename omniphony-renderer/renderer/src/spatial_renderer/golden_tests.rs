//! Null tests: render fixed scenes and compare against committed goldens.
//!
//! These are the safety net for render-path refactors. A change that preserves
//! behaviour leaves the peak residual below the gate; a change that alters a
//! single sample audibly does not.
//!
//! Regenerate after an intended change:
//!   OMNIPHONY_BLESS_GOLDENS=1 cargo test -p renderer
//! and quote the printed residual in the pull request.

use dsp_fixtures::golden::assert_matches_golden;
use dsp_fixtures::scene::{RampMode, make_pcm, prepared, render_blocks};

/// 0.25 s at 48 kHz. `GAIN_SLEW_SECS` is 0.02, so this covers the 20 ms
/// fade-in plus ~230 ms of steady motion.
const BLOCKS: usize = 300;

/// Fresh movement events every 8th block, so the golden exercises both the
/// ramping path and the steady path.
const MOVE_EVERY: usize = 8;

const N_OBJECTS: usize = 32;

#[test]
fn null_speaker_714_32obj() {
    let (mut r, _) = prepared("7.1.4", N_OBJECTS, RampMode::Frame, true, false);
    let pcm = make_pcm(N_OBJECTS);
    let out = render_blocks(&mut r, &pcm, N_OBJECTS, BLOCKS, MOVE_EVERY);
    let channels = out.len() / (BLOCKS * dsp_fixtures::scene::BLOCK_SAMPLES);
    assert_eq!(channels, 12, "7.1.4 must render 12 speaker channels");
    assert_matches_golden("speaker_714_32obj", &out, channels);
}
