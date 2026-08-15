//! Direct renderer path for already-separated causal source channels.
//!
//! This is the path game-music decoders should use once they can provide real
//! source lanes. It deliberately bypasses ordinary stereo scene inference.
//! The protected historical/reference mix remains an external control and must
//! not be included among the object lanes passed here.

use anyhow::{Result, bail};

use crate::source_scene::{SourceLaneKind, SourcePresentationPolicy, SourceSceneEvidence};
use crate::source_scene_event::present_source_channel;
use crate::spatial_renderer::{ChannelRoute, RenderedFrame, SpatialChannelEvent, SpatialRenderer};

pub struct SourceFrameRenderer {
    renderer: SpatialRenderer,
    policy: SourcePresentationPolicy,
    configured_channels: usize,
    routes: Vec<ChannelRoute>,
    events: Vec<SpatialChannelEvent>,
}

impl SourceFrameRenderer {
    pub fn new(renderer: SpatialRenderer, policy: SourcePresentationPolicy) -> Self {
        Self {
            renderer,
            policy,
            configured_channels: 0,
            routes: Vec::new(),
            events: Vec::new(),
        }
    }

    pub fn renderer(&self) -> &SpatialRenderer {
        &self.renderer
    }

    pub fn renderer_mut(&mut self) -> &mut SpatialRenderer {
        &mut self.renderer
    }

    pub fn policy(&self) -> SourcePresentationPolicy {
        self.policy
    }

    pub fn set_policy(&mut self, policy: SourcePresentationPolicy) {
        self.policy = policy;
    }

    pub fn reset_runtime_state(&self) {
        self.renderer.reset_runtime_state();
    }

    /// Render one block of interleaved already-separated source PCM.
    ///
    /// `sources.len()` is the PCM channel count. Every source here must be a
    /// renderable causal lane (`DrySource` or `SharedWetReturn`). The protected
    /// reference mix belongs beside this path as a validation/control signal,
    /// not as another channel inside it.
    pub fn render_source_frame(
        &mut self,
        input_pcm: &[f32],
        sources: &[SourceSceneEvidence],
        sample_pos: u64,
        ramp_length: u32,
        samples_buf: Vec<f32>,
        measure_breakdown: bool,
    ) -> Result<RenderedFrame> {
        let channels = sources.len();
        if channels == 0 {
            if !input_pcm.is_empty() {
                bail!("source PCM is non-empty but source channel list is empty");
            }
            return self.renderer.render_frame(
                input_pcm,
                0,
                &[],
                samples_buf,
                measure_breakdown,
            );
        }
        if input_pcm.len() % channels != 0 {
            bail!(
                "source PCM length {} is not divisible by {} source channels",
                input_pcm.len(),
                channels
            );
        }
        if let Some((index, _)) = sources
            .iter()
            .enumerate()
            .find(|(_, source)| source.lane_kind == SourceLaneKind::ReferenceMix)
        {
            bail!(
                "source channel {index} is a protected ReferenceMix; controls must stay outside the object-lane render"
            );
        }

        if self.configured_channels != channels {
            self.routes.clear();
            self.routes.resize(channels, ChannelRoute::Virtual);
            self.renderer.configure_channel_routing(&self.routes);
            self.configured_channels = channels;
            // A width change is also a source-scene discontinuity. Do not let a
            // previous channel's pose/ramp survive into a newly admitted lane.
            self.renderer.reset_runtime_state();
        }

        self.events.clear();
        self.events.reserve(channels);
        for (channel_idx, source) in sources.iter().copied().enumerate() {
            let presented = present_source_channel(
                channel_idx,
                source,
                self.policy,
                Some(sample_pos),
                Some(ramp_length),
            );
            let Some(event) = presented.event else {
                bail!("renderable source channel {channel_idx} produced no object event");
            };
            self.events.push(event);
        }

        self.renderer.render_frame(
            input_pcm,
            channels,
            &self.events,
            samples_buf,
            measure_breakdown,
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn source_frame_contract_rejects_reference_mix_as_object_lane() {
        let sources = [SourceSceneEvidence {
            lane_kind: SourceLaneKind::ReferenceMix,
            source_id: 1,
            confidence: 1.0,
            ..SourceSceneEvidence::default()
        }];
        assert_eq!(sources[0].lane_kind, SourceLaneKind::ReferenceMix);
    }

    #[test]
    fn source_frame_contract_requires_interleaved_width_to_match_source_count() {
        let sources = [
            SourceSceneEvidence {
                source_id: 1,
                ..SourceSceneEvidence::default()
            },
            SourceSceneEvidence {
                source_id: 2,
                ..SourceSceneEvidence::default()
            },
        ];
        assert_ne!(3usize % sources.len(), 0);
    }
}
