//! Native Windows speaker-bed ingress for the realtime endpoint host.
//!
//! This path is deliberately evidence-preserving. A WAVEFORMATEXTENSIBLE channel
//! mask is already authored spatial information, so Omniphony maps each real
//! speaker lane to an authored source coordinate and bypasses stereo inference.
//! The source-aware renderer then uses the same 22-direction shell / binaural
//! semantics as the rest of Current.
//!
//! LFE is the exception. It is real source evidence, but it is not a directional
//! loudspeaker object. Keep it out of HRTF placement, low-pass it defensively,
//! and add it coherently to both ears before the shared headphone calibration.
//! Missing canonical anchors, including all four lower 8.1.4.4 anchors, remain
//! empty rather than being synthesized from neighboring channels.

use crate::StereoLookaheadPeakGuard;
use crate::noire_x_profile::NoireXPersonalEq;
use orender_engine::{SourceRendererOptions, SourceSpatialMode, build_source_frame_renderer};
use renderer::source_frame::SourceFrameRenderer;
use renderer::source_scene::{SourceLaneKind, SourceSceneEvidence};
use std::f32::consts::PI;

const BED_LINEAR_OUTPUT_GAIN: f32 = 0.90;
const LFE_CUTOFF_HZ: f32 = 120.0;

pub(crate) const SPEAKER_FRONT_LEFT: u32 = 0x0000_0001;
pub(crate) const SPEAKER_FRONT_RIGHT: u32 = 0x0000_0002;
pub(crate) const SPEAKER_FRONT_CENTER: u32 = 0x0000_0004;
pub(crate) const SPEAKER_LOW_FREQUENCY: u32 = 0x0000_0008;
pub(crate) const SPEAKER_BACK_LEFT: u32 = 0x0000_0010;
pub(crate) const SPEAKER_BACK_RIGHT: u32 = 0x0000_0020;
pub(crate) const SPEAKER_FRONT_LEFT_OF_CENTER: u32 = 0x0000_0040;
pub(crate) const SPEAKER_FRONT_RIGHT_OF_CENTER: u32 = 0x0000_0080;
pub(crate) const SPEAKER_BACK_CENTER: u32 = 0x0000_0100;
pub(crate) const SPEAKER_SIDE_LEFT: u32 = 0x0000_0200;
pub(crate) const SPEAKER_SIDE_RIGHT: u32 = 0x0000_0400;
pub(crate) const SPEAKER_TOP_CENTER: u32 = 0x0000_0800;
pub(crate) const SPEAKER_TOP_FRONT_LEFT: u32 = 0x0000_1000;
pub(crate) const SPEAKER_TOP_FRONT_CENTER: u32 = 0x0000_2000;
pub(crate) const SPEAKER_TOP_FRONT_RIGHT: u32 = 0x0000_4000;
pub(crate) const SPEAKER_TOP_BACK_LEFT: u32 = 0x0000_8000;
pub(crate) const SPEAKER_TOP_BACK_CENTER: u32 = 0x0001_0000;
pub(crate) const SPEAKER_TOP_BACK_RIGHT: u32 = 0x0002_0000;

const SUPPORTED_MASK: u32 = SPEAKER_FRONT_LEFT
    | SPEAKER_FRONT_RIGHT
    | SPEAKER_FRONT_CENTER
    | SPEAKER_LOW_FREQUENCY
    | SPEAKER_BACK_LEFT
    | SPEAKER_BACK_RIGHT
    | SPEAKER_FRONT_LEFT_OF_CENTER
    | SPEAKER_FRONT_RIGHT_OF_CENTER
    | SPEAKER_BACK_CENTER
    | SPEAKER_SIDE_LEFT
    | SPEAKER_SIDE_RIGHT
    | SPEAKER_TOP_CENTER
    | SPEAKER_TOP_FRONT_LEFT
    | SPEAKER_TOP_FRONT_CENTER
    | SPEAKER_TOP_FRONT_RIGHT
    | SPEAKER_TOP_BACK_LEFT
    | SPEAKER_TOP_BACK_CENTER
    | SPEAKER_TOP_BACK_RIGHT;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum SpeakerRole {
    FrontLeft,
    FrontRight,
    FrontCenter,
    Lfe,
    BackLeft,
    BackRight,
    FrontLeftOfCenter,
    FrontRightOfCenter,
    BackCenter,
    SideLeft,
    SideRight,
    TopCenter,
    TopFrontLeft,
    TopFrontCenter,
    TopFrontRight,
    TopBackLeft,
    TopBackCenter,
    TopBackRight,
}

