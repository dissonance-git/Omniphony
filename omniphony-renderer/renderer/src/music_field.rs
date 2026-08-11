//! Frequency-aware stereo music support-field extraction.
//!
//! The finished stereo master remains authoritative. This module analyzes real
//! L/R magnitude and phase relationships in the frequency domain, reuses the
//! portable stereo/scene evidence laws, and turns that evidence into a small set
//! of causal support lanes for the binaural renderer.
//!
//! The FFT is analysis-only. Audible support is extracted with a causal
//! multiband filter bank so this stage does not introduce an STFT synthesis
//! latency that would have to be reconciled with the protected direct master.

use crate::scene_inference::{
    SceneEvidenceInput, SceneEvidenceKind, infer_scene_evidence,
};
use crate::stereo_inference::{
    StereoBinEvidence, StereoEvidenceTracker, StereoInferenceParams, estimate_bin,
};
use rustfft::{Fft, FftPlanner, num_complex::Complex};
use std::f32::consts::PI;
use std::sync::Arc;

pub const MUSIC_FIELD_CHANNELS: usize = 8;
const FFT_SIZE: usize = 1024;
const TRACK_TIME_CONSTANT_MS: f32 = 140.0;
const CROSSOVER_HZ: [f32; 3] = [220.0, 1_200.0, 5_000.0];

#[derive(Debug, Clone, Copy, Default)]
pub struct MusicFieldSnapshot {
    pub anchor: f32,
    pub broad: f32,
    pub lateral: f32,
    pub diffuse: f32,
    pub lateral_pan: f32,
    pub side_fraction: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct BandAccum {
    weight: f32,
    anchor: f32,
    broad: f32,
    lateral: f32,
    diffuse: f32,
    pan_num: f32,
    pan_weight: f32,
    side_fraction: f32,
}

#[derive(Debug, Clone, Copy, Default)]
struct BandControl {
    anchor: f32,
    broad: f32,
    lateral: f32,
    diffuse: f32,
    pan: f32,
    side_fraction: f32,
}

impl BandControl {
    fn approach(&mut self, target: Self) {
        self.anchor = slew(self.anchor, target.anchor);
        self.broad = slew(self.broad, target.broad);
        self.lateral = slew(self.lateral, target.lateral);
        self.diffuse = slew(self.diffuse, target.diffuse);
        self.pan = slew_signed(self.pan, target.pan);
        self.side_fraction = slew(self.side_fraction, target.side_fraction);
    }
}

fn slew(current: f32, target: f32) -> f32 {
    let coefficient = if target > current { 0.32 } else { 0.12 };
    (current + coefficient * (target - current)).clamp(0.0, 1.0)
}

fn slew_signed(current: f32, target: f32) -> f32 {
    let coefficient = if target.abs() > current.abs() { 0.30 } else { 0.12 };
    (current + coefficient * (target - current)).clamp(-1.0, 1.0)
}

#[derive(Debug, Clone, Copy)]
struct OnePoleLowPass {
    alpha: f32,
    state: f32,
}

impl OnePoleLowPass {
    fn new(sample_rate_hz: u32, cutoff_hz: f32) -> Self {
        let dt = 1.0 / sample_rate_hz.max(1) as f32;
        let rc = 1.0 / (2.0 * PI * cutoff_hz.max(1.0));
        Self {
            alpha: dt / (rc + dt),
            state: 0.0,
        }
    }

    fn process(&mut self, sample: f32) -> f32 {
        self.state += self.alpha * (sample - self.state);
        self.state
    }
}

struct ChannelBandSplit {
    low_220: OnePoleLowPass,
    low_1200: OnePoleLowPass,
    low_5000: OnePoleLowPass,
}

impl ChannelBandSplit {
    fn new(sample_rate_hz: u32) -> Self {
        Self {
            low_220: OnePoleLowPass::new(sample_rate_hz, CROSSOVER_HZ[0]),
            low_1200: OnePoleLowPass::new(sample_rate_hz, CROSSOVER_HZ[1]),
            low_5000: OnePoleLowPass::new(sample_rate_hz, CROSSOVER_HZ[2]),
        }
    }

