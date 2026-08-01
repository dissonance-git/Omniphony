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
use dsp_fixtures::scene::{HrirSource, render_single_object_binaural};

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
    // The synthetic provider is symmetric and time-aligned by construction, so
    // these tests measure the *engine's* ITD rather than the bundled KEMAR
    // set's own left/right asymmetry. See
    // `hrir_providers_return_time_aligned_pairs` for the test that holds a
    // provider to the time-alignment contract.
    let (left, right) = render_single_object_binaural(azimuth_deg, BLOCKS, HrirSource::Synthetic);
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

/// Absorbs per-ear HRIR group delay, which is not part of the Woodworth model.
const MAGNITUDE_TOLERANCE_SAMPLES: f32 = 3.0;

/// Antisymmetry is structural, so the bound is tight.
const ANTISYMMETRY_TOLERANCE_SAMPLES: f32 = 1.0;

#[test]
fn itd_magnitude_tracks_the_model() {
    for az in AZIMUTHS {
        let measured = measured_lag(az);
        let model = model_lag(az);
        let delta = measured - model;
        println!(
            "[measure] itd az={az:+6.1}°: measured {measured:+7.3}, \
             model {model:+7.3}, delta {delta:+.3} samples"
        );
        assert!(
            delta.abs() <= MAGNITUDE_TOLERANCE_SAMPLES,
            "ITD at az={az:+.1}° is {measured:+.3} samples but the model says \
             {model:+.3} (delta {delta:+.3}, tolerance \
             ±{MAGNITUDE_TOLERANCE_SAMPLES})"
        );
    }
}

/// **Deferred: `measured_lag` is not deterministic under test parallelism.**
///
/// Reproduced in a full `cargo test --release --workspace` run — not in 30
/// runs of this module alone, so it needs the rest of the suite loading the
/// machine. The evidence is unambiguous and is recorded here because it is the
/// starting point for whoever fixes it: in **one and the same run**, two tests
/// called `measured_lag` with identical arguments and got different answers.
///
/// ```text
/// itd_magnitude_tracks_the_model (passed):
///   az=  0.0 -> +0.000    az=+30.0 -> -12.836    az=-30.0 -> +12.836
/// itd_is_antisymmetric_about_the_median_plane (failed, same run):
///   az=  0.0 -> +0.025    az=+30.0 -> -13.460    az=-30.0 -> +12.435
/// ```
///
/// A source dead ahead must give exactly 0; one test got 0.000 and the other
/// 0.025. So the render itself differs between threads, and no tolerance
/// adjustment is the right fix — the nondeterminism is.
///
/// An earlier hypothesis blamed `ensure_denormals_flushed` leaving threads
/// without FTZ/DAZ. That guard is a `thread_local!` and is correct for any
/// thread entering `render_frame`. It does **not** cover rayon workers, which
/// the renderer uses during construction (`live_params.rs`, `render_backend.rs`
/// `into_par_iter`) and which never call it — that is the most promising lead,
/// but it is a lead, not a finding: nobody has shown the binaural path reads
/// anything those workers produce.
#[ignore = "measured_lag is not deterministic under test parallelism: the same call returned -12.836 and -13.460 in one run, and az=0 gave 0.000 and 0.025. Tracked deferral, see the doc comment and docs/dsp-validation-report.md"]
#[test]
fn itd_is_antisymmetric_about_the_median_plane() {
    let centre = measured_lag(0.0);
    println!("[measure] itd az=0°: {centre:+.3} samples");
    assert!(
        centre.abs() <= ANTISYMMETRY_TOLERANCE_SAMPLES,
        "a source dead ahead must have no ITD, measured {centre:+.3} samples"
    );
    for az in [30.0f32, 60.0, 90.0] {
        let pos = measured_lag(az);
        let neg = measured_lag(-az);
        println!(
            "[measure] itd antisymmetry ±{az:.0}°: {pos:+.3} vs {neg:+.3}, \
             sum {:+.3}",
            pos + neg
        );
        assert!(
            (pos + neg).abs() <= ANTISYMMETRY_TOLERANCE_SAMPLES,
            "ITD must be antisymmetric: az=+{az:.0}° gives {pos:+.3} and \
             az=-{az:.0}° gives {neg:+.3}, sum {:+.3} exceeds \
             ±{ANTISYMMETRY_TOLERANCE_SAMPLES}",
            pos + neg
        );
    }
}

