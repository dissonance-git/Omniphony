//! Personal Dan Clark Noire X output correction used by the primary listening profile.
//!
//! This is deliberately separate from Omniphony's public Current-model foundation.
//! The coefficients independently implement the same RBJ biquad / shelf-corner
//! semantics used by the listener's former Equalizer APO profile. No Equalizer APO
//! runtime dependency is required. The correction is optional and defaults on to
//! preserve the established primary listening profile; the Windows tray can switch
//! it live without changing the Current spatial renderer.

use std::env;
use std::f64::consts::PI;
use std::fs;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

const GLOBAL_PREAMP_DB: f64 = -4.0;
const RIGHT_PREAMP_DB: f64 = -0.4;
const RIGHT_DELAY_MS: f64 = 0.02;
const SETTING_POLL_MS: u64 = 500;
const SETTING_FILE_NAME: &str = "personal-eq.txt";

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum FilterKind {
    HighPass,
    Peaking,
    LowShelf,
    HighShelf,
}

#[derive(Clone, Copy, Debug)]
struct FilterSpec {
    kind: FilterKind,
    frequency_hz: f64,
    gain_db: f64,
    q: f64,
}

impl FilterSpec {
    const fn high_pass(frequency_hz: f64, q: f64) -> Self {
        Self { kind: FilterKind::HighPass, frequency_hz, gain_db: 0.0, q }
    }

    const fn peaking(frequency_hz: f64, gain_db: f64, q: f64) -> Self {
        Self { kind: FilterKind::Peaking, frequency_hz, gain_db, q }
    }

    const fn low_shelf(frequency_hz: f64, gain_db: f64, q: f64) -> Self {
        Self { kind: FilterKind::LowShelf, frequency_hz, gain_db, q }
    }

    const fn high_shelf(frequency_hz: f64, gain_db: f64, q: f64) -> Self {
        Self { kind: FilterKind::HighShelf, frequency_hz, gain_db, q }
    }
}

const SHARED_FILTERS: [FilterSpec; 15] = [
    FilterSpec::high_pass(15.0, 0.6),
    FilterSpec::low_shelf(45.0, 3.5, 0.5),
    FilterSpec::peaking(30.0, 1.2, 0.8),
    FilterSpec::peaking(85.0, 2.0, 0.65),
    FilterSpec::peaking(155.0, 1.3, 0.75),
    FilterSpec::peaking(240.0, -0.2, 0.9),
    FilterSpec::peaking(420.0, 0.8, 0.7),
    FilterSpec::peaking(700.0, 0.8, 0.8),
    FilterSpec::peaking(1_200.0, 0.9, 0.7),
    FilterSpec::peaking(1_900.0, 0.5, 0.8),
    FilterSpec::peaking(2_800.0, -0.6, 0.6),
    FilterSpec::peaking(3_800.0, -2.2, 0.9),
    FilterSpec::peaking(4_800.0, -2.6, 1.1),
    FilterSpec::peaking(6_200.0, -0.9, 1.3),
    FilterSpec::high_shelf(7_200.0, -1.8, 0.7),
];

const RIGHT_FILTERS: [FilterSpec; 3] = [
    FilterSpec::peaking(180.0, -0.3, 0.9),
    FilterSpec::peaking(3_000.0, -1.1, 1.0),
    FilterSpec::high_shelf(6_200.0, -0.3, 0.7),
];

#[derive(Clone, Copy, Debug)]
struct Biquad {
    b0: f64,
    b1: f64,
    b2: f64,
    a1: f64,
    a2: f64,
    x1: f64,
    x2: f64,
    y1: f64,
    y2: f64,
}

