//! Construction policy for already-separated causal source lanes.
//!
//! This is intentionally host-agnostic. A foobar component, a native Windows
//! host, or a fixture can all build the same source renderer and therefore use
//! the same Omniphony binaural semantics.
//!
//! The game-music interpreter owns source truth. This module only chooses a
//! presentation policy and a renderer configuration for those source lanes.

use anyhow::Result;
use bridge_api::{RVbapCartesianDefaults, RVbapTableMode};
use renderer::binaural::HrirSource;
use renderer::config::RenderConfig;
use renderer::live_params::{BinauralMode, OutputMode, RampMode};
use renderer::source_frame::SourceFrameRenderer;
use renderer::source_scene::SourcePresentationPolicy;
use renderer::speaker_layout::SpeakerLayout;

use crate::renderer_build::{EvalMode, SpatialRendererParams, build_spatial_renderer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceSpatialMode {
    /// Preserve native laterality and stable source identity, but do not push
    /// support material toward rear/height/depth. Useful as the source-aware
    /// control immediately above historical stereo.
    NativeRouting,
    /// Use the full evidence-earned sphere with the measured HRTF path.
    FullSphere,
    /// Full sphere plus a small listening-room early-reflection field for
    /// stronger externalization. Content/shared source reverb remains separate.
    FullSphereExternalized,
}

impl SourceSpatialMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NativeRouting => "native_routing",
            Self::FullSphere => "full_sphere",
            Self::FullSphereExternalized => "full_sphere_externalized",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceRendererOptions {
    pub mode: SourceSpatialMode,
    /// Measured SAF/KEMAR is the default and can later be replaced by a
    /// listener-specific SOFA set without changing the source-scene contract.
    pub hrir_source: HrirSource,
    /// Metres represented by one ADM unit for binaural distance cues.
    pub unit_scale_m: f32,
    /// Early-reflection return level for the externalized mode only.
    pub reflection_level: f32,
    /// Small listening-room dimensions for externalization, not source reverb.
    pub reflection_room_size_m: [f32; 3],
}

impl Default for SourceRendererOptions {
    fn default() -> Self {
        Self {
            mode: SourceSpatialMode::FullSphere,
            hrir_source: HrirSource::SafKemar,
            unit_scale_m: 1.0,
            reflection_level: 0.22,
            reflection_room_size_m: [4.0, 5.0, 2.7],
        }
    }
}

fn presentation_policy(mode: SourceSpatialMode) -> SourcePresentationPolicy {
    match mode {
        SourceSpatialMode::NativeRouting => SourcePresentationPolicy {
            sphere_strength: 0.0,
            max_rear_azimuth_deg: 100.0,
            max_elevation_deg: 0.0,
            max_distance: 1.0,
        },
        SourceSpatialMode::FullSphere | SourceSpatialMode::FullSphereExternalized => {
            SourcePresentationPolicy {
                sphere_strength: 1.0,
                // A support object may live well behind the listener while
                // leaving a margin around the exact rear singularity.
                max_rear_azimuth_deg: 150.0,
                // Strong enough to create an unmistakable upper hemisphere;
                // vertical placement still requires explicit source/policy
                // affinity from the source-scene evidence.
                max_elevation_deg: 60.0,
                max_distance: 1.75,
            }
        }
    }
}

/// Build the source-aware Omniphony renderer used by game-music integrations.
///
/// The output path is always **direct binaural**: each causal source receives
/// its own HRTF/ITD render from its world position. The temporary speaker/VBAP
/// topology exists only because `SpatialRenderer` currently owns both output
/// modes in one construction object; it is bypassed by the audio hot path once
/// `OutputMode::Binaural + BinauralMode::Direct` is selected.
pub fn build_source_frame_renderer(
    sample_rate: u32,
    render_cfg: Option<&RenderConfig>,
    options: SourceRendererOptions,
) -> Result<SourceFrameRenderer> {
    let mut params = SpatialRendererParams::from_render_config(render_cfg);

    // Keep construction fast and deterministic. Direct binaural bypasses this
    // evaluation table during source rendering, but SpatialRenderer still owns
    // one coherent topology for live control and possible future mode changes.
    params.render_evaluation_mode = Some(EvalMode::Cartesian);
    params.evaluation_mode_explicit = true;
    params.evaluation_cartesian_x_size = Some(4);
    params.evaluation_cartesian_y_size = Some(4);
    params.evaluation_cartesian_z_size = Some(4);
    params.evaluation_cartesian_z_neg_size = Some(4);
    params.vbap_allow_negative_z = true;
    params.no_vbap_allow_negative_z = false;

    let defaults = RVbapCartesianDefaults {
        x_size: 4,
        y_size: 4,
        z_size: 4,
        allow_negative_z: true,
    };
    let layout = SpeakerLayout::preset_9_1_6()?;
    let renderer = build_spatial_renderer(
        &params,
        layout,
        sample_rate,
        defaults,
        RVbapTableMode::Cartesian,
        render_cfg,
    )?;

    {
        let control = renderer.renderer_control();
        let mut live = control.live.write();
        live.binaural.output_mode = OutputMode::Binaural;
        live.binaural.mode = BinauralMode::Direct;
        live.binaural.hrir_source = options.hrir_source;
        live.binaural.unit_scale_m = options.unit_scale_m.clamp(0.25, 4.0);
        live.binaural.air_absorption = true;
        live.binaural.reverb.enabled = false;
        live.binaural.reflections.enabled =
            options.mode == SourceSpatialMode::FullSphereExternalized;
        live.binaural.reflections.level = options.reflection_level.clamp(0.0, 1.0);
        live.binaural.reflections.room_size_m = [
            options.reflection_room_size_m[0].max(1.0),
            options.reflection_room_size_m[1].max(1.0),
            options.reflection_room_size_m[2].max(1.0),
        ];
        // Object position ramps remain smooth while keeping the game-music host
        // responsible for source timing. Direct binaural updates HRTF/ITD once
        // per source block, so frame ramps are the honest control granularity.
        live.ramp_mode = RampMode::Frame;
    }

    Ok(SourceFrameRenderer::new(
        renderer,
        presentation_policy(options.mode),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_mode_disables_inferred_depth_and_height() {
        let policy = presentation_policy(SourceSpatialMode::NativeRouting);
        assert_eq!(policy.sphere_strength, 0.0);
        assert_eq!(policy.max_elevation_deg, 0.0);
        assert_eq!(policy.max_distance, 1.0);
    }

    #[test]
    fn full_sphere_uses_rear_height_and_depth_capacity() {
        let policy = presentation_policy(SourceSpatialMode::FullSphere);
        assert_eq!(policy.sphere_strength, 1.0);
        assert!(policy.max_rear_azimuth_deg > 135.0);
        assert!(policy.max_elevation_deg >= 55.0);
        assert!(policy.max_distance > 1.5);
    }

    #[test]
    fn externalization_is_orthogonal_to_sphere_geometry() {
        assert_eq!(
            presentation_policy(SourceSpatialMode::FullSphere),
            presentation_policy(SourceSpatialMode::FullSphereExternalized)
        );
    }
}
