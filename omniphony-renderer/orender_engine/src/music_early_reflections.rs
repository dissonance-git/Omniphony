//! Bounded measured-HRTF early-reflection field for the music path.
//!
//! A literal measured-HRTF convolution for every first-order image of every
//! virtual support source would multiply the FIR count by the speaker count.
//! This module keeps first-order image timing and wall tone per support lane,
//! groups contributions by wall, then applies exactly six measured SAF/KEMAR
//! HRTFs.
//!
//! The transient-aware excitation in this file is intentionally narrower than
//! transient separation or transient reshaping. Each support lane compares a
//! fast energy envelope with a slow energy envelope. A sharp positive rise may
//! briefly increase only that lane's signal entering the early-reflection delay
//! bank. The protected stereo master, coherent foundation, primary support
//! render and late room field are not modified here.

use anyhow::bail;
use renderer::binaural::convolver::EarConvolver;
use renderer::binaural::hrir::{HRIR_LEN, HrirPair, HrirSet};
use renderer::binaural::itd;
use renderer::binaural::measured::MeasuredHrirData;
use renderer::binaural::reflections::{self, NUM_REFLECTIONS};
use renderer::delay_line::DelayLine;
use renderer::music_field::MUSIC_FIELD_CHANNELS;

const CURRENT_UNIT_SCALE_M: f32 = 9.25;
const CURRENT_ROOM_M: [f32; 3] = [23.0, 32.0, 21.0];
const CURRENT_REFLECTION_LEVEL: f32 = 0.36;
const REF_DISTANCE_M: f32 = 1.0;
const MAX_DISTANCE_GAIN: f32 = 4.0;
const MIN_DISTANCE_M: f32 = 0.25;
const RING_CAPACITY_S: f32 = 0.25;
const TONE_SPLIT_HZ: f32 = 4_000.0;
const GENERIC_WALL_HF_AMPLITUDE: f32 = 0.84;
const EXTRA_PATH_HF_DECAY_PER_M: f32 = 0.020;
const ITD_MAX_S: f32 = 0.003;

// Listening-candidate transient law. Fast/slow energy comparison follows the
// established onset-detection idea that a transient is a positive change in
// short-time energy, not simply a loud sample. Values are deliberately bounded
// and local to each existing spatial-support lane so a drum event cannot turn
// the whole mixture's room up at once.
const TRANSIENT_FAST_MS: f32 = 3.0;
const TRANSIENT_SLOW_MS: f32 = 45.0;
const TRANSIENT_RELEASE_MS: f32 = 20.0;
const TRANSIENT_MIN_RMS: f32 = 0.0015;
const TRANSIENT_RISE_THRESHOLD: f32 = 0.75;
const TRANSIENT_FULL_RISE: f32 = 3.0;
const TRANSIENT_MAX_GAIN_DB: f32 = 2.5;

// The legacy analytic reflection panner has total L+R power 4/3 for a unit
// reflection gain (`SHADOW=0.5`, denominator 1.5). A diffuse-normalized HRIR
// pair is approximately 2.0 total-ear power. Scale by sqrt((4/3)/2) so the
// measured-HRTF field primarily changes directional spectral information rather
// than simply making the early field louder.
const HRTF_POWER_MATCH: f32 = 0.816_496_6;

/// Canonical 8.1.4.4 lane directions in `MUSIC_FIELD_CHANNELS` order:
/// L R C LFE Ls Rs Lb Rb Cb Tfl Tfr Tbl Tbr Bfl Bfr Bbl Bbr.
/// Positive azimuth is right, positive elevation is up. The stereo Current
/// extractor leaves C/LFE/Cb/lower lanes EMPTY, but the geometry is complete so
/// authored richer ingress can use the same portable frame later.
const LANE_DIRECTIONS_DEG: [(f32, f32); MUSIC_FIELD_CHANNELS] = [
    (-30.0, 0.0),
    (30.0, 0.0),
    (0.0, 0.0),
    (0.0, 0.0),
    (-90.0, 0.0),
    (90.0, 0.0),
    (-140.0, 0.0),
    (140.0, 0.0),
    (180.0, 0.0),
    (-40.0, 60.0),
    (40.0, 60.0),
    (-140.0, 60.0),
    (140.0, 60.0),
    (-40.0, -60.0),
    (40.0, -60.0),
    (-140.0, -60.0),
    (140.0, -60.0),
];