const ORDERED_ROLES: [(u32, SpeakerRole); 18] = [
    (SPEAKER_FRONT_LEFT, SpeakerRole::FrontLeft),
    (SPEAKER_FRONT_RIGHT, SpeakerRole::FrontRight),
    (SPEAKER_FRONT_CENTER, SpeakerRole::FrontCenter),
    (SPEAKER_LOW_FREQUENCY, SpeakerRole::Lfe),
    (SPEAKER_BACK_LEFT, SpeakerRole::BackLeft),
    (SPEAKER_BACK_RIGHT, SpeakerRole::BackRight),
    (SPEAKER_FRONT_LEFT_OF_CENTER, SpeakerRole::FrontLeftOfCenter),
    (SPEAKER_FRONT_RIGHT_OF_CENTER, SpeakerRole::FrontRightOfCenter),
    (SPEAKER_BACK_CENTER, SpeakerRole::BackCenter),
    (SPEAKER_SIDE_LEFT, SpeakerRole::SideLeft),
    (SPEAKER_SIDE_RIGHT, SpeakerRole::SideRight),
    (SPEAKER_TOP_CENTER, SpeakerRole::TopCenter),
    (SPEAKER_TOP_FRONT_LEFT, SpeakerRole::TopFrontLeft),
    (SPEAKER_TOP_FRONT_CENTER, SpeakerRole::TopFrontCenter),
    (SPEAKER_TOP_FRONT_RIGHT, SpeakerRole::TopFrontRight),
    (SPEAKER_TOP_BACK_LEFT, SpeakerRole::TopBackLeft),
    (SPEAKER_TOP_BACK_CENTER, SpeakerRole::TopBackCenter),
    (SPEAKER_TOP_BACK_RIGHT, SpeakerRole::TopBackRight),
];

#[derive(Clone, Debug)]
pub(crate) struct NativeBedLayout {
    channels: usize,
    channel_mask: u32,
    roles: Vec<SpeakerRole>,
    object_input_indices: Vec<usize>,
    sources: Vec<SourceSceneEvidence>,
    lfe_index: Option<usize>,
}

impl NativeBedLayout {
    pub(crate) fn new(channels: usize, channel_mask: u32) -> Result<Self, String> {
        if channels == 0 {
            return Err("native bed has zero channels".to_string());
        }
        if channel_mask == 0 {
            return Err("native bed requires a WAVEFORMATEXTENSIBLE channel mask".to_string());
        }
        if channel_mask & !SUPPORTED_MASK != 0 {
            return Err(format!(
                "native bed contains unsupported speaker bits 0x{:08x}",
                channel_mask & !SUPPORTED_MASK
            ));
        }
        if channel_mask.count_ones() as usize != channels {
            return Err(format!(
                "native bed channel count {channels} does not match mask 0x{channel_mask:08x}"
            ));
        }

        let mut roles = Vec::with_capacity(channels);
        let mut object_input_indices = Vec::with_capacity(channels);
        let mut sources = Vec::with_capacity(channels);
        let mut lfe_index = None;

        for &(bit, role) in &ORDERED_ROLES {
            if channel_mask & bit == 0 {
                continue;
            }
            let input_index = roles.len();
            roles.push(role);
            if role == SpeakerRole::Lfe {
                lfe_index = Some(input_index);
                continue;
            }

            let (azimuth_deg, elevation_deg) = speaker_angles(role)
                .ok_or_else(|| format!("speaker role {role:?} has no authored position"))?;
            object_input_indices.push(input_index);
            sources.push(SourceSceneEvidence {
                lane_kind: SourceLaneKind::DrySource,
                source_id: bit as u64,
                authored_position: Some(spherical_position(azimuth_deg, elevation_deg, 1.0)),
                foundation: if matches!(
                    role,
                    SpeakerRole::FrontLeft | SpeakerRole::FrontRight | SpeakerRole::FrontCenter
                ) {
                    1.0
                } else {
                    0.0
                },
                foreground: if role == SpeakerRole::FrontCenter { 1.0 } else { 0.0 },
                confidence: 1.0,
                ..SourceSceneEvidence::default()
            });
        }

        if roles.len() != channels {
            return Err("native bed mask expansion width mismatch".to_string());
        }

        Ok(Self {
            channels,
            channel_mask,
            roles,
            object_input_indices,
            sources,
            lfe_index,
        })
    }

