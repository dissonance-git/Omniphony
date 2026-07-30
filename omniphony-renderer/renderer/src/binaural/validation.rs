//! End-to-end interaural time difference.
//!
//! This deliberately does **not** compare `itd::ear_delays_seconds` against
//! Woodworth's formula — `itd.rs` *implements* Woodworth, so such a test would
//! be circular and would prove nothing. Instead it measures the lag between the
//! left and right channels of an actual binaural render, which exercises the
//! delay lines, the convolver, the interpolation and the head-pose rotation as
//! a chain.
//!
//! Three properties, because per-ear HRIR group delay biases any raw comparison
//! against the model:
//!
//! 1. **Antisymmetry** — `lag(+az) = −lag(−az)`, and `lag(0°) ≈ 0`. Structural,
//!    so it is immune to that bias.
//! 2. **Monotonicity** — |lag| grows from 0° toward 90°.
//! 3. **Magnitude** — within ±3 samples of the model, the tolerance absorbing
//!    the group delay.

use dsp_fixtures::analysis::estimate_lag_samples;
use dsp_fixtures::scene::render_single_object_binaural;

use super::itd::{DEFAULT_HEAD_RADIUS_M, ear_delays_seconds};

/// 128 blocks of 40 samples = 5120 samples, ample for a ±64-sample search.
const BLOCKS: usize = 128;
const MAX_LAG: usize = 64;
const SAMPLE_RATE: f32 = 48_000.0;

/// Azimuths measured in the PR gate. 0 and ±90 bracket the range; the
/// intermediate angles catch a sign error that the extremes would not.
const AZIMUTHS: [f32; 7] = [0.0, 30.0, -30.0, 60.0, -60.0, 90.0, -90.0];

/// Measured lag in samples: positive means the right channel is delayed, so a
/// source on the right (positive azimuth) yields a negative value.
fn measured_lag(azimuth_deg: f32) -> f32 {
    let (left, right) = render_single_object_binaural(azimuth_deg, BLOCKS);
    estimate_lag_samples(&left, &right, MAX_LAG)
}

/// Model lag in samples, matching the sign convention of [`measured_lag`].
///
/// `ear_delays_seconds` returns `(left_delay, right_delay)`, both ≥ 0, with the
/// far ear carrying the delay. `right_delay − left_delay` is therefore positive
/// when the right ear is the far one, which is the same convention as the
/// cross-correlation estimate.
fn model_lag(azimuth_deg: f32) -> f32 {
    let (l, r) = ear_delays_seconds((azimuth_deg).to_radians(), 0.0, DEFAULT_HEAD_RADIUS_M);
    (r - l) * SAMPLE_RATE
}

#[test]
fn measure_binaural_itd() {
    // PHASE 1: report only. Task 11 converts this into assertions.
    for az in AZIMUTHS {
        let measured = measured_lag(az);
        let model = model_lag(az);
        println!(
            "[measure] itd az={az:+6.1}°: measured {measured:+7.3} samples \
             ({:+7.1} µs), model {model:+7.3} samples, delta {:+.3} samples \
             (target ±3)",
            measured / SAMPLE_RATE * 1e6,
            measured - model
        );
    }

    // Antisymmetry and monotonicity, reported as derived quantities.
    for az in [30.0f32, 60.0, 90.0] {
        let pos = measured_lag(az);
        let neg = measured_lag(-az);
        println!(
            "[measure] itd antisymmetry ±{az:.0}°: {pos:+.3} vs {neg:+.3}, \
             sum {:+.3} samples (target |sum| ≤ 1)",
            pos + neg
        );
    }
    let mags: Vec<f32> = [0.0f32, 30.0, 60.0, 90.0]
        .iter()
        .map(|az| measured_lag(*az).abs())
        .collect();
    println!(
        "[measure] itd monotonicity |lag| at 0/30/60/90°: {mags:?} \
         (target strictly increasing)"
    );
}
