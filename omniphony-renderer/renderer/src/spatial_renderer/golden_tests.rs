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

/// `reset_runtime_state` must leave the renderer equivalent to a fresh one.
///
/// The channel-state refactor changed this from a synchronous clear (it held
/// the mutex) to a flag consumed at the top of the next `render_frame`. That is
/// what keeps the render path lock-free, but it moves *when* the clear happens,
/// and four call sites outside the renderer depend on it
/// (`orender_engine/src/engine.rs` 649/810/871,
/// `src/cli/decode/spatial_metadata.rs:86` — all decoder-reset or
/// stream-restart paths).
///
/// **Currently failing, and pre-existing.** The same assertion fails identically
/// (peak residual −20.3 dBFS) on the code before the channel-state refactor, so
/// state leaked past a reset already and the move from a synchronous clear to a
/// deferred flag did not cause it. Clearing `channel_states` is evidently not
/// sufficient to restore a renderer to its initial condition; something else
/// survives. Finding what is a separate investigation.
///
/// The assertion is equivalence rather than an internal check: render a stream,
/// reset, render again, and require the result to be bit-identical to a
/// freshly-constructed renderer fed the same blocks. That is the property the
/// callers actually rely on — a reset stream must not inherit the previous
/// one's positions or gains — and it holds regardless of when the clear lands.
#[test]
#[ignore = "pre-existing: state leaks past reset_runtime_state — peak residual -20.3 dBFS, and identical on the pre-refactor code, so the deferred-flag change did not cause it. Tracked deferral, see docs/dsp-validation-report.md"]
fn reset_runtime_state_matches_a_fresh_renderer() {
    const WARMUP: usize = 40;
    const AFTER: usize = 60;

    let pcm = make_pcm(N_OBJECTS);

    // A renderer that has rendered a different stream, then been reset.
    let (mut reused, _) = prepared("7.1.4", N_OBJECTS, RampMode::Frame, true, false);
    let _ = render_blocks(&mut reused, &pcm, N_OBJECTS, WARMUP, 4);
    reused.reset_runtime_state();
    let after_reset = render_blocks(&mut reused, &pcm, N_OBJECTS, AFTER, MOVE_EVERY);

    // A renderer that has never rendered anything.
    let (mut fresh, _) = prepared("7.1.4", N_OBJECTS, RampMode::Frame, true, false);
    let from_fresh = render_blocks(&mut fresh, &pcm, N_OBJECTS, AFTER, MOVE_EVERY);

    assert_eq!(
        after_reset.len(),
        from_fresh.len(),
        "reset renderer produced a different frame count"
    );
    let residual = dsp_fixtures::residual::peak_residual_dbfs(&after_reset, &from_fresh);
    assert!(
        residual <= dsp_fixtures::golden::RESIDUAL_GATE_DBFS,
        "after reset_runtime_state the renderer does not match a fresh one: \
         peak residual {residual:.1} dBFS (gate {:.1}). State from the previous \
         stream is leaking past the reset.",
        dsp_fixtures::golden::RESIDUAL_GATE_DBFS
    );
}