    /// The four outputs sum exactly to `sample` at each sample despite the
    /// individual one-pole phase responses because adjacent bands are formed by
    /// subtraction from the same parallel low-pass states.
    fn split(&mut self, sample: f32) -> [f32; 4] {
        let a = self.low_220.process(sample);
        let b = self.low_1200.process(sample);
        let c = self.low_5000.process(sample);
        [a, b - a, c - b, sample - c]
    }
}

/// Portable music-field extractor.
///
/// Output order is the canonical 7.1 bed order expected by the bridge:
/// `L R C LFE Ls Rs Lb Rb`. C and LFE are intentionally zero. The support
/// lanes are:
///
/// - L/R: broad source extent, kept closer to the front/side hemisphere;
/// - Ls/Rs: persistent lateral/object-like evidence;
/// - Lb/Rb: diffuse/field-like evidence.
///
/// The caller remains responsible for adding the binaurally rendered support
/// around the untouched, latency-aligned stereo master.
pub struct MusicFieldProcessor {
    sample_rate_hz: u32,
    fft: Arc<dyn Fft<f32>>,
    left_fft: Vec<Complex<f32>>,
    right_fft: Vec<Complex<f32>>,
    trackers: Vec<StereoEvidenceTracker>,
    controls: [BandControl; 3],
    left_split: ChannelBandSplit,
    right_split: ChannelBandSplit,
    snapshot: MusicFieldSnapshot,
}

impl MusicFieldProcessor {
    pub fn new(sample_rate_hz: u32) -> Self {
        let mut planner = FftPlanner::<f32>::new();
        let fft = planner.plan_fft_forward(FFT_SIZE);
        Self {
            sample_rate_hz,
            fft,
            left_fft: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            right_fft: vec![Complex::new(0.0, 0.0); FFT_SIZE],
            trackers: vec![StereoEvidenceTracker::default(); FFT_SIZE / 2 + 1],
            controls: [BandControl::default(); 3],
            left_split: ChannelBandSplit::new(sample_rate_hz),
            right_split: ChannelBandSplit::new(sample_rate_hz),
            snapshot: MusicFieldSnapshot::default(),
        }
    }

    pub fn snapshot(&self) -> MusicFieldSnapshot {
        self.snapshot
    }

    pub fn process_interleaved_stereo(&mut self, input: &[f32]) -> Vec<f32> {
        if input.len() < 2 || input.len() % 2 != 0 {
            return Vec::new();
        }

        self.analyze(input);

        let frames = input.len() / 2;
        let mut out = Vec::with_capacity(frames * MUSIC_FIELD_CHANNELS);
        for frame in input.chunks_exact(2) {
            let left_bands = self.left_split.split(frame[0]);
            let right_bands = self.right_split.split(frame[1]);

            let mut broad_l = 0.0;
            let mut broad_r = 0.0;
            let mut lateral_l = 0.0;
            let mut lateral_r = 0.0;
            let mut diffuse_l = 0.0;
            let mut diffuse_r = 0.0;

            // Band 0 (<220 Hz) is the protected foundation and intentionally
            // never enters the support field. Controls 0..2 correspond to the
            // three bands above that floor.
            for band in 1..4 {
                let control = self.controls[band - 1];
                let left = left_bands[band];
                let right = right_bands[band];
                let mid = 0.5 * (left + right);
                let side = 0.5 * (left - right);

                // Preserve center authority. A small amount of coherent mid may
                // remain in broad support, but stable anchors are strongly
                // suppressed by the evidence controls before this point.
                let relational_l = left - 0.85 * mid;
                let relational_r = right - 0.85 * mid;

                broad_l += relational_l * control.broad * 0.60;
                broad_r += relational_r * control.broad * 0.60;

                let steer_l = (0.5 - 0.5 * control.pan).clamp(0.0, 1.0);
                let steer_r = (0.5 + 0.5 * control.pan).clamp(0.0, 1.0);
                lateral_l += (0.75 * relational_l + 0.25 * side)
                    * control.lateral
                    * (0.65 + 0.35 * steer_l)
                    * 0.85;
                lateral_r += (0.75 * relational_r - 0.25 * side)
                    * control.lateral
                    * (0.65 + 0.35 * steer_r)
                    * 0.85;

                // Diffuse evidence keeps the source's actual differential
                // relationship. No synthetic room or random decorrelator is
                // introduced here; Omniphony owns the binaural geometry.
                diffuse_l += side * control.diffuse * 0.75;
                diffuse_r -= side * control.diffuse * 0.75;
            }

            out.extend_from_slice(&[
                broad_l,
                broad_r,
                0.0, // C: protected direct master owns center authority.
                0.0, // LFE: low-frequency foundation stays in the master.
                lateral_l,
                lateral_r,
                diffuse_l,
                diffuse_r,
            ]);
        }
        out
    }

