//! Sound-neutral HRTF spectral measurement.
//!
//! The production HrirSet is broadband-energy normalized, but that does not
//! characterize common frequency-dependent colour across directions. This test
//! fingerprints the cos(elevation)-weighted diffuse response of the interpolated
//! SAF/KEMAR grid. A future support-only diffuse-field compensation stage should
//! be derived from this response rather than from hand-tuned music EQ.

use renderer::binaural::hrir::{HRIR_LEN, HrirPair, HrirSet};
use renderer::binaural::measured::MeasuredHrirData;

fn power_at(pair: &HrirPair, frequency_hz: f64, sample_rate: f64) -> f64 {
    let omega = std::f64::consts::TAU * frequency_hz / sample_rate;
    let ear_power = |h: &[f32; HRIR_LEN]| -> f64 {
        let mut re = 0.0f64;
        let mut im = 0.0f64;
        for (n, &x) in h.iter().enumerate() {
            let phase = omega * n as f64;
            re += x as f64 * phase.cos();
            im -= x as f64 * phase.sin();
        }
        re * re + im * im
    };
    0.5 * (ear_power(&pair.left) + ear_power(&pair.right))
}

fn diffuse_power_db(set: &HrirSet, frequency_hz: f64) -> f64 {
    // Sample a regular sphere through the same public interpolation path used by
    // the renderer. cos(elevation) compensates the denser latitude sampling.
    let mut weighted = 0.0f64;
    let mut weights = 0.0f64;
    let mut pair = HrirPair {
        left: [0.0; HRIR_LEN],
        right: [0.0; HRIR_LEN],
    };

    for elevation in (-40..=90).step_by(10) {
        let weight = (elevation as f64).to_radians().cos().max(0.0);
        for azimuth in (0..360).step_by(10) {
            set.at(azimuth as f32, elevation as f32, &mut pair);
            weighted += weight * power_at(&pair, frequency_hz, 48_000.0);
            weights += weight;
        }
    }

    let power = (weighted / weights.max(1.0e-12)).max(1.0e-20);
    10.0 * power.log10()
}

#[test]
fn saf_kemar_diffuse_spectral_profile_is_finite_and_repeatable() {
    let set = HrirSet::new(&MeasuredHrirData::saf_kemar(), 48_000);
    let frequencies = [
        500.0, 1_000.0, 2_000.0, 3_000.0, 4_000.0, 5_000.0, 6_000.0, 8_000.0, 10_000.0, 12_000.0,
        14_000.0, 16_000.0,
    ];
    let reference_db = diffuse_power_db(&set, 1_000.0);
    let mut min_relative = f64::INFINITY;
    let mut max_relative = f64::NEG_INFINITY;

    for frequency in frequencies {
        let relative = diffuse_power_db(&set, frequency) - reference_db;
        assert!(relative.is_finite(), "non-finite profile at {frequency} Hz");
        min_relative = min_relative.min(relative);
        max_relative = max_relative.max(relative);
        eprintln!("SAF_DFE {frequency:>7.0}Hz {relative:+7.2}dB");
    }

    let span = max_relative - min_relative;
    eprintln!("SAF_DFE sampled_span {span:.2}dB");
    // This is a corruption/sanity guard, not a tonal target. The actual profile
    // is intentionally measured rather than forced flat in this change.
    assert!(
        span < 40.0,
        "implausible SAF diffuse spectral span: {span:.2} dB"
    );
}
