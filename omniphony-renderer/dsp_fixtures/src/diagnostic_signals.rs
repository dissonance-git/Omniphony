//! Deterministic listening and validation stimuli adapted from Omniphony v0.5.
//!
//! Broadband noise is useful for general colouration, but it is intentionally a
//! weak localisation probe. This bank keeps the stronger complementary probes
//! in the dev-only fixture layer so CI, synthetic Windows smoke tests, and
//! listening experiments can ask *which spatial cue failed* without importing
//! Studio or object-test UI machinery into the product.
//!
//! - `PinkBursts`: repeated onsets for point localisation and motion stepping.
//! - `PinkLow`: <~1.5 kHz, dominated by interaural timing cues.
//! - `PinkHigh`: >~3 kHz, dominated by level/spectral cues.
//! - `ElevationBand`: ~7.1–9 kHz, where pinna/elevation structure is exposed.
//! - `Tone500`: level/panning-ripple probe.
//! - `Clicks`: impulse/seam/comb-filter/pre-echo probe.

use renderer::crossover::{BiquadState, LR4CrossoverBank};

const PINK_RAW_RMS: f32 = 1.717_1;
const BURST_ON_S: f32 = 0.030;
const BURST_PERIOD_S: f32 = 0.250;
const BURST_EDGE_S: f32 = 0.005;
const TONE_HZ: f32 = 500.0;
const CLICK_PERIOD_S: f32 = 0.250;
const LOW_HZ: f32 = 1500.0;
const HIGH_HZ: f32 = 3000.0;
const BAND_LO_HZ: f32 = 7100.0;
const BAND_HI_HZ: f32 = 9000.0;

// Same measured loudness compensation used by upstream v0.5. The filters below
// use the renderer's own LR4 implementation, so the transfer functions are the
// same 24 dB/octave Butterworth cascades rather than a fixture approximation.
const MAKEUP_LOW: f32 = 1.228;
const MAKEUP_HIGH: f32 = 2.207;
const MAKEUP_BAND: f32 = 9.755;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DiagnosticSignal {
    PinkNoise,
    PinkBursts,
    PinkLow,
    PinkHigh,
    ElevationBand,
    Tone500,
    Clicks,
}

#[derive(Debug, Clone)]
struct PinkNoise {
    poles: [f32; 3],
    rng: u32,
}

impl PinkNoise {
    /// Nominal peak-to-RMS ratio. Scaled output is also clamped, because a
    /// Gaussian-like noise source has no mathematical peak ceiling.
    const CREST: f32 = 4.5;

    fn new(seed: u32) -> Self {
        Self {
            poles: [0.0; 3],
            rng: if seed == 0 { 0x9E37_79B9 } else { seed },
        }
    }

    #[inline]
    fn white(&mut self) -> f32 {
        self.rng ^= self.rng << 13;
        self.rng ^= self.rng >> 17;
        self.rng ^= self.rng << 5;
        ((self.rng >> 8) as f32) * (2.0 / 16_777_216.0) - 1.0
    }

    #[inline]
    fn next_sample(&mut self) -> f32 {
        let white = self.white();
        self.poles[0] = 0.997_7 * self.poles[0] + white * 0.099_046;
        self.poles[1] = 0.963_0 * self.poles[1] + white * 0.296_516;
        self.poles[2] = 0.570_0 * self.poles[2] + white * 1.052_691;
        (self.poles[0] + self.poles[1] + self.poles[2] + white * 0.184_8) / PINK_RAW_RMS
    }

    fn reset(&mut self) {
        self.poles = [0.0; 3];
    }
}

pub fn level_divisor(signal: DiagnosticSignal) -> f32 {
    match signal {
        DiagnosticSignal::PinkNoise
        | DiagnosticSignal::PinkBursts
        | DiagnosticSignal::PinkLow
        | DiagnosticSignal::PinkHigh
        | DiagnosticSignal::ElevationBand
        | DiagnosticSignal::Tone500 => PinkNoise::CREST,
        DiagnosticSignal::Clicks => 1.0,
    }
}

/// Stateful deterministic generator for the diagnostic bank.
///
/// It is intentionally dev-only. Product render paths do not depend on this
/// crate, but installer/CI smoke tools may consume its generated PCM offline.
pub struct DiagnosticSignalGen {
    noise: PinkNoise,
    sample_rate: u32,
    tick: u32,
    phase: f32,
    low_bank: LR4CrossoverBank,
    low_states: Vec<BiquadState>,
    high_bank: LR4CrossoverBank,
    high_states: Vec<BiquadState>,
    band_bank: LR4CrossoverBank,
    band_states: Vec<BiquadState>,
}

