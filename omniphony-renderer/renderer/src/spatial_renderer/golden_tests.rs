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

/// 0.125 s at 48 kHz. `GAIN_SLEW_SECS` is 0.02, so this covers the 20 ms
/// fade-in plus ~105 ms of steady motion.
///
/// Halved from 300 blocks to hold the suite inside its time budget: at 300 the
/// three null tests took 2.20 s and `cargo test --workspace` reached 9.96 s
/// against a 10 s ceiling.
const BLOCKS: usize = 150;

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

#[test]
fn null_binaural_kemar() {
    let (mut r, pcm) = dsp_fixtures::scene::prepared_binaural(N_OBJECTS, RampMode::Frame);
    let out = render_blocks(&mut r, &pcm, N_OBJECTS, BLOCKS, MOVE_EVERY);
    let channels = out.len() / (BLOCKS * dsp_fixtures::scene::BLOCK_SAMPLES);
    assert_eq!(channels, 2, "the binaural path must render 2 channels");
    assert_matches_golden("binaural_kemar", &out, channels);
}

#[test]
fn null_crossover_bands() {
    let (mut r, pcm) = dsp_fixtures::scene::prepared_crossover(N_OBJECTS, RampMode::Frame);
    let out = render_blocks(&mut r, &pcm, N_OBJECTS, BLOCKS, MOVE_EVERY);
    let channels = out.len() / (BLOCKS * dsp_fixtures::scene::BLOCK_SAMPLES);
    assert_eq!(
        channels, 12,
        "crossover_layout is 7.1.4 band-limited — still 12 speakers"
    );
    assert_matches_golden("crossover_bands", &out, channels);
}

/// Half the channels never receive metadata.
///
/// The other scenes send events for every object, so none of them exercises
/// what a channel does with no cached metadata at all.
///
/// Honest scope: this scene does *not* discriminate the one case that motivated
/// it. Filtering the channel-state lookup on `initialized` — which makes the
/// "missing cached metadata, skipping" arm reachable — leaves this golden
/// unchanged, because such a channel still reads a default 0 dB gain and
/// renders the same either way. The scene is kept for the coverage it does add:
/// it is the only null test where the object count and the metadata count
/// differ, which is the shape a real stream takes when objects appear
/// mid-stream.
#[test]
fn null_partial_metadata() {
    let (mut r, _) = prepared("7.1.4", N_OBJECTS, RampMode::Frame, true, false);
    let pcm = make_pcm(N_OBJECTS);
    let out = dsp_fixtures::scene::render_blocks_partial_metadata(
        &mut r,
        &pcm,
        N_OBJECTS,
        N_OBJECTS / 2,
        BLOCKS,
        MOVE_EVERY,
    );
    let channels = out.len() / (BLOCKS * dsp_fixtures::scene::BLOCK_SAMPLES);
    assert_eq!(channels, 12, "7.1.4 must render 12 speaker channels");
    assert_matches_golden("partial_metadata", &out, channels);
}