    pub(crate) fn channels(&self) -> usize {
        self.channels
    }

    pub(crate) fn channel_mask(&self) -> u32 {
        self.channel_mask
    }

    fn object_count(&self) -> usize {
        self.sources.len()
    }

    /// Deterministic safety fold-down used only if the worker misses its bounded
    /// deadline. Normal native-bed audio never takes this path.
    pub(crate) fn safety_downmix_frame(&self, frame: &[f32]) -> [f32; 2] {
        if frame.len() != self.channels {
            return [0.0, 0.0];
        }
        let mut left = 0.0f32;
        let mut right = 0.0f32;
        for (index, role) in self.roles.iter().copied().enumerate() {
            let sample = if frame[index].is_finite() { frame[index] } else { 0.0 };
            match role {
                SpeakerRole::FrontLeft | SpeakerRole::FrontLeftOfCenter => left += sample,
                SpeakerRole::FrontRight | SpeakerRole::FrontRightOfCenter => right += sample,
                SpeakerRole::FrontCenter => {
                    left += sample * 0.707_106_77;
                    right += sample * 0.707_106_77;
                }
                SpeakerRole::Lfe => {
                    left += sample * 0.5;
                    right += sample * 0.5;
                }
                SpeakerRole::SideLeft | SpeakerRole::BackLeft => left += sample * 0.707_106_77,
                SpeakerRole::SideRight | SpeakerRole::BackRight => right += sample * 0.707_106_77,
                SpeakerRole::BackCenter => {
                    left += sample * 0.5;
                    right += sample * 0.5;
                }
                SpeakerRole::TopFrontLeft | SpeakerRole::TopBackLeft => left += sample * 0.5,
                SpeakerRole::TopFrontRight | SpeakerRole::TopBackRight => right += sample * 0.5,
                SpeakerRole::TopCenter | SpeakerRole::TopFrontCenter | SpeakerRole::TopBackCenter => {
                    left += sample * 0.353_553_38;
                    right += sample * 0.353_553_38;
                }
            }
        }
        let peak = left.abs().max(right.abs());
        if peak > 1.0 {
            [left / peak, right / peak]
        } else {
            [left, right]
        }
    }
}

fn speaker_angles(role: SpeakerRole) -> Option<(f32, f32)> {
    match role {
        SpeakerRole::FrontLeft => Some((-30.0, 0.0)),
        SpeakerRole::FrontRight => Some((30.0, 0.0)),
        SpeakerRole::FrontCenter => Some((0.0, 0.0)),
        SpeakerRole::Lfe => None,
        SpeakerRole::BackLeft => Some((-135.0, 0.0)),
        SpeakerRole::BackRight => Some((135.0, 0.0)),
        SpeakerRole::FrontLeftOfCenter => Some((-15.0, 0.0)),
        SpeakerRole::FrontRightOfCenter => Some((15.0, 0.0)),
        SpeakerRole::BackCenter => Some((180.0, 0.0)),
        SpeakerRole::SideLeft => Some((-90.0, 0.0)),
        SpeakerRole::SideRight => Some((90.0, 0.0)),
        SpeakerRole::TopCenter => Some((0.0, 90.0)),
        SpeakerRole::TopFrontLeft => Some((-30.0, 45.0)),
        SpeakerRole::TopFrontCenter => Some((0.0, 45.0)),
        SpeakerRole::TopFrontRight => Some((30.0, 45.0)),
        SpeakerRole::TopBackLeft => Some((-135.0, 45.0)),
        SpeakerRole::TopBackCenter => Some((180.0, 45.0)),
        SpeakerRole::TopBackRight => Some((135.0, 45.0)),
    }
}