#[derive(Clone, Copy, Default)]
struct PathTap {
    delay_samples: f32,
    gain: f32,
    hf_gain: f32,
    tone_state: f32,
}

#[derive(Clone, Copy)]
struct TransientReflectionExciter {
    fast_energy: f32,
    slow_energy: f32,
    envelope: f32,
    fast_alpha: f32,
    slow_alpha: f32,
    release_coeff: f32,
    max_delta: f32,
}

impl TransientReflectionExciter {
    fn new(sample_rate_hz: u32) -> Self {
        let sample_rate_hz = sample_rate_hz.max(1) as f32;
        let one_pole_alpha =
            |time_ms: f32| 1.0 - (-1.0 / (0.001 * time_ms.max(0.01) * sample_rate_hz)).exp();
        Self {
            fast_energy: 0.0,
            slow_energy: 0.0,
            envelope: 0.0,
            fast_alpha: one_pole_alpha(TRANSIENT_FAST_MS),
            slow_alpha: one_pole_alpha(TRANSIENT_SLOW_MS),
            release_coeff: (-1.0 / (0.001 * TRANSIENT_RELEASE_MS.max(0.01) * sample_rate_hz)).exp(),
            max_delta: 10.0_f32.powf(TRANSIENT_MAX_GAIN_DB / 20.0) - 1.0,
        }
    }

    #[inline]
    fn gain(&mut self, input: f32) -> f32 {
        let energy = input * input;
        self.fast_energy += self.fast_alpha * (energy - self.fast_energy);
        self.slow_energy += self.slow_alpha * (energy - self.slow_energy);

        let target = if self.fast_energy > TRANSIENT_MIN_RMS * TRANSIENT_MIN_RMS {
            let positive_rise = (self.fast_energy - self.slow_energy).max(0.0);
            let relative_rise = positive_rise / (self.slow_energy + 1.0e-9);
            ((relative_rise - TRANSIENT_RISE_THRESHOLD)
                / (TRANSIENT_FULL_RISE - TRANSIENT_RISE_THRESHOLD))
                .clamp(0.0, 1.0)
        } else {
            0.0
        };

        if target > self.envelope {
            self.envelope = target;
        } else {
            self.envelope *= self.release_coeff;
        }

        1.0 + self.max_delta * self.envelope
    }
}

struct SourceReflectionBank {
    ring: Vec<f32>,
    write_pos: usize,
    taps: [PathTap; NUM_REFLECTIONS],
    tone_alpha: f32,
    air_state: f32,
    air_coeff: f32,
    transient: TransientReflectionExciter,
}

impl SourceReflectionBank {
    fn new(sample_rate_hz: u32, source_m: [f32; 3]) -> (Self, [[f32; 3]; NUM_REFLECTIONS]) {
        let cap = (RING_CAPACITY_S * sample_rate_hz as f32).ceil() as usize + 2;
        let direct_distance = norm(source_m).max(MIN_DISTANCE_M);
        let images = reflections::first_order_images(source_m, CURRENT_ROOM_M);
        let mut taps = [PathTap::default(); NUM_REFLECTIONS];
        let mut directions = [[0.0f32; 3]; NUM_REFLECTIONS];

        for (i, image) in images.iter().copied().enumerate() {
            let image_distance = norm(image).max(MIN_DISTANCE_M);
            let relative_path_m = (image_distance - direct_distance).max(0.0);
            let delay_s = relative_path_m / reflections::speed_of_sound();
            let distance_gain = (REF_DISTANCE_M / image_distance).clamp(0.0, MAX_DISTANCE_GAIN);
            let hf_gain = (GENERIC_WALL_HF_AMPLITUDE
                * (-EXTRA_PATH_HF_DECAY_PER_M * relative_path_m).exp())
            .clamp(0.45, 0.90);
            taps[i] = PathTap {
                delay_samples: (delay_s * sample_rate_hz as f32).clamp(0.0, (cap - 2) as f32),
                gain: CURRENT_REFLECTION_LEVEL * distance_gain,
                hf_gain,
                tone_state: 0.0,
            };
            directions[i] = normalized(image);
        }

        let air_coeff = if direct_distance > 3.0 {
            let fc = (20_000.0 * (-0.05 * (direct_distance - 3.0)).exp()).max(2_000.0);
            (-std::f32::consts::TAU * fc / sample_rate_hz as f32).exp()
        } else {
            0.0
        };

        (
            Self {
                ring: vec![0.0; cap],
                write_pos: 0,
                taps,
                tone_alpha: 1.0
                    - (-std::f32::consts::TAU * TONE_SPLIT_HZ / sample_rate_hz as f32).exp(),
                air_state: 0.0,
                air_coeff,
                transient: TransientReflectionExciter::new(sample_rate_hz),
            },
            directions,
        )
    }