impl DiagnosticSignalGen {
    pub fn new(seed: u32, sample_rate: u32) -> Self {
        let sample_rate = sample_rate.max(1);
        let low_bank = LR4CrossoverBank::new(&[LOW_HZ], sample_rate);
        let high_bank = LR4CrossoverBank::new(&[HIGH_HZ], sample_rate);
        let band_bank = LR4CrossoverBank::new(&[BAND_LO_HZ, BAND_HI_HZ], sample_rate);
        let low_states = vec![BiquadState::default(); low_bank.state_count()];
        let high_states = vec![BiquadState::default(); high_bank.state_count()];
        let band_states = vec![BiquadState::default(); band_bank.state_count()];
        Self {
            noise: PinkNoise::new(seed),
            sample_rate,
            tick: 0,
            phase: 0.0,
            low_bank,
            low_states,
            high_bank,
            high_states,
            band_bank,
            band_states,
        }
    }

    pub fn reset(&mut self) {
        self.noise.reset();
        self.tick = 0;
        self.phase = 0.0;
        for state in &mut self.low_states {
            *state = BiquadState::default();
        }
        for state in &mut self.high_states {
            *state = BiquadState::default();
        }
        for state in &mut self.band_states {
            *state = BiquadState::default();
        }
    }

    /// One nominally unit-RMS sample before peak-level scaling.
    pub fn next_sample(&mut self, signal: DiagnosticSignal) -> f32 {
        let rate = self.sample_rate as f32;
        match signal {
            DiagnosticSignal::PinkNoise => self.noise.next_sample(),
            DiagnosticSignal::PinkBursts => {
                let period = (BURST_PERIOD_S * rate) as u32;
                let on = (BURST_ON_S * rate) as u32;
                let edge = (BURST_EDGE_S * rate).max(1.0);
                let t = self.tick;
                self.tick += 1;
                if self.tick >= period.max(1) {
                    self.tick = 0;
                }
                let raw = self.noise.next_sample();
                if t >= on {
                    return 0.0;
                }
                let into = t as f32;
                let out_of = (on - t) as f32;
                let envelope = (into / edge).min(out_of / edge).clamp(0.0, 1.0);
                let envelope = 0.5 - 0.5 * (std::f32::consts::PI * envelope).cos();
                raw * envelope
            }
            DiagnosticSignal::PinkLow => {
                let raw = self.noise.next_sample();
                self.low_bank
                    .process_sample(raw, &mut self.low_states)
                    .get(0)
                    * MAKEUP_LOW
            }
            DiagnosticSignal::PinkHigh => {
                let raw = self.noise.next_sample();
                self.high_bank
                    .process_sample(raw, &mut self.high_states)
                    .get(1)
                    * MAKEUP_HIGH
            }
            DiagnosticSignal::ElevationBand => {
                let raw = self.noise.next_sample();
                self.band_bank
                    .process_sample(raw, &mut self.band_states)
                    .get(1)
                    * MAKEUP_BAND
            }
            DiagnosticSignal::Tone500 => {
                let sample = self.phase.sin() * std::f32::consts::SQRT_2;
                self.phase += std::f32::consts::TAU * TONE_HZ / rate;
                if self.phase >= std::f32::consts::TAU {
                    self.phase -= std::f32::consts::TAU;
                }
                sample
            }
            DiagnosticSignal::Clicks => {
                let period = (CLICK_PERIOD_S * rate) as u32;
                let t = self.tick;
                self.tick += 1;
                if self.tick >= period.max(1) {
                    self.tick = 0;
                }
                if t == 0 { 1.0 } else { 0.0 }
            }
        }
    }

    /// Generate a sample at an exact peak ceiling. Switching signal type changes
    /// the cue being tested rather than unexpectedly changing the safety bound.
    pub fn next_scaled(&mut self, signal: DiagnosticSignal, peak_level: f32) -> f32 {
        let peak = peak_level.abs();
        if peak == 0.0 {
            return 0.0;
        }
        let sample = self.next_sample(signal) * peak / level_divisor(signal);
        sample.clamp(-peak, peak)
    }

