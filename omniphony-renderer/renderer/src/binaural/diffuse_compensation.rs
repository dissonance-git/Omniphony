//! Static diffuse-field colour compensation for binaural support.
//!
//! This is **not** headphone EQ and must not be placed on the authoritative
//! programme/master path. It compensates part of the direction-independent
//! spectral colour introduced by a known HRTF set when that rendered signal is
//! used as an additive spatial-support branch.
//!
//! The first profile is derived from Omniphony's measured SAF/KEMAR diffuse
//! fingerprint. The measured direction-weighted response, relative to 1 kHz,
//! rises by roughly +7 dB through 4-6 kHz and again near 10 kHz. Full diffuse-
//! field inversion would be too aggressive for a first listening candidate:
//! directional HRTF spectral structure is useful for localization. Therefore
//! this profile applies only a broad, bounded **partial** inverse.
//!
//! Design constraints:
//! - fixed / time-invariant gain only;
//! - identical topology in both ears;
//! - no compressor, limiter, AGC or content-dependent detector;
//! - causal minimum-phase IIR implementation;
//! - portable renderer code with no Windows dependency.

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy)]
struct Biquad {
    b0: f32,
    b1: f32,
    b2: f32,
    a1: f32,
    a2: f32,
    z1: f32,
    z2: f32,
}

impl Biquad {
    fn from_coefficients(b0: f32, b1: f32, b2: f32, a0: f32, a1: f32, a2: f32) -> Self {
        let inv_a0 = if a0.abs() > 1.0e-12 { 1.0 / a0 } else { 1.0 };
        Self {
            b0: b0 * inv_a0,
            b1: b1 * inv_a0,
            b2: b2 * inv_a0,
            a1: a1 * inv_a0,
            a2: a2 * inv_a0,
            z1: 0.0,
            z2: 0.0,
        }
    }

    fn peaking(sample_rate_hz: u32, frequency_hz: f32, q: f32, gain_db: f32) -> Self {
        let fs = sample_rate_hz.max(1) as f32;
        let f = frequency_hz.clamp(1.0, 0.49 * fs);
        let w0 = 2.0 * PI * f / fs;
        let cos_w0 = w0.cos();
        let alpha = w0.sin() / (2.0 * q.max(0.05));
        let a = 10.0_f32.powf(gain_db / 40.0);
        Self::from_coefficients(
            1.0 + alpha * a,
            -2.0 * cos_w0,
            1.0 - alpha * a,
            1.0 + alpha / a,
            -2.0 * cos_w0,
            1.0 - alpha / a,
        )
    }

