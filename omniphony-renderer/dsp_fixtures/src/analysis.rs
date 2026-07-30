//! Measurement helpers for the acceptance tests: frequency response of an
//! impulse response, and interaural lag by cross-correlation.
//!
//! Both are verified below against analytically known answers. A validation
//! harness whose own measurements are wrong passes everything.

use realfft::RealFftPlanner;

use crate::residual::lin_to_dbfs;

/// Magnitude response of `ir`, as `(frequency_hz, magnitude_db)` for every
/// real-FFT bin (`ir.len()/2 + 1` of them).
///
/// No window is applied: callers pass an impulse response long enough that its
/// tail has decayed. Windowing an already-decayed IR would only smear the
/// response, and truncating an undecayed one shows up as passband ripple —
/// which is why the LR4 test uses 32768 samples.
pub fn magnitude_response_db(ir: &[f32], sample_rate: u32) -> Vec<(f32, f32)> {
    assert!(ir.len() >= 2, "need at least 2 samples for an FFT");
    let n = ir.len();
    let mut planner = RealFftPlanner::<f32>::new();
    let fft = planner.plan_fft_forward(n);
    let mut input = ir.to_vec();
    let mut spectrum = fft.make_output_vec();
    fft.process(&mut input, &mut spectrum)
        .expect("realfft forward transform");
    spectrum
        .iter()
        .enumerate()
        .map(|(k, c)| {
            let freq = k as f32 * sample_rate as f32 / n as f32;
            (freq, lin_to_dbfs(c.norm()))
        })
        .collect()
}

/// Lag, in samples, by which `right` is delayed relative to `left`.
///
/// Positive means `right[n] ≈ left[n - lag]`. For a binaural render this means
/// a source on the **right** returns a *negative* value: the contralateral
/// (left) ear is the delayed one.
///
/// The integer cross-correlation peak is refined by parabolic interpolation, so
/// sub-sample delays are recovered — necessary because ITD at 48 kHz is only
/// ~31 samples at full deflection and the interesting differences are fractions
/// of a sample.
pub fn estimate_lag_samples(left: &[f32], right: &[f32], max_lag: usize) -> f32 {
    assert_eq!(left.len(), right.len(), "channels must be equal length");
    assert!(
        left.len() > 2 * max_lag + 2,
        "signal ({}) too short for a ±{max_lag} lag search",
        left.len()
    );

    let n = left.len() as i64;
    let corr = |lag: i64| -> f64 {
        let mut acc = 0.0f64;
        let start = lag.max(0);
        let end = (n + lag).min(n);
        for i in start..end {
            acc += left[(i - lag) as usize] as f64 * right[i as usize] as f64;
        }
        acc
    };

    let ml = max_lag as i64;
    let mut best = 0i64;
    let mut best_v = f64::NEG_INFINITY;
    for lag in -ml..=ml {
        let v = corr(lag);
        if v > best_v {
            best_v = v;
            best = lag;
        }
    }

    // Parabolic refinement around the peak. Skipped at the search edges, where
    // one neighbour is unavailable.
    if best > -ml && best < ml {
        let cm = corr(best - 1);
        let cp = corr(best + 1);
        let denom = cm - 2.0 * best_v + cp;
        if denom.abs() > f64::EPSILON {
            return best as f32 + (0.5 * (cm - cp) / denom) as f32;
        }
    }
    best as f32
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Bandlimited unit pulse centred at `delay` samples (possibly fractional).
    /// A windowed sinc is the right test signal: it has a known sub-sample
    /// position, unlike a bare impulse.
    fn sinc_pulse(len: usize, delay: f64) -> Vec<f32> {
        (0..len)
            .map(|n| {
                let x = n as f64 - delay;
                let s = if x.abs() < 1e-12 {
                    1.0
                } else {
                    (std::f64::consts::PI * x).sin() / (std::f64::consts::PI * x)
                };
                // Hann window over the whole buffer keeps the edges tame.
                let w =
                    0.5 - 0.5 * (2.0 * std::f64::consts::PI * n as f64 / (len - 1) as f64).cos();
                (s * w) as f32
            })
            .collect()
    }

    #[test]
    fn unit_impulse_is_flat_at_zero_db() {
        let mut ir = vec![0.0f32; 1024];
        ir[0] = 1.0;
        let resp = magnitude_response_db(&ir, 48_000);
        assert_eq!(resp.len(), 1024 / 2 + 1, "realfft returns n/2+1 bins");
        for (freq, db) in &resp {
            assert!(
                db.abs() < 1e-3,
                "unit impulse must be 0 dB everywhere; got {db} dB at {freq} Hz"
            );
        }
    }

    #[test]
    fn magnitude_response_reports_a_known_gain() {
        let mut ir = vec![0.0f32; 512];
        ir[0] = 0.5; // -6.0206 dB, flat
        let resp = magnitude_response_db(&ir, 48_000);
        for (_, db) in &resp {
            assert!((db - -6.0206).abs() < 1e-2, "expected -6.02 dB, got {db}");
        }
    }

    #[test]
    fn bin_frequencies_span_dc_to_nyquist() {
        let ir = vec![0.0f32; 480];
        let resp = magnitude_response_db(&ir, 48_000);
        assert!((resp[0].0 - 0.0).abs() < 1e-6, "first bin is DC");
        let last = resp.last().expect("non-empty").0;
        assert!(
            (last - 24_000.0).abs() < 1.0,
            "last bin is Nyquist, got {last}"
        );
    }

    #[test]
    fn recovers_an_integer_lag() {
        let left = sinc_pulse(512, 100.0);
        let right = sinc_pulse(512, 107.0);
        // right is delayed by 7 samples relative to left.
        let lag = estimate_lag_samples(&left, &right, 64);
        assert!((lag - 7.0).abs() < 0.02, "expected +7.0, got {lag}");
    }

    #[test]
    fn recovers_a_fractional_lag() {
        let left = sinc_pulse(512, 100.0);
        let right = sinc_pulse(512, 107.5);
        let lag = estimate_lag_samples(&left, &right, 64);
        assert!((lag - 7.5).abs() < 0.1, "expected +7.5, got {lag}");
    }

    #[test]
    fn lag_sign_is_negative_when_left_is_delayed() {
        let left = sinc_pulse(512, 107.0);
        let right = sinc_pulse(512, 100.0);
        let lag = estimate_lag_samples(&left, &right, 64);
        assert!((lag - -7.0).abs() < 0.02, "expected -7.0, got {lag}");
    }

    #[test]
    fn identical_channels_have_zero_lag() {
        let s = sinc_pulse(512, 100.0);
        let lag = estimate_lag_samples(&s, &s, 64);
        assert!(lag.abs() < 0.02, "expected 0.0, got {lag}");
    }
}