    #[inline]
    fn process(&mut self, mut input: f32) -> [f32; NUM_REFLECTIONS] {
        // Only the signal entering the early-reflection delay bank receives the
        // transient-dependent gain. The direct master and primary spatial field
        // are outside this module and therefore cannot be reshaped by it.
        input *= self.transient.gain(input);

        if self.air_coeff > 0.0 {
            self.air_state += (input - self.air_state) * (1.0 - self.air_coeff);
            input = self.air_state;
        }

        let cap = self.ring.len();
        self.ring[self.write_pos] = input;
        let mut out = [0.0f32; NUM_REFLECTIONS];
        for (i, tap) in self.taps.iter_mut().enumerate() {
            let delayed = read_frac(&self.ring, cap, self.write_pos, tap.delay_samples);
            tap.tone_state += (delayed - tap.tone_state) * self.tone_alpha;
            let toned = tap.tone_state + tap.hf_gain * (delayed - tap.tone_state);
            out[i] = tap.gain * toned;
        }
        self.write_pos += 1;
        if self.write_pos >= cap {
            self.write_pos = 0;
        }
        out
    }
}

struct HrtfWallBus {
    delay_l: DelayLine,
    delay_r: DelayLine,
    conv_l: EarConvolver,
    conv_r: EarConvolver,
}

impl HrtfWallBus {
    fn new(sample_rate_hz: u32, hrir: &HrirSet, direction: [f32; 3]) -> Self {
        let az = direction[0].atan2(direction[1]);
        let horiz = (direction[0] * direction[0] + direction[1] * direction[1]).sqrt();
        let el = direction[2].atan2(horiz);
        let mut pair = HrirPair {
            left: [0.0; HRIR_LEN],
            right: [0.0; HRIR_LEN],
        };
        hrir.at(az.to_degrees(), el.to_degrees(), &mut pair);

        let max_itd = (ITD_MAX_S * sample_rate_hz as f32).ceil() as usize;
        let mut delay_l = DelayLine::new(max_itd);
        let mut delay_r = DelayLine::new(max_itd);
        let (itd_l, itd_r) = itd::ear_delays_seconds(az, el, itd::DEFAULT_HEAD_RADIUS_M);
        delay_l.set_target_ms(itd_l * 1_000.0, sample_rate_hz);
        delay_r.set_target_ms(itd_r * 1_000.0, sample_rate_hz);

        let mut conv_l = EarConvolver::new();
        let mut conv_r = EarConvolver::new();
        conv_l.set_coeffs(&pair.left);
        conv_r.set_coeffs(&pair.right);
        Self {
            delay_l,
            delay_r,
            conv_l,
            conv_r,
        }
    }

    #[inline]
    fn process(&mut self, input: f32) -> (f32, f32) {
        let x = input * HRTF_POWER_MATCH;
        (
            self.conv_l.process(self.delay_l.process(x)),
            self.conv_r.process(self.delay_r.process(x)),
        )
    }
}

pub(crate) struct HrtfEarlyReflectionField {
    sources: Vec<Option<SourceReflectionBank>>,
    buses: [HrtfWallBus; NUM_REFLECTIONS],
}

