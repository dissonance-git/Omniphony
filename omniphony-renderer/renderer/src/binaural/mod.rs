//! Independent binaural (headphone) output stage.
//!
//! This is a **parallel render path**, not a [`GainModel`] backend: a backend
//! only emits per-speaker gains and cannot carry the per-ear delay (ITD) or the
//! stateful HRTF convolution a binaural renderer needs. When
//! [`OutputMode::Binaural`] is selected, `SpatialRenderer::render_frame` skips
//! the whole VBAP / crossover / speaker chain and calls [`BinauralRenderer`]
//! instead, producing a 2-channel (L/R) interleaved frame.
//!
//! Pipeline per input channel, per frame:
//! `pos_adm → rotate(head_pose) → (az, el, dist)`
//!   → distance gain → per-ear ITD delay → per-ear HRIR convolution
//!   → sum into `[L, R]`.
//!
//! Space scaling is a single **isotropic** `unit_scale_m` (metres per ADM unit);
//! the anisotropic `room_ratio` is deliberately *not* reused here because it
//! would distort directions and corrupt HRTF localisation.
//!
//! [`GainModel`]: crate::render_backend::GainModel
//! [`OutputMode`]: crate::live_params::OutputMode

pub mod convolver;
pub mod head_pose;
pub mod hrir;
pub mod itd;
pub mod tracking;

pub use head_pose::HeadPose;
pub use tracking::{HeadTracking, HeadTrackingFormat};

use crate::delay_line::DelayLine;
use convolver::EarConvolver;
use hrir::{HRIR_LEN, HrirPair, HrirSet};

/// Reference distance (m) at which the distance gain is unity.
const REF_DISTANCE_M: f32 = 1.0;
/// Closest distance (m) used for the 1/d law, bounding near-source boost.
const MIN_DISTANCE_M: f32 = 0.25;
/// Maximum distance gain, so a source at the origin can't blow up.
const MAX_DISTANCE_GAIN: f32 = 4.0;
/// Delay-line capacity for the ITD (s) — comfortably above the ~0.7 ms max.
const ITD_MAX_S: f32 = 0.003;

/// Per-input-channel binaural DSP state, lazily created on first use.
struct ChannelDsp {
    delay_l: DelayLine,
    delay_r: DelayLine,
    conv_l: EarConvolver,
    conv_r: EarConvolver,
}

impl ChannelDsp {
    fn new(sample_rate: u32) -> Self {
        let max_delay = (ITD_MAX_S * sample_rate as f32).ceil() as usize;
        Self {
            delay_l: DelayLine::new(max_delay),
            delay_r: DelayLine::new(max_delay),
            conv_l: EarConvolver::new(),
            conv_r: EarConvolver::new(),
        }
    }
}

/// Owns the per-channel binaural DSP state and the HRIR set; renders all input
/// channels of a frame to interleaved stereo.
pub struct BinauralRenderer {
    sample_rate: u32,
    hrir: HrirSet,
    /// Per-input-channel DSP state, indexed directly by channel (sparse tail is
    /// fine; reset when the channel count shrinks).
    channels: Vec<Option<ChannelDsp>>,
    /// Reusable HRIR scratch so `at()` writes in place (no per-channel alloc).
    hrir_scratch: HrirPair,
}

impl BinauralRenderer {
    pub fn new(sample_rate: u32) -> Self {
        Self {
            sample_rate,
            hrir: HrirSet::synthetic(sample_rate),
            channels: Vec::new(),
            hrir_scratch: HrirPair {
                left: [0.0; HRIR_LEN],
                right: [0.0; HRIR_LEN],
            },
        }
    }

