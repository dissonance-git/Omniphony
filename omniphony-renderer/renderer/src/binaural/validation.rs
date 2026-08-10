//! End-to-end interaural time difference and measured-HRIR timing contracts.
//!
//! The engine deliberately separates two things that are easy to conflate:
//!
//! - **bulk/direct-arrival ITD**, supplied by `itd::ear_delays_seconds`;
//! - **direction-dependent HRTF spectral phase**, retained in each ear filter.
//!
//! A measured HRIR pair can therefore have a non-zero cross-correlation lag
//! after its direct arrivals have been aligned. Cross-correlation finds the lag
//! that makes two *spectrally different filters* resemble one another best; it
//! is not a direct measurement of residual bulk propagation delay.
//!
//! The validation below keeps those questions separate.

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
    // these tests measure the *engine's* analytic ITD path rather than phase
    // structure in a measured HRTF data set.
    let (left, right) = render_single_object_binaural(azimuth_deg, BLOCKS, HrirSource::Synthetic);
    estimate_lag_samples(&left, &right, MAX_LAG)
}

/// Model lag in samples, matching the sign convention of [`measured_lag`].
fn model_lag(azimuth_deg: f32) -> f32 {
    let (l, r) = ear_delays_seconds((azimuth_deg).to_radians(), 0.0, DEFAULT_HEAD_RADIUS_M);
    (r - l) * SAMPLE_RATE
}

/// Absorbs tiny convolution / interpolation measurement error.
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

/// This gate used to be ignored because identical `measured_lag` calls changed
/// under parallel test load. The renderer itself was not proven nondeterministic:
/// the fixture requested `HrirSource::Synthetic` from a renderer born with the
/// default SAF KEMAR grid, and HRIR source changes intentionally rebuild on a
/// worker thread. Rendering 64 tiny blocks as fast as possible was incorrectly
/// treated as enough *wall-clock* time for that worker to finish. Under load,
/// capture could begin on different HRIR grids.
///
/// `dsp_fixtures::scene::render_single_object_binaural` now issues the async
/// request, gives the control-plane rebuild a bounded settling interval, and
/// has its own repeated-render determinism regression. Antisymmetry is therefore
/// an active gate again instead of a scheduler-dependent deferral.
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

/// Detect the bulk direct-arrival anchor using the same declared amplitude
/// criterion as measured-HRIR preprocessing. This does not make the test
/// tautological: preprocessing aligns scattered measurements, while this gate
/// probes the *interpolated regular HRTF grid*. A different threshold (the old
/// validator used 10% while preprocessing used 15%) can relabel low-level
/// pre-ringing as an earlier arrival in one ear and manufacture a false ITD.
fn direct_arrival_index(ir: &[f32]) -> Option<usize> {
    let peak = ir.iter().map(|x| x.abs()).fold(0.0f32, f32::max);
    if peak <= 1.0e-9 {
        return None;
    }
    let threshold = peak * super::measured::ONSET_FRAC;
    ir.iter().position(|x| x.abs() >= threshold)
}

/// Measured HRTFs may retain different spectral phase/group delay in the two
/// ears. What must be absent before Omniphony adds analytic Woodworth ITD is a
/// *bulk direct-arrival offset*.
///
/// The previous version of this test used left/right cross-correlation and
/// therefore called legitimate spectral phase a residual ITD. Steam Audio is a
/// useful reference for the distinction: its HRTF database keeps full phase
/// information and tracks per-ear peak delays separately instead of defining
/// time alignment as a zero cross-correlation lag.
///
/// `MeasuredHrirData` onset-aligns every ear before spatial interpolation. This
/// test probes the resulting regular grid and verifies that the first meaningful
/// arrival remains aligned after interpolation, while allowing the later filter
/// shape to differ freely between ears.
#[test]
fn measured_hrir_direct_arrivals_are_time_aligned() {
    use super::hrir::{HRIR_LEN, HrirPair, HrirSet};
    use super::measured::MeasuredHrirData;

    let set = HrirSet::new(&MeasuredHrirData::saf_kemar(), 48_000);
    let mut pair = HrirPair {
        left: [0.0; HRIR_LEN],
        right: [0.0; HRIR_LEN],
    };

    let mut worst = (0.0f32, 0.0f32, 0isize);
    for az_i in -12..=12 {
        let az = az_i as f32 * 15.0;
        for el in [-30.0f32, 0.0, 30.0, 60.0] {
            set.at(az, el, &mut pair);
            let l = direct_arrival_index(&pair.left)
                .unwrap_or_else(|| panic!("no left direct arrival at az={az} el={el}"));
            let r = direct_arrival_index(&pair.right)
                .unwrap_or_else(|| panic!("no right direct arrival at az={az} el={el}"));
            let delta = l as isize - r as isize;
            if delta.abs() > worst.2.abs() {
                worst = (az, el, delta);
            }
            assert!(
                delta.abs() <= 1,
                "measured HRIR direct arrivals differ by {delta} samples at \
                 az={az:+.1}° el={el:+.1}° (left onset {l}, right onset {r}); \
                 analytic ITD is added separately, so bulk arrival offsets must \
                 be removed before interpolation"
            );
        }
    }
    println!(
        "[measure] measured HRIR direct-arrival alignment: worst {} sample(s) at az={:+.1}° el={:+.1}°",
        worst.2, worst.0, worst.1
    );
}