impl HrtfEarlyReflectionField {
    pub(crate) fn new(sample_rate_hz: u32) -> Self {
        let mut sources = Vec::with_capacity(MUSIC_FIELD_CHANNELS);
        let mut direction_sum = [[0.0f32; 3]; NUM_REFLECTIONS];
        let mut direction_weight = [0.0f32; NUM_REFLECTIONS];

        for channel in 0..MUSIC_FIELD_CHANNELS {
            if matches!(channel, 2 | 3) {
                sources.push(None);
                continue;
            }
            let (az, el) = LANE_DIRECTIONS_DEG[channel];
            let source_m = spherical_position(az, el, CURRENT_UNIT_SCALE_M);
            let (bank, directions) = SourceReflectionBank::new(sample_rate_hz, source_m);
            for wall in 0..NUM_REFLECTIONS {
                let w = bank.taps[wall].gain.max(0.0);
                for axis in 0..3 {
                    direction_sum[wall][axis] += directions[wall][axis] * w;
                }
                direction_weight[wall] += w;
            }
            sources.push(Some(bank));
        }

        let measured = MeasuredHrirData::saf_kemar().resampled_to(sample_rate_hz);
        let hrir = HrirSet::new(&measured, sample_rate_hz);
        let fallback: [[f32; 3]; NUM_REFLECTIONS] = [
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ];
        let directions: [[f32; 3]; NUM_REFLECTIONS] = std::array::from_fn(|wall| {
            if direction_weight[wall] > 1e-9 {
                normalized(direction_sum[wall])
            } else {
                fallback[wall]
            }
        });
        let buses: [HrtfWallBus; NUM_REFLECTIONS] =
            std::array::from_fn(|wall| HrtfWallBus::new(sample_rate_hz, &hrir, directions[wall]));
        Self { sources, buses }
    }

    pub(crate) fn process(&mut self, field_input: &[f32]) -> anyhow::Result<Vec<f32>> {
        if field_input.len() % MUSIC_FIELD_CHANNELS != 0 {
            bail!(
                "HRTF early-reflection field expected {}-channel interleaved support, got {} samples",
                MUSIC_FIELD_CHANNELS,
                field_input.len()
            );
        }
        let frames = field_input.len() / MUSIC_FIELD_CHANNELS;
        let mut out = vec![0.0f32; frames * 2];
        for frame in 0..frames {
            let mut wall_bus = [0.0f32; NUM_REFLECTIONS];
            let base = frame * MUSIC_FIELD_CHANNELS;
            for channel in 0..MUSIC_FIELD_CHANNELS {
                let Some(source) = self.sources[channel].as_mut() else {
                    continue;
                };
                let paths = source.process(field_input[base + channel]);
                for wall in 0..NUM_REFLECTIONS {
                    wall_bus[wall] += paths[wall];
                }
            }
            let o = frame * 2;
            for (wall, bus) in self.buses.iter_mut().enumerate() {
                let (l, r) = bus.process(wall_bus[wall]);
                out[o] += l;
                out[o + 1] += r;
            }
        }
        Ok(out)
    }
}

fn spherical_position(az_deg: f32, el_deg: f32, distance_m: f32) -> [f32; 3] {
    let az = az_deg.to_radians();
    let el = el_deg.to_radians();
    let ce = el.cos();
    [
        distance_m * ce * az.sin(),
        distance_m * ce * az.cos(),
        distance_m * el.sin(),
    ]
}

#[inline]
fn norm(v: [f32; 3]) -> f32 {
    (v[0] * v[0] + v[1] * v[1] + v[2] * v[2]).sqrt()
}

#[inline]
fn normalized(v: [f32; 3]) -> [f32; 3] {
    let n = norm(v).max(1e-9);
    [v[0] / n, v[1] / n, v[2] / n]
}

#[inline]
fn read_frac(ring: &[f32], cap: usize, write_pos: usize, delay: f32) -> f32 {
    let lo = delay.floor();
    let frac = delay - lo;
    let lo = lo as usize;
    let idx0 = (write_pos + cap - lo % cap) % cap;
    let idx1 = (idx0 + cap - 1) % cap;
    ring[idx0] * (1.0 - frac) + ring[idx1] * frac
}

#[cfg(test)]
mod tests {
    use super::*;

    fn impulse_field(frames: usize, channel: usize) -> Vec<f32> {
        let mut field = vec![0.0f32; frames * MUSIC_FIELD_CHANNELS];
        field[channel] = 1.0;
        field
    }

    #[test]
    fn lane_direction_table_matches_canonical_width() {
        assert_eq!(LANE_DIRECTIONS_DEG.len(), 17);
        assert_eq!(LANE_DIRECTIONS_DEG[8], (180.0, 0.0));
        assert!(LANE_DIRECTIONS_DEG[13].1 < 0.0);
        assert!(LANE_DIRECTIONS_DEG[16].1 < 0.0);
    }

