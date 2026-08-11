//! Small, source-coherent foundation/body enhancement for stereo music.
//!
//! This is deliberately not a second headphone EQ and not a spatial bass path.
//! The finished stereo master remains explicit. The processor emits only the
//! additive delta needed to give the protected master more pressure, body and
//! density before it is linearly combined with Omniphony's spatial support.
//!
//! Design constraints:
//! - no compression, limiting, saturation or dynamics-dependent gain;
//! - no fake LFE and no HRTF rendering of the low-frequency foundation;
//! - identical filter topology in left and right channels so stereo relations
//!   remain intact;
//! - minimum-phase IIR shaping only, with downstream headroom owned by the host.

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy)]
pub struct MusicFoundationTuning {
    /// Broad low-frequency pressure / mass.
    pub low_shelf_db: f32,
    /// Upper-bass / lower-mid body.
    pub body_db: f32,
    /// Small midrange density correction.
    pub density_db: f32,
    /// Gentle upper-presence relaxation; negative values reduce emphasis.
    pub presence_shelf_db: f32,
}

impl Default for MusicFoundationTuning {
    fn default() -> Self {
        // Physical listening established a stronger invariant than the first
        // conservative pass: Omniphony ON must never feel weaker than OFF in
        // bass pressure, kick weight or drum body. Keep this coherent and
        // non-spatial rather than trying to recover impact with fake LFE or
        // extra room energy.
        Self {
            low_shelf_db: 2.30,
            body_db: 1.20,
            density_db: 0.50,
            presence_shelf_db: -0.35,
        }
    }
}

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
    fn identity() -> Self {
        Self {
            b0: 1.0,
            b1: 0.0,
            b2: 0.0,
            a1: 0.0,
            a2: 0.0,
            z1: 0.0,
            z2: 0.0,
        }
    }

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
        if gain_db.abs() < 1.0e-6 {
            return Self::identity();
        }
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

    fn low_shelf(sample_rate_hz: u32, frequency_hz: f32, gain_db: f32) -> Self {
        if gain_db.abs() < 1.0e-6 {
            return Self::identity();
        }
        let fs = sample_rate_hz.max(1) as f32;
        let f = frequency_hz.clamp(1.0, 0.49 * fs);
        let w0 = 2.0 * PI * f / fs;
        let cos_w0 = w0.cos();
        let sin_w0 = w0.sin();
        let a = 10.0_f32.powf(gain_db / 40.0);
        let alpha = 0.5 * sin_w0 * (2.0_f32).sqrt();
        let two_sqrt_a_alpha = 2.0 * a.sqrt() * alpha;
        Self::from_coefficients(
            a * ((a + 1.0) - (a - 1.0) * cos_w0 + two_sqrt_a_alpha),
            2.0 * a * ((a - 1.0) - (a + 1.0) * cos_w0),
            a * ((a + 1.0) - (a - 1.0) * cos_w0 - two_sqrt_a_alpha),
            (a + 1.0) + (a - 1.0) * cos_w0 + two_sqrt_a_alpha,
            -2.0 * ((a - 1.0) + (a + 1.0) * cos_w0),
            (a + 1.0) + (a - 1.0) * cos_w0 - two_sqrt_a_alpha,
        )
    }

    fn high_shelf(sample_rate_hz: u32, frequency_hz: f32, gain_db: f32) -> Self {
        if gain_db.abs() < 1.0e-6 {
            return Self::identity();
        }
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
}

struct ChannelFoundation {
    pressure: Biquad,
    body: Biquad,
    density: Biquad,
    presence: Biquad,
}

impl ChannelFoundation {
    fn new(sample_rate_hz: u32, tuning: MusicFoundationTuning) -> Self {
        Self {
            pressure: Biquad::low_shelf(sample_rate_hz, 85.0, tuning.low_shelf_db),
            body: Biquad::peaking(sample_rate_hz, 240.0, 0.80, tuning.body_db),
            density: Biquad::peaking(sample_rate_hz, 800.0, 0.70, tuning.density_db),
            presence: Biquad::high_shelf(sample_rate_hz, 4_500.0, tuning.presence_shelf_db),
        }
    }

