//! Independent binaural (headphone) output stage.
//!
//! This is a **parallel render path**, not a [`GainModel`] backend: a backend
//! only emits per-speaker gains and cannot carry the per-ear delay (ITD) or the
//! stateful HRTF convolution a binaural renderer needs. When
//! [`OutputMode::Binaural`] is selected, `SpatialRenderer::render_frame` skips
//! the whole VBAP / crossover / speaker chain and calls [`BinauralRenderer`]
//! instead, producing a 2-channel (L/R) interleaved frame.
//!
//! Pipeline per object/channel (built out across milestones):
//! `pos_adm → rotate(head_pose) → ×unit_scale_m → (az, el, dist_m)`
//!   → ITD (per-ear fractional delay) + ILD (per-ear gain) + HRIR convolution
//!   → sum into `[L, R]`.
//!
//! Space scaling is a single **isotropic** `unit_scale_m` (metres per ADM unit);
//! the anisotropic `room_ratio` is deliberately *not* reused here because it
//! would distort directions and corrupt HRTF localisation.
//!
//! [`GainModel`]: crate::render_backend::GainModel
//! [`OutputMode`]: crate::live_params::OutputMode

pub mod head_pose;

pub use head_pose::HeadPose;

/// Owns the per-stream binaural DSP state and renders object signals to stereo.
///
/// M0: stereo passthrough downmix only — validates the 2-channel plumbing
/// end-to-end. The per-object ITD/ILD/HRTF machinery lands in M1.
pub struct BinauralRenderer {
    #[allow(dead_code)]
    sample_rate: u32,
}

impl BinauralRenderer {
    pub fn new(sample_rate: u32) -> Self {
        Self { sample_rate }
    }

    /// M0 placeholder: energy-normalised downmix of every input channel into
    /// both ears. `out` must be `sample_length * 2` long (interleaved L/R) and
    /// pre-zeroed. Replaced by the full per-object spatialiser in M1.
    pub fn render_passthrough(
        &mut self,
        input_pcm: &[f32],
        input_channel_count: usize,
        sample_length: usize,
        out: &mut [f32],
    ) {
        debug_assert_eq!(out.len(), sample_length * 2);
        if input_channel_count == 0 || sample_length == 0 {
            return;
        }
        // Equal contribution from each channel, normalised by √N so a full-scale
        // mono signal stays near unity rather than summing to N.
        let gain = 1.0 / (input_channel_count as f32).sqrt();
        for s in 0..sample_length {
            let in_base = s * input_channel_count;
            let mut acc = 0.0f32;
            for c in 0..input_channel_count {
                acc += input_pcm[in_base + c];
            }
            acc *= gain;
            let out_base = s * 2;
            out[out_base] = acc;
            out[out_base + 1] = acc;
        }
    }
}
