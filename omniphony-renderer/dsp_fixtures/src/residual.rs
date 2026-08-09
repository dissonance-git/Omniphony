//! Null-comparison and signal-fidelity arithmetic for Omniphony validation.
//!
//! The renderer needs two independent acceptance axes:
//!
//! ```text
//! spatial / perceptual improvement
//! +
//! measurable fidelity preservation
//! ```
//!
//! A spectacular spatial presentation is not a pass if it quietly flattens
//! crest factor, changes gain, clips peaks, or damages a claimed bypass path.
//! These helpers deliberately stay small and deterministic so every higher-level
//! listening experiment can report the same basic fidelity measurements.

/// Linear amplitude to dBFS. Returns [`f32::NEG_INFINITY`] for zero or
/// negative input rather than NaN, so a perfect null or silent signal reports
/// as `-inf`.
pub fn lin_to_dbfs(x: f32) -> f32 {
    if x <= 0.0 {
        f32::NEG_INFINITY
    } else {
        20.0 * x.log10()
    }
}

/// Peak absolute level of a signal, in dBFS.
pub fn peak_dbfs(x: &[f32]) -> f32 {
    lin_to_dbfs(x.iter().map(|v| v.abs()).fold(0.0f32, f32::max))
}

/// RMS level of a signal, in dBFS.
///
/// Accumulation uses `f64` so long captures do not lose meaningful precision.
pub fn rms_dbfs(x: &[f32]) -> f32 {
    if x.is_empty() {
        return f32::NEG_INFINITY;
    }
    let mean_square = x
        .iter()
        .map(|&v| {
            let v = v as f64;
            v * v
        })
        .sum::<f64>()
        / x.len() as f64;
    lin_to_dbfs(mean_square.sqrt() as f32)
}

/// Crest factor in dB (`peak - RMS`).
///
/// A meaningful drop in crest factor between matched signals can reveal
/// transient flattening or hidden limiting even when average loudness appears
/// unchanged. Silence returns 0 rather than NaN.
pub fn crest_factor_db(x: &[f32]) -> f32 {
    let peak = peak_dbfs(x);
    let rms = rms_dbfs(x);
    if !peak.is_finite() && !rms.is_finite() {
        0.0
    } else {
        peak - rms
    }
}

/// Arithmetic DC offset of a signal.
pub fn dc_offset(x: &[f32]) -> f32 {
    if x.is_empty() {
        return 0.0;
    }
    (x.iter().map(|&v| v as f64).sum::<f64>() / x.len() as f64) as f32
}