fn spherical_position(azimuth_deg: f32, elevation_deg: f32, distance: f32) -> [f64; 3] {
    let azimuth = azimuth_deg.to_radians();
    let elevation = elevation_deg.to_radians();
    let horizontal = elevation.cos() * distance;
    [
        (azimuth.sin() * horizontal) as f64,
        (azimuth.cos() * horizontal) as f64,
        (elevation.sin() * distance) as f64,
    ]
}

#[derive(Clone, Copy, Debug)]
struct OnePoleLowPass {
    alpha: f32,
    state: f32,
}

impl OnePoleLowPass {
    fn new(sample_rate_hz: u32, cutoff_hz: f32) -> Self {
        let rate = sample_rate_hz.max(1) as f32;
        let alpha = 1.0 - (-2.0 * PI * cutoff_hz.max(1.0) / rate).exp();
        Self { alpha, state: 0.0 }
    }

    fn process(&mut self, sample: f32) -> f32 {
        let input = if sample.is_finite() { sample } else { 0.0 };
        self.state += self.alpha * (input - self.state);
        self.state
    }
}

struct LfeLowPass {
    first: OnePoleLowPass,
    second: OnePoleLowPass,
}

impl LfeLowPass {
    fn new(sample_rate_hz: u32) -> Self {
        Self {
            first: OnePoleLowPass::new(sample_rate_hz, LFE_CUTOFF_HZ),
            second: OnePoleLowPass::new(sample_rate_hz, LFE_CUTOFF_HZ),
        }
    }

    fn process(&mut self, sample: f32) -> f32 {
        self.second.process(self.first.process(sample))
    }
}

pub(crate) struct NativeBedPipeline {
    layout: NativeBedLayout,
    renderer: SourceFrameRenderer,
    object_pcm: Vec<f32>,
    render_buf: Vec<f32>,
    lfe: LfeLowPass,
    headphone_eq: NoireXPersonalEq,
    peak_guard: StereoLookaheadPeakGuard,
    sample_pos: u64,
}

impl NativeBedPipeline {
    pub(crate) fn new(
        sample_rate_hz: u32,
        channels: usize,
        channel_mask: u32,
    ) -> Result<Self, String> {
        let layout = NativeBedLayout::new(channels, channel_mask)?;
        let renderer = build_source_frame_renderer(
            sample_rate_hz,
            None,
            SourceRendererOptions {
                mode: SourceSpatialMode::FullSphere,
                externalization: false,
                ..SourceRendererOptions::default()
            },
        )
        .map_err(|error| error.to_string())?;

        Ok(Self {
            layout,
            renderer,
            object_pcm: Vec::new(),
            render_buf: Vec::new(),
            lfe: LfeLowPass::new(sample_rate_hz),
            headphone_eq: NoireXPersonalEq::new(sample_rate_hz),
            peak_guard: StereoLookaheadPeakGuard::new(sample_rate_hz),
            sample_pos: 0,
        })
    }

