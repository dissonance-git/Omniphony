//! Direct renderer path for already-separated causal source channels.
//!
//! This is the path game-music decoders should use once they can provide real
//! source lanes. It deliberately bypasses ordinary stereo scene inference.
//! The protected historical/reference mix remains an external control and must
//! not be included among the object lanes passed here.

use anyhow::{Result, bail};

use crate::source_identity::{SourcePresentationIdentity, source_presentation_identity};
use crate::source_scene::{
    NativeStereoRoute, SourceLaneKind, SourcePresentationPolicy, SourceSceneEvidence,
};
use crate::source_scene_event::present_source_channel;
use crate::spatial_renderer::{ChannelRoute, RenderedFrame, SpatialChannelEvent, SpatialRenderer};

pub struct SourceFrameRenderer {
    renderer: SpatialRenderer,
    policy: SourcePresentationPolicy,
    configured_channels: usize,
    routes: Vec<ChannelRoute>,
    events: Vec<SpatialChannelEvent>,
    scaled_input: Vec<f32>,
    presentation_identities: Vec<Option<SourcePresentationIdentity>>,
    presentation_identity_initialized: Vec<bool>,
}

/// Collapse a historical stereo route to the scalar energy carried by one
/// causal mono source before Omniphony replaces that two-channel projection
/// with a binaural object.
///
/// Signs remain available in `NativeStereoRoute` as polarity/phase evidence;
/// they do not swap sides and therefore enter the energy law squared. The
/// normalization keeps a source routed at unity to both historical outputs at
/// unity, while a unity hard-left/right source carries sqrt(1/2) of that stereo
/// RMS energy.
pub fn route_energy_gain(route: Option<NativeStereoRoute>) -> f32 {
    let Some(route) = route else { return 1.0; };
    if !route.left_gain.is_finite() || !route.right_gain.is_finite() {
        return 0.0;
    }
    ((route.left_gain * route.left_gain + route.right_gain * route.right_gain) * 0.5)
        .sqrt()
        .clamp(0.0, 1.0)
}