    pub fn render(&mut self, signal: DiagnosticSignal, frames: usize, peak_level: f32) -> Vec<f32> {
        (0..frames)
            .map(|_| self.next_scaled(signal, peak_level))
            .collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn run(signal: DiagnosticSignal, n: usize) -> Vec<f32> {
        let mut generator = DiagnosticSignalGen::new(0x85EB_CA6B, 48_000);
        (0..n).map(|_| generator.next_sample(signal)).collect()
    }

    fn rms(samples: &[f32]) -> f32 {
        (samples
            .iter()
            .map(|sample| (*sample as f64) * (*sample as f64))
            .sum::<f64>()
            / samples.len() as f64)
            .sqrt() as f32
    }

    #[test]
    fn bursts_have_true_silent_gaps_and_repeat() {
        let samples = run(DiagnosticSignal::PinkBursts, 48_000);
        let on = (BURST_ON_S * 48_000.0) as usize;
        let period = (BURST_PERIOD_S * 48_000.0) as usize;
        assert!(rms(&samples[on / 4..on * 3 / 4]) > 0.1);
        assert_eq!(
            samples[on + 10..period]
                .iter()
                .filter(|sample| **sample != 0.0)
                .count(),
            0,
            "burst gap must be mathematically silent"
        );
        assert!(rms(&samples[period + on / 4..period + on * 3 / 4]) > 0.1);
    }

    #[test]
    fn tone500_has_the_expected_quarter_cycle_clock() {
        let samples = run(DiagnosticSignal::Tone500, 97);
        assert!(samples[0].abs() < 1.0e-6);
        assert!((samples[24] - std::f32::consts::SQRT_2).abs() < 1.0e-4);
        assert!(samples[48].abs() < 1.0e-4);
        assert!((samples[72] + std::f32::consts::SQRT_2).abs() < 1.0e-4);
        assert!(samples[96].abs() < 1.0e-4);
    }

    #[test]
    fn band_limited_signals_reject_each_others_low_range() {
        let energy_below = |samples: &[f32], cutoff_hz: f32| {
            let a = (-std::f32::consts::TAU * cutoff_hz / 48_000.0).exp();
            let mut z = 0.0f32;
            let mut sum = 0.0f64;
            for sample in samples {
                z += (sample - z) * (1.0 - a);
                sum += (z as f64) * (z as f64);
            }
            (sum / samples.len() as f64).sqrt() as f32
        };
        let low = run(DiagnosticSignal::PinkLow, 240_000);
        let high = run(DiagnosticSignal::PinkHigh, 240_000);
        let low_energy = energy_below(&low, 500.0);
        let high_energy = energy_below(&high, 500.0);
        assert!(
            low_energy > high_energy * 4.0,
            "high-cue probe still carries too much low energy: low={low_energy}, high={high_energy}"
        );
    }

    #[test]
    fn continuous_probes_land_near_one_shared_listening_level() {
        let peak = 0.5f32;
        let scaled_rms = |signal| {
            rms(&run(signal, 240_000)) * peak / level_divisor(signal)
        };
        let reference = scaled_rms(DiagnosticSignal::PinkNoise);
        for signal in [
            DiagnosticSignal::PinkLow,
            DiagnosticSignal::PinkHigh,
            DiagnosticSignal::ElevationBand,
            DiagnosticSignal::Tone500,
        ] {
            let got = scaled_rms(signal);
            let db = 20.0 * (got / reference).log10();
            assert!(
                db.abs() < 2.25,
                "{signal:?} is {db:.2} dB from broadband pink at the same level"
            );
        }
    }

    #[test]
    fn scaled_bank_is_finite_and_respects_the_requested_peak() {
        for signal in [
            DiagnosticSignal::PinkNoise,
            DiagnosticSignal::PinkBursts,
            DiagnosticSignal::PinkLow,
            DiagnosticSignal::PinkHigh,
            DiagnosticSignal::ElevationBand,
            DiagnosticSignal::Tone500,
            DiagnosticSignal::Clicks,
        ] {
            let mut generator = DiagnosticSignalGen::new(0x1234_5678, 48_000);
            let samples = generator.render(signal, 96_000, 0.25);
            assert!(samples.iter().all(|sample| sample.is_finite()));
            let measured_peak = samples.iter().map(|sample| sample.abs()).fold(0.0, f32::max);
            assert!(measured_peak <= 0.250_001, "{signal:?} exceeded peak: {measured_peak}");
        }
    }
}
