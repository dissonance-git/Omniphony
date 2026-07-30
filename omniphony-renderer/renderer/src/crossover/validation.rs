//! LR4 crossover reconstruction flatness.
//!
//! `filter.rs` documents that each split sums to a 2nd-order allpass, and that
//! for N bands every already-emitted band is passed through the current
//! splitter's compensating allpass, so the total is a cascade of N−1 allpasses.
//! An allpass has magnitude exactly 1, therefore:
//!
//!   **the sum of all bands must be magnitude-flat at 0 dB.**
//!
//! Any deviation is coefficient or float error, not a design property. That is
//! what makes this a theory-derived threshold rather than a ratchet.
//!
//! **Phase is deliberately not asserted.** Allpass summing rotates phase by
//! design; asserting flat phase would be asserting the filter is broken.

use dsp_fixtures::analysis::magnitude_response_db;

use super::filter::{BiquadState, LR4CrossoverBank};

/// Impulse response length. LR4 ringing at the lowest cutoff takes tens of
/// milliseconds; truncating it leaks into the spectrum and reads as passband
/// ripple, so this must stay long.
const IR_LEN: usize = 32_768;

/// The cutoffs the shipped band-limited layout produces (see
/// `dsp_fixtures::scene::crossover_layout`): three band edges, four bands.
const DEFAULT_CUTOFFS: [f32; 3] = [80.0, 200.0, 500.0];

const SAMPLE_RATE: u32 = 48_000;

/// Sum of all band outputs for a unit impulse — the reconstruction IR.
fn reconstruction_ir(cutoffs: &[f32], sample_rate: u32) -> Vec<f32> {
    let bank = LR4CrossoverBank::new(cutoffs, sample_rate);
    let mut states = vec![BiquadState::default(); bank.state_count()];
    (0..IR_LEN)
        .map(|i| {
            let x = if i == 0 { 1.0 } else { 0.0 };
            let bands = bank.process_sample(x, &mut states);
            (0..bands.len()).map(|b| bands.get(b)).sum()
        })
        .collect()
}

/// Worst deviation from 0 dB over the asserted band, as `(freq_hz, dev_db)`.
///
/// The band is `[4·fc_min, min(20 kHz, 0.45·fs)]`: bounded below because the
/// truncated IR is unreliable near DC, and above so the 44.1 kHz case does not
/// assert flatness into the anti-alias region near Nyquist.
fn worst_flatness_deviation(cutoffs: &[f32], sample_rate: u32) -> (f32, f32) {
    let ir = reconstruction_ir(cutoffs, sample_rate);
    let resp = magnitude_response_db(&ir, sample_rate);
    let fc_min = cutoffs.iter().copied().fold(f32::INFINITY, f32::min);
    let lo = 4.0 * fc_min;
    let hi = 20_000.0f32.min(0.45 * sample_rate as f32);
    let mut worst = (0.0f32, 0.0f32);
    for (freq, db) in resp {
        if freq < lo || freq > hi {
            continue;
        }
        if db.abs() > worst.1.abs() {
            worst = (freq, db);
        }
    }
    worst
}

#[test]
fn measure_lr4_reconstruction_flatness() {
    // PHASE 1: report only. Task 11 converts this into an assertion.
    let (freq, dev) = worst_flatness_deviation(&DEFAULT_CUTOFFS, SAMPLE_RATE);
    println!(
        "[measure] lr4_flatness cutoffs={DEFAULT_CUTOFFS:?} fs={SAMPLE_RATE}: \
         worst deviation {dev:+.4} dB at {freq:.1} Hz (target ±0.25 dB)"
    );
}