    fn analyze(&mut self, input: &[f32]) {
        self.left_fft.fill(Complex::new(0.0, 0.0));
        self.right_fft.fill(Complex::new(0.0, 0.0));

        let frames = input.len() / 2;
        let usable = frames.min(FFT_SIZE);
        let start_frame = frames.saturating_sub(usable);
        for i in 0..usable {
            let source = (start_frame + i) * 2;
            let window = if usable > 1 {
                0.5 - 0.5 * (2.0 * PI * i as f32 / (usable - 1) as f32).cos()
            } else {
                1.0
            };
            self.left_fft[i] = Complex::new(input[source] * window, 0.0);
            self.right_fft[i] = Complex::new(input[source + 1] * window, 0.0);
        }

        self.fft.process(&mut self.left_fft);
        self.fft.process(&mut self.right_fft);

        let mut reference_magnitude = 1.0e-9_f32;
        for bin in 1..=FFT_SIZE / 2 {
            let l = self.left_fft[bin].norm();
            let r = self.right_fft[bin].norm();
            reference_magnitude = reference_magnitude.max(l.hypot(r));
        }

        let elapsed_ms = frames as f32 * 1000.0 / self.sample_rate_hz.max(1) as f32;
        let mut accum = [BandAccum::default(); 3];
        let params = StereoInferenceParams {
            focus: 0.05,
            object_separation: 0.15,
        };

        for bin in 1..=FFT_SIZE / 2 {
            let frequency_hz = bin as f32 * self.sample_rate_hz as f32 / FFT_SIZE as f32;
            if frequency_hz < CROSSOVER_HZ[0] {
                continue;
            }
            let band = if frequency_hz < CROSSOVER_HZ[1] {
                0
            } else if frequency_hz < CROSSOVER_HZ[2] {
                1
            } else {
                2
            };

            let left = self.left_fft[bin];
            let right = self.right_fft[bin];
            let estimate = estimate_bin(
                StereoBinEvidence {
                    left_magnitude: left.norm(),
                    right_magnitude: right.norm(),
                    left_phase: left.im.atan2(left.re),
                    right_phase: right.im.atan2(right.re),
                },
                params,
            );
            let tracked = self.trackers[bin].update(
                estimate,
                elapsed_ms,
                TRACK_TIME_CONSTANT_MS,
            );
            let candidate = infer_scene_evidence(SceneEvidenceInput {
                frequency_hz,
                estimate,
                tracked,
                magnitude: estimate.total_magnitude,
                reference_magnitude,
            });

            let weight = estimate.total_magnitude.max(1.0e-9);
            let anchor = if matches!(candidate.kind, SceneEvidenceKind::FrontalAnchor) {
                candidate.foundation_support.max(0.72)
            } else {
                candidate.foundation_support
            }
            .clamp(0.0, 1.0);
            let movable = (1.0 - 0.90 * anchor).clamp(0.0, 1.0);

            let lateral = if matches!(candidate.kind, SceneEvidenceKind::LateralObjectCandidate) {
                candidate
                    .reassignment_safety
                    .max(0.55 * candidate.object_support)
            } else {
                0.20 * candidate.reassignment_safety
            } * movable;

            let broad = match candidate.kind {
                SceneEvidenceKind::BroadSource => 0.35 + 0.65 * candidate.side_fraction,
                SceneEvidenceKind::LateralObjectCandidate => 0.18 + 0.30 * candidate.side_fraction,
                SceneEvidenceKind::DiffuseField => 0.10 + 0.20 * candidate.side_fraction,
                SceneEvidenceKind::FrontalAnchor => 0.0,
            } * movable;

            let diffuse = if matches!(candidate.kind, SceneEvidenceKind::DiffuseField) {
                candidate.field_support
            } else {
                0.18 * candidate.field_support
            } * movable;

            let a = &mut accum[band];
            a.weight += weight;
            a.anchor += weight * anchor;
            a.broad += weight * broad;
            a.lateral += weight * lateral;
            a.diffuse += weight * diffuse;
            a.side_fraction += weight * candidate.side_fraction;
            let pan_weight = weight * candidate.object_support.max(0.05);
            a.pan_num += pan_weight * candidate.pan;
            a.pan_weight += pan_weight;
        }

        let mut snapshot_weight = 0.0;
        let mut snapshot = MusicFieldSnapshot::default();
        for (index, a) in accum.into_iter().enumerate() {
            let target = if a.weight > 1.0e-9 {
                BandControl {
                    anchor: (a.anchor / a.weight).clamp(0.0, 1.0),
                    broad: ((a.broad / a.weight) * 1.35).clamp(0.0, 1.0),
                    lateral: ((a.lateral / a.weight) * 1.70).clamp(0.0, 1.0),
                    diffuse: ((a.diffuse / a.weight) * 1.45).clamp(0.0, 1.0),
                    pan: if a.pan_weight > 1.0e-9 {
                        (a.pan_num / a.pan_weight).clamp(-1.0, 1.0)
                    } else {
                        0.0
                    },
                    side_fraction: (a.side_fraction / a.weight).clamp(0.0, 1.0),
                }
            } else {
                BandControl::default()
            };
            self.controls[index].approach(target);

            let w = a.weight.max(1.0e-9);
            snapshot_weight += w;
            snapshot.anchor += w * self.controls[index].anchor;
            snapshot.broad += w * self.controls[index].broad;
            snapshot.lateral += w * self.controls[index].lateral;
            snapshot.diffuse += w * self.controls[index].diffuse;
            snapshot.lateral_pan += w * self.controls[index].pan;
            snapshot.side_fraction += w * self.controls[index].side_fraction;
        }

        if snapshot_weight > 1.0e-9 {
            snapshot.anchor /= snapshot_weight;
            snapshot.broad /= snapshot_weight;
            snapshot.lateral /= snapshot_weight;
            snapshot.diffuse /= snapshot_weight;
            snapshot.lateral_pan /= snapshot_weight;
            snapshot.side_fraction /= snapshot_weight;
        }
        self.snapshot = snapshot;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn mono_center_does_not_become_a_large_support_field() {
        let mut processor = MusicFieldProcessor::new(48_000);
        let mut input = Vec::new();
        for i in 0..1024 {
            let x = (2.0 * PI * 1000.0 * i as f32 / 48_000.0).sin() * 0.5;
            input.extend_from_slice(&[x, x]);
        }
        let out = processor.process_interleaved_stereo(&input);
        assert_eq!(out.len(), 1024 * MUSIC_FIELD_CHANNELS);
        let energy: f32 = out.iter().map(|x| x * x).sum::<f32>() / out.len() as f32;
        assert!(energy < 0.01);
    }

    #[test]
    fn hard_left_material_produces_lateral_support_without_lfe() {
        let mut processor = MusicFieldProcessor::new(48_000);
        let mut input = Vec::new();
        for i in 0..4096 {
            let x = (2.0 * PI * 1800.0 * i as f32 / 48_000.0).sin() * 0.5;
            input.extend_from_slice(&[x, 0.0]);
        }
        let mut lateral_energy = 0.0;
        let mut lfe_energy = 0.0;
        for chunk in input.chunks(2048) {
            let out = processor.process_interleaved_stereo(chunk);
            for frame in out.chunks_exact(MUSIC_FIELD_CHANNELS) {
                lateral_energy += frame[4] * frame[4] + frame[5] * frame[5];
                lfe_energy += frame[3] * frame[3];
            }
        }
        assert!(lateral_energy > 0.0);
        assert_eq!(lfe_energy, 0.0);
    }

    #[test]
    fn bass_foundation_is_not_emitted_into_support_channels() {
        let mut processor = MusicFieldProcessor::new(48_000);
        let mut input = Vec::new();
        for i in 0..4096 {
            let x = (2.0 * PI * 60.0 * i as f32 / 48_000.0).sin() * 0.5;
            input.extend_from_slice(&[x, x * 0.8]);
        }
        let mut support_energy = 0.0;
        for chunk in input.chunks(2048) {
            let out = processor.process_interleaved_stereo(chunk);
            support_energy += out.iter().map(|x| x * x).sum::<f32>();
        }
        assert!(support_energy < 0.5);
    }
}