/// Difference in RMS level, `candidate - reference`, in dB.
///
/// Returns 0 for two silent signals, +∞ when only the candidate has energy,
/// and −∞ when only the reference has energy. This is useful for matched-level
/// gates without hiding silence behind an arbitrary epsilon.
pub fn rms_level_delta_db(reference: &[f32], candidate: &[f32]) -> f32 {
    let a = rms_dbfs(reference);
    let b = rms_dbfs(candidate);
    match (a.is_finite(), b.is_finite()) {
        (false, false) => 0.0,
        (false, true) => f32::INFINITY,
        (true, false) => f32::NEG_INFINITY,
        (true, true) => b - a,
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SignalFidelityStats {
    pub peak_dbfs: f32,
    pub rms_dbfs: f32,
    pub crest_factor_db: f32,
    pub dc_offset: f32,
}

/// Common scalar fidelity summary for one mono/interleaved capture.
///
/// Frequency response, phase/group delay and interaural measurements remain
/// separate because collapsing them into one score would hide failure modes.
pub fn signal_fidelity_stats(x: &[f32]) -> SignalFidelityStats {
    SignalFidelityStats {
        peak_dbfs: peak_dbfs(x),
        rms_dbfs: rms_dbfs(x),
        crest_factor_db: crest_factor_db(x),
        dc_offset: dc_offset(x),
    }
}

/// Largest absolute sample-by-sample difference, in dBFS. This is the strict
/// null-comparison gate.
///
/// Panics if the slices differ in length so a shape mismatch cannot silently
/// truncate the comparison.
pub fn peak_residual_dbfs(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "residual needs equal-length signals");
    let peak = a
        .iter()
        .zip(b)
        .map(|(x, y)| (x - y).abs())
        .fold(0.0f32, f32::max);
    lin_to_dbfs(peak)
}

/// RMS of the difference, in dBFS. Reported alongside the peak for context;
/// not itself a strict null gate. Accumulates in `f64` so long renders do not
/// lose precision in the sum.
pub fn rms_residual_dbfs(a: &[f32], b: &[f32]) -> f32 {
    assert_eq!(a.len(), b.len(), "residual needs equal-length signals");
    if a.is_empty() {
        return f32::NEG_INFINITY;
    }
    let sum_sq: f64 = a
        .iter()
        .zip(b)
        .map(|(x, y)| {
            let d = (*x - *y) as f64;
            d * d
        })
        .sum();
    lin_to_dbfs((sum_sq / a.len() as f64).sqrt() as f32)
}

/// Locate the largest deviation in an interleaved signal:
/// `(frame, channel, delta)`.
///
/// Used only for failure messages. A bare "golden mismatch" is not actionable.
pub fn worst_deviation(a: &[f32], b: &[f32], channels: usize) -> (usize, usize, f32) {
    assert_eq!(a.len(), b.len(), "residual needs equal-length signals");
    assert!(channels > 0, "channels must be non-zero");
    let mut best = (0usize, 0usize, 0.0f32);
    for (i, (x, y)) in a.iter().zip(b).enumerate() {
        let d = (x - y).abs();
        if d > best.2 {
            best = (i / channels, i % channels, d);
        }
    }
    best
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identical_signals_have_negative_infinite_residual() {
        let a = vec![0.1, -0.5, 0.9, 0.0];
        let r = peak_residual_dbfs(&a, &a);
        assert_eq!(
            r,
            f32::NEG_INFINITY,
            "identical inputs must be -inf dBFS, not NaN or a finite value"
        );
    }

    #[test]
    fn constant_offset_gives_the_analytic_value() {
        // A difference of exactly 1e-6 is exactly -120 dBFS.
        let a = vec![0.0f32; 64];
        let b = vec![1e-6f32; 64];
        let peak = peak_residual_dbfs(&a, &b);
        assert!(
            (peak - -120.0).abs() < 0.01,
            "expected -120 dBFS for a 1e-6 offset, got {peak}"
        );
        // Every sample differs by the same amount, so RMS equals peak.
        let rms = rms_residual_dbfs(&a, &b);
        assert!(
            (rms - -120.0).abs() < 0.01,
            "expected -120 dBFS RMS for a constant offset, got {rms}"
        );
    }

    #[test]
    fn peak_dbfs_reads_full_scale_as_zero() {
        assert!((peak_dbfs(&[0.0, -1.0, 0.5]) - 0.0).abs() < 1e-6);
        assert_eq!(peak_dbfs(&[0.0, 0.0]), f32::NEG_INFINITY);
    }

    #[test]
    fn rms_dbfs_has_known_values() {
        // Constant 0.5 has RMS 0.5 = -6.0206 dBFS.
        let x = vec![0.5f32; 32];
        assert!((rms_dbfs(&x) - -6.0206).abs() < 0.01);

        // Alternating ±1 is full-scale RMS.
        let y = [1.0f32, -1.0, 1.0, -1.0];
        assert!(rms_dbfs(&y).abs() < 1e-6);
    }

    #[test]
    fn crest_factor_detects_peak_to_average_structure() {
        // [1, 0, 0, 0] has RMS 0.5 and peak 1 → 6.0206 dB crest factor.
        let x = [1.0f32, 0.0, 0.0, 0.0];
        assert!((crest_factor_db(&x) - 6.0206).abs() < 0.01);
        assert_eq!(crest_factor_db(&[0.0, 0.0]), 0.0);
    }

    #[test]
    fn dc_offset_reports_signed_mean() {
        assert!((dc_offset(&[0.5, 0.5, -0.5, 0.5]) - 0.25).abs() < 1e-6);
        assert_eq!(dc_offset(&[]), 0.0);
    }

    #[test]
    fn rms_level_delta_tracks_known_gain() {
        let reference = [1.0f32, -1.0, 1.0, -1.0];
        let candidate = [0.5f32, -0.5, 0.5, -0.5];
        assert!((rms_level_delta_db(&reference, &candidate) - -6.0206).abs() < 0.01);
        assert_eq!(rms_level_delta_db(&[0.0], &[0.0]), 0.0);
        assert!(rms_level_delta_db(&[0.0], &[0.1]).is_infinite());
    }

    #[test]
    fn fidelity_summary_is_consistent_with_components() {
        let x = [1.0f32, 0.0, -0.5, 0.0];
        let stats = signal_fidelity_stats(&x);
        assert_eq!(stats.peak_dbfs, peak_dbfs(&x));
        assert_eq!(stats.rms_dbfs, rms_dbfs(&x));
        assert_eq!(stats.crest_factor_db, crest_factor_db(&x));
        assert_eq!(stats.dc_offset, dc_offset(&x));
    }

    #[test]
    fn worst_deviation_locates_frame_and_channel() {
        // 3 channels, 4 frames. Plant the largest error at frame 2, channel 1.
        let a = vec![0.0f32; 12];
        let mut b = vec![0.0f32; 12];
        b[1 * 3] = 0.01; // frame 1, channel 0 — smaller
        b[2 * 3 + 1] = 0.50; // frame 2, channel 1 — largest
        let (frame, channel, delta) = worst_deviation(&a, &b, 3);
        assert_eq!((frame, channel), (2, 1));
        assert!((delta - 0.50).abs() < 1e-6, "got {delta}");
    }
}