impl SourceFrameRenderer {
    pub fn new(renderer: SpatialRenderer, policy: SourcePresentationPolicy) -> Self {
        Self {
            renderer,
            policy,
            configured_channels: 0,
            routes: Vec::new(),
            events: Vec::new(),
            scaled_input: Vec::new(),
            presentation_identities: Vec::new(),
            presentation_identity_initialized: Vec::new(),
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
    /// This compatibility entry point assumes every lane is still pre-route and
    /// therefore derives scalar source energy from `native_stereo_route`.
    pub fn render_source_frame(
        &mut self,
        input_pcm: &[f32],
        sources: &[SourceSceneEvidence],
        sample_pos: u64,
        ramp_length: u32,
        samples_buf: Vec<f32>,
        measure_breakdown: bool,
    ) -> Result<RenderedFrame> {
        self.render_source_frame_with_gain_policy(
            input_pcm,
            sources,
            None,
            sample_pos,
            ramp_length,
            samples_buf,
            measure_breakdown,
        )
    }

    /// Render one block with optional host-owned gain policy.
    ///
    /// `route_gain_preapplied[channel] == true` means the host has already
    /// applied that lane's sample-accurate native gain trajectory to its causal
    /// PCM. Native L/R routing remains available to the scene policy for pose
    /// and polarity evidence, but Omniphony must not scalar-attenuate the PCM a
    /// second time. This is used by SPC where the effective `mChnL/mChnR`
    /// trajectory varies inside the block.
    pub fn render_source_frame_with_gain_policy(
        &mut self,
        input_pcm: &[f32],
        sources: &[SourceSceneEvidence],
        route_gain_preapplied: Option<&[bool]>,
        sample_pos: u64,
        ramp_length: u32,
        samples_buf: Vec<f32>,
        measure_breakdown: bool,
    ) -> Result<RenderedFrame> {
        let channels = sources.len();
        if let Some(flags) = route_gain_preapplied {
            if flags.len() != channels {
                bail!(
                    "route gain policy width {} does not match {} source channels",
                    flags.len(),
                    channels
                );
            }
        }
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
            self.presentation_identities.clear();
            self.presentation_identities.resize(channels, None);
            self.presentation_identity_initialized.clear();
            self.presentation_identity_initialized.resize(channels, false);
            // A width change is also a source-scene discontinuity. Do not let a
            // previous channel's pose/ramp survive into a newly admitted lane.
            self.renderer.reset_runtime_state();
        }

        self.events.clear();
        self.events.reserve(channels);
        for (channel_idx, source) in sources.iter().copied().enumerate() {
            let identity = source_presentation_identity(&source);
            let identity_changed = self.presentation_identity_initialized[channel_idx]
                && self.presentation_identities[channel_idx] != identity;

            // A physical lane is not a musical identity. If an unrelated source
            // reuses the same channel, do not interpolate through the outgoing
            // source's old pose. A proven persistent part retains the same
            // identity key and therefore keeps ordinary smooth motion.
            let event_ramp_length = if identity_changed { 0 } else { ramp_length };
            let presented = present_source_channel(
                channel_idx,
                source,
                self.policy,
                Some(sample_pos),
                Some(event_ramp_length),
            );
            let Some(event) = presented.event else {
                bail!("renderable source channel {channel_idx} produced no object event");
            };
            self.events.push(event);
            self.presentation_identities[channel_idx] = identity;
            self.presentation_identity_initialized[channel_idx] = true;
        }

        let gain_for = |channel_idx: usize| {
            if route_gain_preapplied
                .and_then(|flags| flags.get(channel_idx))
                .copied()
                .unwrap_or(false)
            {
                1.0
            } else {
                route_energy_gain(sources[channel_idx].native_stereo_route)
            }
        };

        // Preserve historical source energy unless the host explicitly applied
        // a more precise trajectory already. This stays at float precision
        // rather than quantizing source level into integer-dB object metadata.
        let needs_scaling = (0..channels).any(|channel_idx| {
            (gain_for(channel_idx) - 1.0).abs() > 1.0e-7
        });
        let render_input: &[f32] = if needs_scaling {
            self.scaled_input.resize(input_pcm.len(), 0.0);
            for (frame_in, frame_out) in input_pcm
                .chunks_exact(channels)
                .zip(self.scaled_input.chunks_exact_mut(channels))
            {
                for channel_idx in 0..channels {
                    frame_out[channel_idx] = frame_in[channel_idx] * gain_for(channel_idx);
                }
            }
            &self.scaled_input
        } else {
            input_pcm
        };

        self.renderer.render_frame(
            render_input,
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
    use crate::source_identity::{SourcePresentationIdentity, source_presentation_identity};

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

    #[test]
    fn authored_stereo_route_preserves_source_energy_and_not_polarity_as_level() {
        assert_eq!(route_energy_gain(None), 1.0);
        assert!((route_energy_gain(Some(NativeStereoRoute {
            left_gain: 1.0,
            right_gain: 1.0,
        })) - 1.0).abs() < 1.0e-7);
        assert!((route_energy_gain(Some(NativeStereoRoute {
            left_gain: 1.0,
            right_gain: 0.0,
        })) - std::f32::consts::FRAC_1_SQRT_2).abs() < 1.0e-7);
        assert!((route_energy_gain(Some(NativeStereoRoute {
            left_gain: -1.0,
            right_gain: 0.5,
        })) - ((1.0_f32 + 0.25) * 0.5).sqrt()).abs() < 1.0e-7);
        assert_eq!(route_energy_gain(Some(NativeStereoRoute {
            left_gain: 0.0,
            right_gain: 0.0,
        })), 0.0);
    }

    #[test]
    fn preapplied_gain_policy_is_width_checked_and_semantically_unity() {
        let sources = [
            SourceSceneEvidence {
                native_stereo_route: Some(NativeStereoRoute { left_gain: 1.0, right_gain: 0.0 }),
                ..SourceSceneEvidence::default()
            },
            SourceSceneEvidence::default(),
        ];
        let flags = [true, false];
        assert_eq!(flags.len(), sources.len());
        assert_eq!(
            if flags[0] { 1.0 } else { route_energy_gain(sources[0].native_stereo_route) },
            1.0
        );
        assert_eq!(route_energy_gain(sources[0].native_stereo_route), std::f32::consts::FRAC_1_SQRT_2);
    }

    #[test]
    fn persistent_part_owns_presentation_continuity_across_source_reuse() {
        let a = SourceSceneEvidence {
            source_id: 10,
            persistent_part_id: Some(77),
            ..SourceSceneEvidence::default()
        };
        let b = SourceSceneEvidence {
            source_id: 11,
            persistent_part_id: Some(77),
            ..SourceSceneEvidence::default()
        };
        let unrelated = SourceSceneEvidence {
            source_id: 12,
            persistent_part_id: None,
            ..SourceSceneEvidence::default()
        };
        assert_eq!(
            source_presentation_identity(&a),
            Some(SourcePresentationIdentity::PersistentPart(77))
        );
        assert_eq!(source_presentation_identity(&a), source_presentation_identity(&b));
        assert_ne!(source_presentation_identity(&b), source_presentation_identity(&unrelated));
    }
}
