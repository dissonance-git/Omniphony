//! Construction policy for already-separated causal source lanes.
//!
//! This is intentionally host-agnostic. A foobar component, a native Windows
//! host, or a fixture can all build the same source renderer and therefore use
//! the same Omniphony binaural semantics.
//!
//! Retro VGM Compiler owns source truth. This module chooses the listening
//! presentation. FullSphere is deliberately an immersive remix mode, not a
//! claim that the historical source authored modern rear/height coordinates.

use anyhow::Result;
use bridge_api::{RVbapCartesianDefaults, RVbapTableMode};
use renderer::binaural::HrirSource;
use renderer::config::RenderConfig;
use renderer::live_params::{BinauralMode, OutputMode, RampMode};
use renderer::source_frame::SourceFrameRenderer;
use renderer::source_scene::{SharedWetPresentationPolicy, SourcePresentationPolicy};
use renderer::speaker_layout::SpeakerLayout;

use crate::renderer_build::{EvalMode, SpatialRendererParams, build_spatial_renderer};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceSpatialMode {
    /// Preserve native laterality and stable source identity, but do not add
    /// creative rear/height/depth. Useful as the source-aware control directly
    /// above the protected historical/reference mix.
    NativeRouting,
    /// Mix recovered real sources into Omniphony's full immersive field. Native
    /// route and authored geometry remain constraints; otherwise width, depth,
    /// height and extent are explicitly DERIVED production decisions.
    FullSphere,
}

impl SourceSpatialMode {
    pub fn as_str(self) -> &'static str {
        match self {
            Self::NativeRouting => "native_routing",
            Self::FullSphere => "full_sphere",
        }
    }
}

#[derive(Debug, Clone)]
pub struct SourceRendererOptions {
    pub mode: SourceSpatialMode,
    /// Listening-room early reflections are an externalization control, not
    /// part of source geometry. Keeping this independent lets a listener test
    /// full sphere dry versus the same exact scene with room cues.
    pub externalization: bool,
    /// Measured SAF/KEMAR is the default and can later be replaced by a
    /// listener-specific SOFA set without changing the source-scene contract.
    pub hrir_source: HrirSource,
    /// Metres represented by one ADM unit for binaural distance cues.
    pub unit_scale_m: f32,
    /// Early-reflection return level when externalization is enabled.
    pub reflection_level: f32,
    /// Small listening-room dimensions for externalization, not source reverb.
    pub reflection_room_size_m: [f32; 3],
}

impl Default for SourceRendererOptions {
    fn default() -> Self {
        Self {
            mode: SourceSpatialMode::FullSphere,
            externalization: false,
            hrir_source: HrirSource::SafKemar,
            unit_scale_m: 1.0,
            reflection_level: 0.22,
            reflection_room_size_m: [4.0, 5.0, 2.7],
        }
    }
}

pub fn source_presentation_policy(mode: SourceSpatialMode) -> SourcePresentationPolicy {
    match mode {
        SourceSpatialMode::NativeRouting => SourcePresentationPolicy {
            sphere_strength: 0.0,
            max_rear_azimuth_deg: 100.0,
            max_elevation_deg: 0.0,
            max_distance: 1.0,
            // The historical wet field still exists in the source mix, but the
            // control mode adds no modern field scale, height, depth, or extent.
            shared_wet: SharedWetPresentationPolicy {
                strength: 0.0,
                rear_azimuth_deg: 100.0,
                elevation_deg: 0.0,
                distance: 1.0,
                extent: [0.0, 0.0, 0.0],
            },
        },
        SourceSpatialMode::FullSphere => SourcePresentationPolicy {
            // FullSphere intentionally opens the source-native remix rather than
            // waiting for historical proof of a speaker coordinate that the old
            // format could never encode.
            sphere_strength: 1.0,
            // Dynamic source objects may live well behind the listener while
            // leaving a margin around the exact rear singularity.
            max_rear_azimuth_deg: 150.0,
            // Strong enough to create an unmistakable upper hemisphere. Musical
            // role and native routing still shape where each source actually goes.
            max_elevation_deg: 60.0,
            max_distance: 1.75,
            // Historical shared effects, especially S-DSP echo, form their own
            // environmental layer. Keep it wide and rearward but slightly below
            // the dry-object maximums so the direct musical scene remains legible.
            shared_wet: SharedWetPresentationPolicy {
                strength: 1.0,
                rear_azimuth_deg: 140.0,
                elevation_deg: 38.0,
                distance: 1.60,
                extent: [1.0, 0.95, 0.85],
            },
        },
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
        live.binaural.reflections.enabled = options.externalization;
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
        source_presentation_policy(options.mode),
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn native_mode_disables_creative_depth_height_and_wet_scale() {
        let policy = source_presentation_policy(SourceSpatialMode::NativeRouting);
        assert_eq!(policy.sphere_strength, 0.0);
        assert_eq!(policy.max_elevation_deg, 0.0);
        assert_eq!(policy.max_distance, 1.0);
        assert_eq!(policy.shared_wet.strength, 0.0);
        assert_eq!(policy.shared_wet.extent, [0.0, 0.0, 0.0]);
    }

    #[test]
    fn full_sphere_opens_immersive_rear_height_depth_and_wet_layer() {
        let policy = source_presentation_policy(SourceSpatialMode::FullSphere);
        assert_eq!(policy.sphere_strength, 1.0);
        assert!(policy.max_rear_azimuth_deg > 135.0);
        assert!(policy.max_elevation_deg >= 55.0);
        assert!(policy.max_distance > 1.5);
        assert!(policy.shared_wet.strength > 0.9);
        assert!(policy.shared_wet.rear_azimuth_deg > 120.0);
        assert!(policy.shared_wet.elevation_deg > 25.0);
        assert!(policy.shared_wet.distance > 1.4);
        assert!(policy.shared_wet.extent[0] > policy.shared_wet.extent[2]);
    }

    #[test]
    fn externalization_defaults_off_so_geometry_can_be_tested_alone() {
        let options = SourceRendererOptions::default();
        assert_eq!(options.mode, SourceSpatialMode::FullSphere);
        assert!(!options.externalization);
    }
}
