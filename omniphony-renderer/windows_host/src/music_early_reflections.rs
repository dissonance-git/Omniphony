//! Bounded HRTF-rendered early-reflection challenger for the music path.
//!
//! The current renderer's first-order room is deliberately cheap: each source
//! gets six image-source taps with analytic ITD/ILD and broad HF wall loss. A
//! literal measured-HRTF convolution for every image of every virtual source
//! would multiply the FIR count by the speaker count. This module keeps the
//! physical first-order timing/tone law but mixes contributions by wall first,
//! then applies exactly six measured SAF/KEMAR HRTFs.
//!
//! It is used only by the `external` listening challenger. The normal current
//! model still owns the default reflection implementation.

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

// The legacy analytic reflection panner has total L+R power 4/3 for a unit
// reflection gain (`SHADOW=0.5`, denominator 1.5). A diffuse-normalized HRIR
// pair is approximately 2.0 total-ear power. Scale by sqrt((4/3)/2) so this
// challenger primarily changes directional spectral information rather than
// simply making the early field louder.
const HRTF_POWER_MATCH: f32 = 0.816_496_6;

/// Canonical 7.1.4 evidence-lane directions, matching the current music-field
/// meaning rather than pretending the lanes are newly inferred objects.
/// Positive azimuth is right, positive elevation is up.
const LANE_DIRECTIONS_DEG: [(f32, f32); MUSIC_FIELD_CHANNELS] = [
    (-30.0, 0.0),  // L
    (30.0, 0.0),   // R
    (0.0, 0.0),    // C (protected / normally silent in support)
    (0.0, 0.0),    // LFE (protected / normally silent in support)
    (-90.0, 0.0),  // Ls
    (90.0, 0.0),   // Rs
    (-140.0, 0.0), // Lb
    (140.0, 0.0),  // Rb
    (-40.0, 60.0), // Tfl
    (40.0, 60.0),  // Tfr
    (-140.0, 60.0),// Tbl
    (140.0, 60.0), // Tbr
];

#[derive(Clone, Copy, Default)]
struct PathTap {
    delay_samples: f32,
    gain: f32,
    hf_gain: f32,
    tone_state: f32,
}

struct SourceReflectionBank {
    ring: Vec<f32>,
    write_pos: usize,
    taps: [PathTap; NUM_REFLECTIONS],
    tone_alpha: f32,
    air_state: f32,
    air_coeff: f32,
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

        // Match the current renderer's source-distance air cue before the
        // reflection-only wall/extra-path loss is applied.
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
                    - (-std::f32::consts::TAU * TONE_SPLIT_HZ / sample_rate_hz as f32)
                        .exp(),
                air_state: 0.0,
                air_coeff,
            },
            directions,
        )
    }

    #[inline]
    fn process(&mut self, mut input: f32) -> [f32; NUM_REFLECTIONS] {
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

/// Fixed-cost measured-HRTF early field for the twelve music evidence lanes.
///
/// Every source contribution is delayed and wall-filtered independently first.
/// Contributions that hit the same physical wall are then summed, producing six
/// wall buses. Each wall bus receives one measured KEMAR HRTF + analytic ITD.
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
            // Center and LFE remain protected by the master and are deliberately
            // absent from the derived music support field. Ignore them here too
            // if an upstream regression ever leaks energy into those lanes.
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
        let fallback = [
            [1.0, 0.0, 0.0],
            [-1.0, 0.0, 0.0],
            [0.0, 1.0, 0.0],
            [0.0, -1.0, 0.0],
            [0.0, 0.0, 1.0],
            [0.0, 0.0, -1.0],
        ];
        let directions = std::array::from_fn(|wall| {
            if direction_weight[wall] > 1e-9 {
                normalized(direction_sum[wall])
            } else {
                fallback[wall]
            }
        });
        let buses = std::array::from_fn(|wall| {
            HrtfWallBus::new(sample_rate_hz, &hrir, directions[wall])
        });
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
        // The current 23x32x21 m room makes the closest first-order detour many
        // milliseconds long. The first 10 ms must therefore contain no added
        // reflection energy at all.
        let early_energy: f32 = out[..960].iter().map(|x| x * x).sum();
        let tail_energy: f32 = out[960..].iter().map(|x| x * x).sum();
        assert!(early_energy < 1e-10, "early reflection arrived too soon: {early_energy}");
        assert!(tail_energy > 1e-8, "HRTF reflection field produced no delayed energy");
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
        assert!(max_error < 1e-6, "callback boundary changed reflection field: {max_error}");
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
        assert!(right_energy[1] > right_energy[0], "right wall lacks right-ear dominance: {right_energy:?}");
        assert!(left_energy[0] > left_energy[1], "left wall lacks left-ear dominance: {left_energy:?}");
    }
}