impl Biquad {
    fn new(spec: FilterSpec, sample_rate_hz: u32) -> Self {
        let sample_rate = sample_rate_hz.max(1) as f64;
        let a = 10.0f64.powf(spec.gain_db / 40.0);
        let mut frequency_hz = spec.frequency_hz;

        // Equalizer APO treats ordinary LS/HS Fc as a corner frequency. When
        // Q is supplied, it first derives S only for the corner->center
        // frequency conversion, then still uses Q in the RBJ alpha term.
        if matches!(spec.kind, FilterKind::LowShelf | FilterKind::HighShelf) {
            let q = spec.q.max(f64::EPSILON);
            let s = 1.0 / (((1.0 / (q * q) - 2.0) / (a + 1.0 / a)) + 1.0);
            let center_factor = 10.0f64.powf(spec.gain_db.abs() / 80.0 / s);
            match spec.kind {
                FilterKind::LowShelf => frequency_hz *= center_factor,
                FilterKind::HighShelf => frequency_hz /= center_factor,
                _ => {}
            }
        }

        frequency_hz = frequency_hz.clamp(1.0e-6, sample_rate * 0.499_999);
        let omega = 2.0 * PI * frequency_hz / sample_rate;
        let sn = omega.sin();
        let cs = omega.cos();
        let alpha = sn / (2.0 * spec.q.max(f64::EPSILON));
        let beta = 2.0 * a.sqrt() * alpha;

        let (b0, b1, b2, a0, a1, a2) = match spec.kind {
            FilterKind::HighPass => (
                (1.0 + cs) / 2.0,
                -(1.0 + cs),
                (1.0 + cs) / 2.0,
                1.0 + alpha,
                -2.0 * cs,
                1.0 - alpha,
            ),
            FilterKind::Peaking => (
                1.0 + alpha * a,
                -2.0 * cs,
                1.0 - alpha * a,
                1.0 + alpha / a,
                -2.0 * cs,
                1.0 - alpha / a,
            ),
            FilterKind::LowShelf => (
                a * ((a + 1.0) - (a - 1.0) * cs + beta),
                2.0 * a * ((a - 1.0) - (a + 1.0) * cs),
                a * ((a + 1.0) - (a - 1.0) * cs - beta),
                (a + 1.0) + (a - 1.0) * cs + beta,
                -2.0 * ((a - 1.0) + (a + 1.0) * cs),
                (a + 1.0) + (a - 1.0) * cs - beta,
            ),
            FilterKind::HighShelf => (
                a * ((a + 1.0) + (a - 1.0) * cs + beta),
                -2.0 * a * ((a - 1.0) + (a + 1.0) * cs),
                a * ((a + 1.0) + (a - 1.0) * cs - beta),
                (a + 1.0) - (a - 1.0) * cs + beta,
                2.0 * ((a - 1.0) - (a + 1.0) * cs),
                (a + 1.0) - (a - 1.0) * cs - beta,
            ),
        };

        Self {
            b0: b0 / a0,
            b1: b1 / a0,
            b2: b2 / a0,
            a1: a1 / a0,
            a2: a2 / a0,
            x1: 0.0,
            x2: 0.0,
            y1: 0.0,
            y2: 0.0,
        }
    }

    fn process(&mut self, input: f32) -> f32 {
        let x = if input.is_finite() { input as f64 } else { 0.0 };
        let y = self.b0 * x + self.b1 * self.x1 + self.b2 * self.x2
            - self.a1 * self.y1 - self.a2 * self.y2;
        self.x2 = self.x1;
        self.x1 = x;
        self.y2 = self.y1;
        self.y1 = if y.abs() < 1.0e-30 { 0.0 } else { y };
        self.y1 as f32
    }
}

struct SampleDelay {
    samples: Vec<f32>,
    offset: usize,
}

impl SampleDelay {
    fn new(sample_rate_hz: u32, delay_ms: f64) -> Self {
        let count = ((sample_rate_hz as f64 * delay_ms / 1000.0) + 0.5).floor() as usize;
        Self { samples: vec![0.0; count], offset: 0 }
    }

    fn process(&mut self, input: f32) -> f32 {
        if self.samples.is_empty() {
            return input;
        }
        let output = self.samples[self.offset];
        self.samples[self.offset] = input;
        self.offset += 1;
        if self.offset == self.samples.len() {
            self.offset = 0;
        }
        output
    }

    #[cfg(test)]
    fn len(&self) -> usize { self.samples.len() }
}

pub(crate) struct NoireXPersonalEq {
    sample_rate_hz: u32,
    enabled: bool,
    setting_path: PathBuf,
    last_setting_check: Instant,
    global_gain: f32,
    right_gain: f32,
    shared: Vec<[Biquad; 2]>,
    right_only: Vec<Biquad>,
    right_delay: SampleDelay,
}

impl NoireXPersonalEq {
    pub(crate) fn new(sample_rate_hz: u32) -> Self {
        let setting_path = personal_eq_setting_path();
        let enabled = read_personal_eq_enabled(&setting_path);
        Self {
            sample_rate_hz,
            enabled,
            setting_path,
            last_setting_check: Instant::now(),
            global_gain: db_to_gain(GLOBAL_PREAMP_DB),
            right_gain: db_to_gain(RIGHT_PREAMP_DB),
            shared: build_shared_filters(sample_rate_hz),
            right_only: build_right_filters(sample_rate_hz),
            right_delay: SampleDelay::new(sample_rate_hz, RIGHT_DELAY_MS),
        }
    }

    fn refresh_enabled(&mut self) {
        if self.last_setting_check.elapsed() < Duration::from_millis(SETTING_POLL_MS) {
            return;
        }
        self.last_setting_check = Instant::now();
        let enabled = read_personal_eq_enabled(&self.setting_path);
        if enabled != self.enabled {
            self.enabled = enabled;
            self.shared = build_shared_filters(self.sample_rate_hz);
            self.right_only = build_right_filters(self.sample_rate_hz);
            self.right_delay = SampleDelay::new(self.sample_rate_hz, RIGHT_DELAY_MS);
        }
    }