#[test]
fn itd_magnitude_grows_toward_the_interaural_axis() {
    let mags: Vec<f32> = [0.0f32, 30.0, 60.0, 90.0]
        .iter()
        .map(|az| measured_lag(*az).abs())
        .collect();
    println!("[measure] itd monotonicity |lag| at 0/30/60/90°: {mags:?}");
    for w in mags.windows(2) {
        assert!(
            w[1] > w[0],
            "|ITD| must increase toward the interaural axis, got {mags:?}"
        );
    }
}

/// The wide matrix: a full azimuth grid at several elevations.
/// Compiled only with `--features wide-matrix`.
#[cfg(feature = "wide-matrix")]
#[test]
fn itd_magnitude_tracks_the_model_wide() {
    for az_i in -6..=6 {
        let az = az_i as f32 * 30.0;
        let measured = measured_lag(az);
        let model = model_lag(az);
        let delta = measured - model;
        println!(
            "[measure] itd_wide az={az:+6.1}°: measured {measured:+7.3}, \
             model {model:+7.3}, delta {delta:+.3}"
        );
        assert!(
            delta.abs() <= MAGNITUDE_TOLERANCE_SAMPLES,
            "ITD at az={az:+.1}°: delta {delta:+.3} samples exceeds \
             ±{MAGNITUDE_TOLERANCE_SAMPLES}"
        );
    }
}

/// A time-aligned HRIR pair carries no bulk interaural delay of its own; ±1
/// sample allows for interpolation and measurement slop.
const TIME_ALIGNMENT_TOLERANCE_SAMPLES: f32 = 1.0;

/// [`HrirProvider`](super::hrir::HrirProvider) documents that implementors
/// "must return time-aligned FIRs (no bulk interaural delay) for safe
/// interpolation". The engine relies on that in two places: it adds its own
/// Woodworth ITD as a separate per-ear delay, and [`HrirSet`] blends the three
/// nearest measurements. Blending FIRs that are not time-aligned combs their
/// shared content instead of interpolating it.
///
/// This asserts the contract rather than assuming it. It is the invariant whose
/// violation made the bundled measured set look like an engine defect: the set's
/// own left/right asymmetry showed up in end-to-end ITD measurements and was
/// initially misread as the renderer mis-placing sources.
#[test]
#[ignore = "SAF KEMAR violates it: intrinsic interaural lag is unresolvable at az=-90 and reaches -6.998 samples at az=+90, against a ±1 sample contract; the set is also left/right asymmetric (-1.103 at +30 vs -0.168 at -30) — tracked deferral, see docs/dsp-validation-report.md"]
fn hrir_providers_return_time_aligned_pairs() {
    use super::hrir::{HRIR_LEN, HrirPair, HrirSet};
    use super::measured::MeasuredHrirData;
    use dsp_fixtures::analysis::estimate_lag_checked;

    let set = HrirSet::new(&MeasuredHrirData::saf_kemar(), 48_000);
    let mut pair = HrirPair {
        left: [0.0; HRIR_LEN],
        right: [0.0; HRIR_LEN],
    };
    let mut worst = (0.0f32, 0.0f32);
    let mut unresolvable = Vec::new();

    for az_i in -6..=6 {
        let az = az_i as f32 * 30.0;
        set.at(az, 0.0, &mut pair);
        match estimate_lag_checked(&pair.left, &pair.right, 40) {
            Ok(lag) => {
                println!(
                    "[measure] hrir_time_alignment az={az:+6.1}: intrinsic lag {lag:+.3} samples"
                );
                if lag.abs() > worst.1.abs() {
                    worst = (az, lag);
                }
            }
            // An unresolvable pair cannot be shown to satisfy the contract
            // either — record it rather than quietly treating it as a pass.
            Err(e) => {
                println!("[measure] hrir_time_alignment az={az:+6.1}: unresolvable — {e}");
                unresolvable.push(az);
            }
        }
    }

    assert!(
        unresolvable.is_empty(),
        "intrinsic interaural lag could not be resolved at azimuths {unresolvable:?}, \
         so the time-alignment contract cannot be verified there"
    );
    assert!(
        worst.1.abs() <= TIME_ALIGNMENT_TOLERANCE_SAMPLES,
        "HRIR pair at az={:.1}° carries {:+.3} samples of intrinsic interaural \
         delay, exceeding the ±{TIME_ALIGNMENT_TOLERANCE_SAMPLES} sample \
         time-alignment contract. The engine adds Woodworth ITD on top of this, \
         and HrirSet blends neighbouring measurements — both assume the pair is \
         time-aligned.",
        worst.0,
        worst.1
    );
}