    pub(crate) fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, String> {
        if input.is_empty() || input.len() % self.layout.channels != 0 {
            return Err("native bed input width mismatch".to_string());
        }
        let frames = input.len() / self.layout.channels;

        self.object_pcm.clear();
        self.object_pcm
            .reserve(frames.saturating_mul(self.layout.object_count()));
        if self.layout.object_count() != 0 {
            for frame in input.chunks_exact(self.layout.channels) {
                for &input_index in &self.layout.object_input_indices {
                    let sample = frame[input_index];
                    self.object_pcm.push(if sample.is_finite() { sample } else { 0.0 });
                }
            }
        }

        let mut mixed = if self.layout.object_count() == 0 {
            vec![0.0f32; frames * 2]
        } else {
            let rendered = self
                .renderer
                .render_source_frame_with_gain_policy(
                    &self.object_pcm,
                    &self.layout.sources,
                    None,
                    self.sample_pos,
                    0,
                    std::mem::take(&mut self.render_buf),
                    false,
                )
                .map_err(|error| error.to_string())?;
            rendered.samples
        };

        if mixed.len() != frames * 2 {
            return Err(format!(
                "native bed renderer returned {} samples for {frames} frames",
                mixed.len()
            ));
        }

        if let Some(lfe_index) = self.layout.lfe_index {
            for (frame_index, frame) in input.chunks_exact(self.layout.channels).enumerate() {
                let lfe = self.lfe.process(frame[lfe_index]);
                mixed[frame_index * 2] += lfe;
                mixed[frame_index * 2 + 1] += lfe;
            }
        }

        for sample in &mut mixed {
            *sample = if sample.is_finite() {
                *sample * BED_LINEAR_OUTPUT_GAIN
            } else {
                0.0
            };
        }

        self.headphone_eq.process_interleaved(&mut mixed);
        self.sample_pos = self.sample_pos.saturating_add(frames as u64);
        Ok(self.peak_guard.process_interleaved(&mixed))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    const MASK_7_1_4: u32 = SPEAKER_FRONT_LEFT
        | SPEAKER_FRONT_RIGHT
        | SPEAKER_FRONT_CENTER
        | SPEAKER_LOW_FREQUENCY
        | SPEAKER_BACK_LEFT
        | SPEAKER_BACK_RIGHT
        | SPEAKER_SIDE_LEFT
        | SPEAKER_SIDE_RIGHT
        | SPEAKER_TOP_FRONT_LEFT
        | SPEAKER_TOP_FRONT_RIGHT
        | SPEAKER_TOP_BACK_LEFT
        | SPEAKER_TOP_BACK_RIGHT;

    #[test]
    fn waveformat_mask_order_is_preserved_and_lfe_is_not_an_object() {
        let layout = NativeBedLayout::new(12, MASK_7_1_4).expect("7.1.4 layout");
        assert_eq!(layout.channels(), 12);
        assert_eq!(layout.channel_mask(), MASK_7_1_4);
        assert_eq!(layout.roles[0], SpeakerRole::FrontLeft);
        assert_eq!(layout.roles[1], SpeakerRole::FrontRight);
        assert_eq!(layout.roles[2], SpeakerRole::FrontCenter);
        assert_eq!(layout.roles[3], SpeakerRole::Lfe);
        assert_eq!(layout.lfe_index, Some(3));
        assert_eq!(layout.object_count(), 11);
    }

    #[test]
    fn top_channels_keep_real_positive_elevation() {
        let (_, elevation) = speaker_angles(SpeakerRole::TopFrontLeft).unwrap();
        assert!(elevation > 0.0);
        let position = spherical_position(-30.0, elevation, 1.0);
        assert!(position[0] < 0.0);
        assert!(position[1] > 0.0);
        assert!(position[2] > 0.0);
    }

    #[test]
    fn unsupported_or_ambiguous_masks_are_rejected_instead_of_guessed() {
        assert!(NativeBedLayout::new(6, 0).is_err());
        assert!(NativeBedLayout::new(6, SPEAKER_FRONT_LEFT | SPEAKER_FRONT_RIGHT).is_err());
        assert!(NativeBedLayout::new(1, 0x8000_0000).is_err());
    }

    #[test]
    fn safety_downmix_is_only_a_bounded_stereo_fallback() {
        let mask = SPEAKER_FRONT_LEFT | SPEAKER_FRONT_RIGHT | SPEAKER_FRONT_CENTER;
        let layout = NativeBedLayout::new(3, mask).unwrap();
        let out = layout.safety_downmix_frame(&[0.5, 0.25, 0.2]);
        assert!(out[0] > out[1]);
        assert!(out.iter().all(|sample| sample.abs() <= 1.0));
    }
}