    /// Render one frame to interleaved stereo.
    ///
    /// - `chan_pos[c]`: world (ADM) position of input channel `c`.
    /// - `chan_gain[c]`: linear gain for channel `c` (object mute/gain folded in).
    /// - `out`: must be `sample_length * 2`, pre-zeroed.
    #[allow(clippy::too_many_arguments)]
    pub fn render_frame(
        &mut self,
        input_pcm: &[f32],
        input_channel_count: usize,
        sample_length: usize,
        head_pose: HeadPose,
        unit_scale_m: f32,
        chan_pos: &[[f64; 3]],
        chan_gain: &[f32],
        out: &mut [f32],
    ) {
        debug_assert_eq!(out.len(), sample_length * 2);
        if input_channel_count == 0 || sample_length == 0 {
            return;
        }
        if self.channels.len() < input_channel_count {
            self.channels.resize_with(input_channel_count, || None);
        }

        for c in 0..input_channel_count {
            let gain = chan_gain.get(c).copied().unwrap_or(0.0);
            if gain == 0.0 {
                continue;
            }
            let pos = chan_pos.get(c).copied().unwrap_or([0.0, 1.0, 0.0]);

            // World → head-relative direction, then spherical angles.
            let hp = head_pose.rotate(pos);
            let (hx, hy, hz) = (hp[0] as f32, hp[1] as f32, hp[2] as f32);
            let az_rad = hx.atan2(hy); // 0 = front, + = right
            let horiz = (hx * hx + hy * hy).sqrt();
            let el_rad = hz.atan2(horiz);

            // Isotropic distance scale → 1/d gain (direction is scale-invariant).
            let dist_norm = ((pos[0] * pos[0] + pos[1] * pos[1] + pos[2] * pos[2]).sqrt()) as f32;
            let dist_m = (dist_norm * unit_scale_m).max(0.0);
            let dist_gain =
                (REF_DISTANCE_M / dist_m.max(MIN_DISTANCE_M)).clamp(0.0, MAX_DISTANCE_GAIN);
            let g = gain * dist_gain;

            // Per-frame HRIR + ITD update (continuous: delay/convolver state persists).
            self.hrir.at(
                az_rad.to_degrees(),
                el_rad.to_degrees(),
                &mut self.hrir_scratch,
            );
            let (itd_l, itd_r) = itd::ear_delays_seconds(az_rad, el_rad);

            let dsp = self.channels[c].get_or_insert_with(|| ChannelDsp::new(self.sample_rate));
            dsp.conv_l.set_coeffs(&self.hrir_scratch.left);
            dsp.conv_r.set_coeffs(&self.hrir_scratch.right);
            dsp.delay_l.set_target_ms(itd_l * 1000.0, self.sample_rate);
            dsp.delay_r.set_target_ms(itd_r * 1000.0, self.sample_rate);

            for s in 0..sample_length {
                let x = input_pcm[s * input_channel_count + c] * g;
                let yl = dsp.conv_l.process(dsp.delay_l.process(x));
                let yr = dsp.conv_r.process(dsp.delay_r.process(x));
                let o = s * 2;
                out[o] += yl;
                out[o + 1] += yr;
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn render_single(pos: [f64; 3]) -> (f32, f32) {
        let mut r = BinauralRenderer::new(48_000);
        let n = 512;
        // White-ish input: alternating sign so both convolver and delay are exercised.
        let input: Vec<f32> = (0..n)
            .map(|i| if i % 2 == 0 { 1.0 } else { -1.0 })
            .collect();
        let mut out = vec![0.0f32; n * 2];
        r.render_frame(
            &input,
            1,
            n,
            HeadPose::identity(),
            1.0,
            &[pos],
            &[1.0],
            &mut out,
        );
        let mut el = 0.0f32;
        let mut er = 0.0f32;
        for s in 0..n {
            el += out[s * 2] * out[s * 2];
            er += out[s * 2 + 1] * out[s * 2 + 1];
        }
        (el, er)
    }

    #[test]
    fn right_source_is_louder_in_right_channel() {
        let (el, er) = render_single([1.0, 0.0, 0.0]); // full right
        assert!(er > el * 1.5, "L={el} R={er}");
    }

    #[test]
    fn left_source_is_louder_in_left_channel() {
        let (el, er) = render_single([-1.0, 0.0, 0.0]); // full left
        assert!(el > er * 1.5, "L={el} R={er}");
    }

    #[test]
    fn front_source_is_balanced() {
        let (el, er) = render_single([0.0, 1.0, 0.0]); // front
        let ratio = el / er;
        assert!((0.8..1.25).contains(&ratio), "L={el} R={er}");
    }

    #[test]
    fn muted_channel_is_silent() {
        let mut r = BinauralRenderer::new(48_000);
        let n = 64;
        let input = vec![1.0f32; n];
        let mut out = vec![0.0f32; n * 2];
        r.render_frame(
            &input,
            1,
            n,
            HeadPose::identity(),
            1.0,
            &[[1.0, 0.0, 0.0]],
            &[0.0],
            &mut out,
        );
        assert!(out.iter().all(|&x| x == 0.0));
    }
}