    pub(crate) fn process_interleaved(&mut self, samples: &mut [f32]) {
        self.refresh_enabled();
        if !self.enabled {
            return;
        }

        for frame in samples.chunks_exact_mut(2) {
            let mut left = finite_or_zero(frame[0]) * self.global_gain;
            let mut right = finite_or_zero(frame[1]) * self.global_gain;

            for pair in &mut self.shared {
                left = pair[0].process(left);
                right = pair[1].process(right);
            }

            right *= self.right_gain;
            for filter in &mut self.right_only {
                right = filter.process(right);
            }
            right = self.right_delay.process(right);

            frame[0] = finite_or_zero(left);
            frame[1] = finite_or_zero(right);
        }
    }
}

fn build_shared_filters(sample_rate_hz: u32) -> Vec<[Biquad; 2]> {
    SHARED_FILTERS
        .iter()
        .map(|&spec| [Biquad::new(spec, sample_rate_hz), Biquad::new(spec, sample_rate_hz)])
        .collect()
}

fn build_right_filters(sample_rate_hz: u32) -> Vec<Biquad> {
    RIGHT_FILTERS
        .iter()
        .map(|&spec| Biquad::new(spec, sample_rate_hz))
        .collect()
}

fn personal_eq_setting_path() -> PathBuf {
    let root = env::var_os("ProgramData")
        .map(PathBuf::from)
        .unwrap_or_else(|| PathBuf::from(r"C:\ProgramData"));
    root.join("Omniphony").join(SETTING_FILE_NAME)
}

fn parse_personal_eq_enabled(text: &str) -> bool {
    !matches!(text.trim().to_ascii_lowercase().as_str(), "0" | "off" | "false" | "disabled")
}

fn read_personal_eq_enabled(path: &Path) -> bool {
    fs::read_to_string(path)
        .map(|text| parse_personal_eq_enabled(&text))
        .unwrap_or(true)
}

fn db_to_gain(db: f64) -> f32 {
    10.0f64.powf(db / 20.0) as f32
}

fn finite_or_zero(sample: f32) -> f32 {
    if sample.is_finite() { sample } else { 0.0 }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn setting_parser_defaults_to_enabled_semantics() {
        assert!(parse_personal_eq_enabled("1"));
        assert!(parse_personal_eq_enabled("on"));
        assert!(parse_personal_eq_enabled("anything-else"));
        assert!(!parse_personal_eq_enabled("0"));
        assert!(!parse_personal_eq_enabled("OFF"));
        assert!(!parse_personal_eq_enabled("false"));
    }

    #[test]
    fn right_compensation_delay_matches_equalizer_apo_rounding_at_48k() {
        let delay = SampleDelay::new(48_000, RIGHT_DELAY_MS);
        assert_eq!(delay.len(), 1);
    }

    #[test]
    fn right_channel_is_delayed_while_left_is_immediate() {
        let mut profile = NoireXPersonalEq::new(48_000);
        profile.enabled = true;
        let mut impulse = vec![0.0f32; 16];
        impulse[0] = 1.0;
        impulse[1] = 1.0;
        profile.process_interleaved(&mut impulse);
        assert!(impulse[0].abs() > 1.0e-6);
        assert_eq!(impulse[1], 0.0);
        assert!(impulse[3].abs() > 1.0e-6);
    }

    #[test]
    fn peaking_filter_hits_requested_center_gain() {
        let sample_rate = 48_000u32;
        let frequency = 1_000.0;
        let mut filter = Biquad::new(FilterSpec::peaking(frequency, 6.0, 1.0), sample_rate);
        let mut in_energy = 0.0f64;
        let mut out_energy = 0.0f64;
        let frames = sample_rate as usize;
        for frame in 0..frames {
            let sample = (2.0 * PI * frequency * frame as f64 / sample_rate as f64).sin() as f32 * 0.1;
            let output = filter.process(sample);
            if frame >= frames / 2 {
                in_energy += (sample as f64).powi(2);
                out_energy += (output as f64).powi(2);
            }
        }
        let gain = (out_energy / in_energy).sqrt();
        let expected = 10.0f64.powf(6.0 / 20.0);
        assert!((gain - expected).abs() < 0.02, "gain={gain} expected={expected}");
    }

    #[test]
    fn hot_profile_processing_remains_finite() {
        let mut profile = NoireXPersonalEq::new(48_000);
        profile.enabled = true;
        let mut samples = vec![4.0f32; 48_000 * 2 / 10];
        profile.process_interleaved(&mut samples);
        assert!(samples.iter().all(|sample| sample.is_finite()));
    }
}