    fn process(&mut self, sample: f32) -> f32 {
        let x = self.pressure.process(sample);
        let x = self.body.process(x);
        let x = self.density.process(x);
        self.presence.process(x)
    }
}

/// Emits only the additive stereo delta. The authoritative master remains a
/// separate path and is summed with this delta later in the host.
pub struct MusicFoundationProcessor {
    left: ChannelFoundation,
    right: ChannelFoundation,
}

impl MusicFoundationProcessor {
    pub fn new(sample_rate_hz: u32) -> Self {
        Self::with_tuning(sample_rate_hz, MusicFoundationTuning::default())
    }

    pub fn with_tuning(sample_rate_hz: u32, tuning: MusicFoundationTuning) -> Self {
        Self {
            left: ChannelFoundation::new(sample_rate_hz, tuning),
            right: ChannelFoundation::new(sample_rate_hz, tuning),
        }
    }

    pub fn process_interleaved_delta(&mut self, input: &[f32]) -> Vec<f32> {
        if input.len() < 2 || input.len() % 2 != 0 {
            return Vec::new();
        }
        let mut delta = Vec::with_capacity(input.len());
        for frame in input.chunks_exact(2) {
            let left = if frame[0].is_finite() { frame[0] } else { 0.0 };
            let right = if frame[1].is_finite() { frame[1] } else { 0.0 };
            delta.push(self.left.process(left) - left);
            delta.push(self.right.process(right) - right);
        }
        delta
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn sine(frequency_hz: f32, frames: usize) -> Vec<f32> {
        let mut out = Vec::with_capacity(frames * 2);
        for i in 0..frames {
            let x = (2.0 * PI * frequency_hz * i as f32 / 48_000.0).sin() * 0.25;
            out.extend_from_slice(&[x, x]);
        }
        out
    }

    fn rms(samples: &[f32]) -> f32 {
        let sum = samples.iter().map(|x| x * x).sum::<f32>();
        (sum / samples.len().max(1) as f32).sqrt()
    }

    #[test]
    fn default_foundation_adds_low_frequency_mass_without_channel_skew() {
        let input = sine(60.0, 16_384);
        let mut p = MusicFoundationProcessor::new(48_000);
        let delta = p.process_interleaved_delta(&input);
        let shaped: Vec<f32> = input.iter().zip(delta.iter()).map(|(a, b)| a + b).collect();
        let start = 4_096 * 2;
        assert!(rms(&shaped[start..]) > rms(&input[start..]));
        for frame in delta[start..].chunks_exact(2) {
            assert!((frame[0] - frame[1]).abs() < 1.0e-6);
        }
    }

    #[test]
    fn default_foundation_adds_body_at_240_hz() {
        let input = sine(240.0, 16_384);
        let mut p = MusicFoundationProcessor::new(48_000);
        let delta = p.process_interleaved_delta(&input);
        let shaped: Vec<f32> = input.iter().zip(delta.iter()).map(|(a, b)| a + b).collect();
        let start = 4_096 * 2;
        assert!(rms(&shaped[start..]) > rms(&input[start..]) * 1.08);
    }

    #[test]
    fn default_foundation_relaxes_upper_presence_slightly() {
        let input = sine(10_000.0, 16_384);
        let mut p = MusicFoundationProcessor::new(48_000);
        let delta = p.process_interleaved_delta(&input);
        let shaped: Vec<f32> = input.iter().zip(delta.iter()).map(|(a, b)| a + b).collect();
        let start = 4_096 * 2;
        assert!(rms(&shaped[start..]) < rms(&input[start..]));
    }

    #[test]
    fn zero_tuning_is_effectively_transparent() {
        let tuning = MusicFoundationTuning {
            low_shelf_db: 0.0,
            body_db: 0.0,
            density_db: 0.0,
            presence_shelf_db: 0.0,
        };
        let input = sine(997.0, 4_096);
        let mut p = MusicFoundationProcessor::with_tuning(48_000, tuning);
        let delta = p.process_interleaved_delta(&input);
        assert!(delta.iter().all(|x| x.abs() < 1.0e-7));
    }
}