    fn high_shelf(sample_rate_hz: u32, frequency_hz: f32, gain_db: f32) -> Self {
        let fs = sample_rate_hz.max(1) as f32;
        let f = frequency_hz.clamp(1.0, 0.49 * fs);
        let w0 = 2.0 * PI * f / fs;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let a = 10.0_f32.powf(gain_db / 40.0);
        let alpha = 0.5 * sin_w0 * (2.0_f32).sqrt();
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
        Self::from_coefficients(
            a * ((a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha),
            -2.0 * a * ((a - 1.0) + (a + 1.0) * cos_w0),
            a * ((a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha),
            (a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha,
            2.0 * ((a - 1.0) - (a + 1.0) * cos_w0),
            (a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha,
        )
    }

    #[inline]
    fn process(&mut self, sample: f32) -> f32 {
        let out = self.b0 * sample + self.z1;
        self.z1 = self.b1 * sample - self.a1 * out + self.z2;
        self.z2 = self.b2 * sample - self.a2 * out;
        out
    }

    fn reset(&mut self) {
        self.z1 = 0.0;
        self.z2 = 0.0;
    }
}

#[derive(Debug, Clone, Copy)]
struct EarCompensation {
    lower_pinna: Biquad,
    upper_pinna: Biquad,
    air_tail: Biquad,
}

impl EarCompensation {
    fn saf_kemar_partial(sample_rate_hz: u32) -> Self {
        // Measured common SAF/KEMAR diffuse response (relative to 1 kHz):
        // ~+7.3 dB at 4-6 kHz, ~+7.5 dB at 10 kHz, and ~+4 dB above 12 kHz.
        // These three broad sections remove roughly half to three-fifths of that
        // common rise rather than flattening the HRTF completely.
        Self {
            lower_pinna: Biquad::peaking(sample_rate_hz, 4_800.0, 0.65, -3.40),
            upper_pinna: Biquad::peaking(sample_rate_hz, 10_000.0, 0.80, -3.00),
            air_tail: Biquad::high_shelf(sample_rate_hz, 12_000.0, -1.20),
        }
    }

    #[inline]
    fn process(&mut self, sample: f32) -> f32 {
        let x = self.lower_pinna.process(sample);
        let x = self.upper_pinna.process(x);
        self.air_tail.process(x)
    }

    fn reset(&mut self) {
        self.lower_pinna.reset();
        self.upper_pinna.reset();
        self.air_tail.reset();
    }
}

/// Partial, static inverse of the common diffuse-field colour measured from the
/// embedded SAF/KEMAR HRTF set.
///
/// Use this on a *rendered binaural support branch*. Do not use it as a master
/// programme EQ. The master already represents the finished recording; this
/// stage exists only to keep the additive HRTF world from imposing its common
/// pinna colour a second time.
pub struct DiffuseFieldCompensator {
    left: EarCompensation,
    right: EarCompensation,
}

impl DiffuseFieldCompensator {
    pub fn saf_kemar_partial(sample_rate_hz: u32) -> Self {
        Self {
            left: EarCompensation::saf_kemar_partial(sample_rate_hz),
            right: EarCompensation::saf_kemar_partial(sample_rate_hz),
        }
    }

    /// Apply compensation in place to interleaved stereo support.
    pub fn process_interleaved_stereo_in_place(&mut self, samples: &mut [f32]) {
        for frame in samples.chunks_exact_mut(2) {
            let left = if frame[0].is_finite() { frame[0] } else { 0.0 };
            let right = if frame[1].is_finite() { frame[1] } else { 0.0 };
            frame[0] = self.left.process(left);
            frame[1] = self.right.process(right);
        }
    }

    pub fn reset_runtime_state(&mut self) {
        self.left.reset();
        self.right.reset();
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn measured_gain_db(frequency_hz: f32) -> f32 {
        let mut p = DiffuseFieldCompensator::saf_kemar_partial(48_000);
        let frames = 24_000usize;
        let settle = 8_000usize;
        let mut in_energy = 0.0f64;
        let mut out_energy = 0.0f64;
        let mut stereo = [0.0f32; 2];
        for i in 0..frames {
            let x = (2.0 * PI * frequency_hz * i as f32 / 48_000.0).sin() * 0.25;
            stereo[0] = x;
            stereo[1] = x;
            p.process_interleaved_stereo_in_place(&mut stereo);
            if i >= settle {
                in_energy += f64::from(x) * f64::from(x);
                out_energy += f64::from(stereo[0]) * f64::from(stereo[0]);
                assert!((stereo[0] - stereo[1]).abs() < 1.0e-6);
            }
        }
        10.0 * (out_energy / in_energy.max(1.0e-20)).log10() as f32
    }

    #[test]
    fn saf_profile_preserves_mid_reference_but_reduces_common_pinna_rise() {
        let at_1k = measured_gain_db(1_000.0);
        let at_5k = measured_gain_db(5_000.0);
        let at_10k = measured_gain_db(10_000.0);

        assert!(at_1k > -0.8, "1 kHz was over-corrected: {at_1k:.2} dB");
        assert!(
            (-5.5..=-3.0).contains(&at_5k),
            "5 kHz partial DFE outside target: {at_5k:.2} dB"
        );
        assert!(
            (-5.5..=-3.0).contains(&at_10k),
            "10 kHz partial DFE outside target: {at_10k:.2} dB"
        );
    }

    #[test]
    fn reset_clears_filter_history() {
        let mut p = DiffuseFieldCompensator::saf_kemar_partial(48_000);
        let mut impulse = [1.0f32, 1.0];
        p.process_interleaved_stereo_in_place(&mut impulse);
        p.reset_runtime_state();
        let mut silence = [0.0f32; 2];
        p.process_interleaved_stereo_in_place(&mut silence);
        assert_eq!(silence, [0.0, 0.0]);
    }
}