    #[test]
    fn transient_exciter_is_bounded_and_returns_to_unity() {
        let mut exciter = TransientReflectionExciter::new(48_000);
        for _ in 0..1_024 {
            assert!((exciter.gain(0.0) - 1.0).abs() < 1.0e-7);
        }

        let peak = exciter.gain(0.5);
        let maximum = 10.0_f32.powf(TRANSIENT_MAX_GAIN_DB / 20.0);
        assert!(
            peak > 1.25,
            "impulse did not excite early room enough: {peak}"
        );
        assert!(
            peak <= maximum + 1.0e-6,
            "transient gain exceeded bound: {peak}"
        );

        let mut settled = peak;
        for _ in 0..4_800 {
            settled = exciter.gain(0.0);
        }
        assert!(
            settled < 1.01,
            "transient room gain did not decay: {settled}"
        );
    }

    #[test]
    fn steady_tone_does_not_sustain_transient_excitation() {
        let mut exciter = TransientReflectionExciter::new(48_000);
        let mut max_tail = 1.0f32;
        for sample in 0..48_000 {
            let x = (std::f32::consts::TAU * 1_000.0 * sample as f32 / 48_000.0).sin() * 0.2;
            let gain = exciter.gain(x);
            if sample > 9_600 {
                max_tail = max_tail.max(gain);
            }
        }
        assert!(
            max_tail < 1.005,
            "steady tone kept transient room excitation alive: {max_tail}"
        );
    }

    #[test]
    fn sub_threshold_impulse_does_not_excitate_room() {
        let mut exciter = TransientReflectionExciter::new(48_000);
        let gain = exciter.gain(0.0001);
        assert!((gain - 1.0).abs() < 1.0e-7);
    }

    #[test]
    fn center_and_lfe_never_enter_reflection_support() {
        for channel in [2usize, 3usize] {
            let mut field = HrtfEarlyReflectionField::new(48_000);
            let out = field.process(&impulse_field(4_096, channel)).unwrap();
            assert!(out.iter().all(|x| x.abs() < 1e-10));
        }
    }

    #[test]
    fn first_order_field_is_delayed_not_a_second_direct_copy() {
        let mut field = HrtfEarlyReflectionField::new(48_000);
        let out = field.process(&impulse_field(6_000, 0)).unwrap();
        let early_energy: f32 = out[..960].iter().map(|x| x * x).sum();
        let tail_energy: f32 = out[960..].iter().map(|x| x * x).sum();
        assert!(
            early_energy < 1e-10,
            "early reflection arrived too soon: {early_energy}"
        );
        assert!(
            tail_energy > 1e-8,
            "HRTF reflection field produced no delayed energy"
        );
    }

    #[test]
    fn processing_is_block_boundary_invariant() {
        let input = impulse_field(8_000, 4);
        let mut whole = HrtfEarlyReflectionField::new(48_000);
        let expected = whole.process(&input).unwrap();

        let split_at_frames = 2_137usize;
        let split_at = split_at_frames * MUSIC_FIELD_CHANNELS;
        let mut split = HrtfEarlyReflectionField::new(48_000);
        let mut actual = split.process(&input[..split_at]).unwrap();
        actual.extend(split.process(&input[split_at..]).unwrap());

        assert_eq!(expected.len(), actual.len());
        let max_error = expected
            .iter()
            .zip(&actual)
            .map(|(a, b)| (a - b).abs())
            .fold(0.0f32, f32::max);
        assert!(
            max_error < 1e-6,
            "callback boundary changed reflection field: {max_error}"
        );
    }

    #[test]
    fn measured_hrtf_wall_bus_has_lateral_asymmetry() {
        let measured = MeasuredHrirData::saf_kemar().resampled_to(48_000);
        let hrir = HrirSet::new(&measured, 48_000);
        let mut right = HrtfWallBus::new(48_000, &hrir, [1.0, 0.0, 0.0]);
        let mut left = HrtfWallBus::new(48_000, &hrir, [-1.0, 0.0, 0.0]);
        let mut right_energy = [0.0f32; 2];
        let mut left_energy = [0.0f32; 2];
        for i in 0..512 {
            let x = if i == 0 { 1.0 } else { 0.0 };
            let (rl, rr) = right.process(x);
            let (ll, lr) = left.process(x);
            right_energy[0] += rl * rl;
            right_energy[1] += rr * rr;
            left_energy[0] += ll * ll;
            left_energy[1] += lr * lr;
        }
        assert!(
            right_energy[1] > right_energy[0],
            "right wall lacks right-ear dominance: {right_energy:?}"
        );
        assert!(
            left_energy[0] > left_energy[1],
            "left wall lacks left-ear dominance: {left_energy:?}"
        );
    }
}
