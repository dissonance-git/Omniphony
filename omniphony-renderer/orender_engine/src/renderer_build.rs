//! Construction of a [`SpatialRenderer`] from neutral, host-agnostic parameters.
//!
//! Both the `orender` CLI and `orender_ffi` build the renderer through this one
//! function so that, given the same parameters, they produce an identical
//! renderer (and therefore bit-identical audio). The CLI fills
//! [`SpatialRendererParams`] from its parsed args; the FFI fills it from a YAML
//! [`RenderConfig`].

use anyhow::{Result, anyhow, bail};
use bridge_api::{RVbapCartesianDefaults, RVbapTableMode};
use renderer::config::RenderConfig;
use renderer::render_backend::RenderBackendKind;
use renderer::live_params::{ExperimentalDistanceLiveParams, LiveEvaluationMode, PreferredEvaluationMode};
use renderer::speaker_layout::SpeakerLayout;
use renderer::spatial_renderer::SpatialRenderer;
use renderer::spatial_vbap::{DistanceModel, VbapTableMode};
use std::path::PathBuf;
use std::str::FromStr;

/// VBAP pre-computed table mode (mirror of the CLI's `EvaluationModeArg`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum EvalMode {
    Polar,
    Cartesian,
}

/// Host-neutral inputs to [`build_spatial_renderer`]. Field names and semantics
/// mirror the `render` CLI args / config keys.
#[derive(Debug, Clone)]
pub struct SpatialRendererParams {
    pub vbap_table: Option<PathBuf>,
    pub evaluation_polar_azimuth_resolution: i32,
    pub evaluation_polar_elevation_resolution: i32,
    pub evaluation_polar_distance_res: i32,
    pub evaluation_polar_distance_max: f32,
    pub render_evaluation_mode: EvalMode,
    pub evaluation_mode_explicit: bool,
    pub evaluation_cartesian_x_size: Option<usize>,
    pub evaluation_cartesian_y_size: Option<usize>,
    pub evaluation_cartesian_z_size: Option<usize>,
    pub evaluation_cartesian_z_neg_size: Option<usize>,
    pub vbap_allow_negative_z: bool,
    pub no_vbap_allow_negative_z: bool,
    pub render_evaluation_position_interpolation: bool,
    pub vbap_distance_model: String,
    pub spread_from_distance: bool,
    pub spread_distance_range: f32,
    pub spread_distance_curve: f32,
    pub vbap_spread_min: f32,
    pub vbap_spread_max: f32,
    pub log_object_positions: bool,
    pub room_ratio: String,
    pub room_ratio_rear: Option<f32>,
    pub room_ratio_lower: Option<f32>,
    pub room_ratio_center_blend: Option<f32>,
    pub master_gain: f32,
    pub auto_gain: bool,
    pub use_loudness: bool,
    pub distance_diffuse: bool,
    pub distance_diffuse_threshold: f32,
    pub distance_diffuse_curve: f32,
}

fn parse_room_ratio(params: &SpatialRendererParams) -> Result<([f32; 3], f32, f32, f32)> {
    let parts: Vec<&str> = params.room_ratio.split(',').collect();
    if parts.len() != 3 {
        bail!(
            "Invalid room-ratio format '{}'. Expected 'width,length,height' (e.g., '1.0,2.0,0.5')",
            params.room_ratio
        );
    }
    let room_ratio = [
        parts[0]
            .trim()
            .parse::<f32>()
            .map_err(|_| anyhow!("Invalid room-ratio width: '{}'", parts[0]))?,
        parts[1]
            .trim()
            .parse::<f32>()
            .map_err(|_| anyhow!("Invalid room-ratio length: '{}'", parts[1]))?,
        parts[2]
            .trim()
            .parse::<f32>()
            .map_err(|_| anyhow!("Invalid room-ratio height: '{}'", parts[2]))?,
    ];
    let room_ratio_rear = params.room_ratio_rear.unwrap_or(room_ratio[1]).max(0.01);
    let room_ratio_lower = params.room_ratio_lower.unwrap_or(0.5).max(0.01);
    let room_ratio_center_blend = params.room_ratio_center_blend.unwrap_or(0.5).clamp(0.0, 1.0);
    Ok((
        room_ratio,
        room_ratio_rear,
        room_ratio_lower,
        room_ratio_center_blend,
    ))
}

fn resolve_evaluation_table_mode(
    params: &SpatialRendererParams,
    vbap_cartesian_defaults: RVbapCartesianDefaults,
) -> Result<(VbapTableMode, bool)> {
    let vbap_allow_negative_z = if params.vbap_allow_negative_z {
        true
    } else if params.no_vbap_allow_negative_z {
        false
    } else {
        vbap_cartesian_defaults.allow_negative_z
    };
    let vbap_table_mode = match params.render_evaluation_mode {
        EvalMode::Polar => VbapTableMode::Polar,
        EvalMode::Cartesian => {
            let x_cells = params
                .evaluation_cartesian_x_size
                .unwrap_or(vbap_cartesian_defaults.x_size as usize);
            let y_cells = params
                .evaluation_cartesian_y_size
                .unwrap_or(vbap_cartesian_defaults.y_size as usize);
            let z_cells = params
                .evaluation_cartesian_z_size
                .unwrap_or(vbap_cartesian_defaults.z_size as usize);
            let z_neg_cells = params.evaluation_cartesian_z_neg_size.unwrap_or(0);
            if x_cells < 1 || y_cells < 1 || z_cells < 1 {
                bail!(
                    "Invalid cartesian VBAP cell count: x={}, y={}, z+={} (each must be >= 1)",
                    x_cells,
                    y_cells,
                    z_cells
                );
            }
            VbapTableMode::Cartesian {
                x_size: x_cells + 1,
                y_size: y_cells + 1,
                z_size: z_cells + 1,
                z_neg_size: z_neg_cells,
            }
        }
    };
    Ok((vbap_table_mode, vbap_allow_negative_z))
}

/// Build a fully-configured [`SpatialRenderer`] from `params` and the bridge's
/// suggested defaults, applying any backend/evaluation/experimental-distance
/// overrides from `render_cfg`.
pub fn build_spatial_renderer(
    params: &SpatialRendererParams,
    layout: SpeakerLayout,
    sample_rate: u32,
    vbap_cartesian_defaults: RVbapCartesianDefaults,
    preferred_evaluation_mode: RVbapTableMode,
    render_cfg: Option<&RenderConfig>,
) -> Result<SpatialRenderer> {
    let distance_model = DistanceModel::from_str(&params.vbap_distance_model)
        .map_err(|e| anyhow!("Invalid distance model: {}", e))?;
    let (room_ratio, room_ratio_rear, room_ratio_lower, room_ratio_center_blend) =
        parse_room_ratio(params)?;

    let (vbap_table_mode, vbap_allow_negative_z) =
        resolve_evaluation_table_mode(params, vbap_cartesian_defaults)?;

    log::info!("VBAP allow_negative_z: {}", vbap_allow_negative_z);

    if let Some(ref vbap_table_path) = params.vbap_table {
        bail!(
            "loading precomputed renderer state from file is no longer supported ({})",
            vbap_table_path.display()
        );
    }

    let renderer = {
        log::info!(
            "Speaker layout: {} speakers ({})",
            layout.num_speakers(),
            layout.speaker_names().join(", ")
        );
        log::info!("Generating VBAP table at runtime (this may take a few seconds)...");
        let start_time = std::time::Instant::now();
        let azimuth_cells = params.evaluation_polar_azimuth_resolution.max(1);
        let elevation_cells = params.evaluation_polar_elevation_resolution.max(1);
        let distance_cells = params.evaluation_polar_distance_res.max(1);
        let azimuth_step_deg = (360.0f32 / (azimuth_cells as f32)).max(1.0).round() as i32;
        let elevation_step_deg = (((if vbap_allow_negative_z { 180.0 } else { 90.0 })
            / (elevation_cells as f32))
            .max(1.0)
            .round()) as i32;
        let distance_step =
            params.evaluation_polar_distance_max.max(0.01) / (distance_cells as f32);

        let renderer = SpatialRenderer::new(
            layout,
            sample_rate,
            azimuth_step_deg,
            elevation_step_deg,
            distance_step,
            params.evaluation_polar_distance_max,
            vbap_table_mode,
            vbap_allow_negative_z,
            params.render_evaluation_position_interpolation,
            distance_model,
            params.spread_from_distance,
            params.spread_distance_range,
            params.spread_distance_curve,
            params.vbap_spread_min,
            params.vbap_spread_max,
            params.log_object_positions,
            room_ratio,
            room_ratio_rear,
            room_ratio_lower,
            room_ratio_center_blend,
            params.master_gain,
            params.auto_gain,
            params.use_loudness,
            params.distance_diffuse,
            params.distance_diffuse_threshold,
            params.distance_diffuse_curve,
            match preferred_evaluation_mode {
                RVbapTableMode::Polar => PreferredEvaluationMode::PrecomputedPolar,
                RVbapTableMode::Cartesian => PreferredEvaluationMode::PrecomputedCartesian,
            },
            if params.evaluation_mode_explicit {
                match params.render_evaluation_mode {
                    EvalMode::Polar => LiveEvaluationMode::PrecomputedPolar,
                    EvalMode::Cartesian => LiveEvaluationMode::PrecomputedCartesian,
                }
            } else {
                LiveEvaluationMode::Auto
            },
            params
                .evaluation_cartesian_x_size
                .unwrap_or(vbap_cartesian_defaults.x_size as usize),
            params
                .evaluation_cartesian_y_size
                .unwrap_or(vbap_cartesian_defaults.y_size as usize),
            params
                .evaluation_cartesian_z_size
                .unwrap_or(vbap_cartesian_defaults.z_size as usize),
            params.evaluation_cartesian_z_neg_size.unwrap_or(0),
        )?;
        let elapsed = start_time.elapsed();
        log::info!("VBAP table generated in {:.2}s", elapsed.as_secs_f64());
        renderer
    };

    log::info!("VBAP spatial rendering enabled");
    let configured_backend = render_cfg
        .and_then(|cfg| cfg.render_backend.as_deref())
        .and_then(RenderBackendKind::from_str);
    let configured_evaluation = render_cfg
        .and_then(|cfg| cfg.render_evaluation_mode.as_deref())
        .and_then(LiveEvaluationMode::from_str);
    let experimental_distance_cfg = render_cfg.map(|cfg| {
        let defaults = ExperimentalDistanceLiveParams::default();
        ExperimentalDistanceLiveParams {
            distance_floor: cfg
                .experimental_distance_distance_floor
                .unwrap_or(defaults.distance_floor)
                .max(0.0),
            min_active_speakers: cfg
                .experimental_distance_min_active_speakers
                .unwrap_or(defaults.min_active_speakers)
                .max(1),
            max_active_speakers: cfg
                .experimental_distance_max_active_speakers
                .unwrap_or(defaults.max_active_speakers)
                .max(1),
            position_error_floor: cfg
                .experimental_distance_position_error_floor
                .unwrap_or(defaults.position_error_floor)
                .max(0.0),
            position_error_nearest_scale: cfg
                .experimental_distance_position_error_nearest_scale
                .unwrap_or(defaults.position_error_nearest_scale)
                .max(0.0),
            position_error_span_scale: cfg
                .experimental_distance_position_error_span_scale
                .unwrap_or(defaults.position_error_span_scale)
                .max(0.0),
        }
    });
    if configured_backend.is_some() || configured_evaluation.is_some() {
        let control = renderer.renderer_control();
        let mut requires_rebuild = false;
        {
            let mut live = control.live.write().unwrap();
            if let Some(configured_backend) = configured_backend {
                if live.backend_id() != configured_backend.as_str() {
                    live.backend_id = configured_backend.as_str().to_string();
                    requires_rebuild = true;
                }
            }
            if let Some(configured_evaluation) = configured_evaluation {
                if live.evaluation.mode != configured_evaluation {
                    live.set_evaluation_mode(configured_evaluation);
                    requires_rebuild = true;
                }
            }
            if let Some(mut experimental_distance) = experimental_distance_cfg {
                if experimental_distance.max_active_speakers
                    < experimental_distance.min_active_speakers
                {
                    experimental_distance.max_active_speakers =
                        experimental_distance.min_active_speakers;
                }
                if live.experimental_distance.distance_floor != experimental_distance.distance_floor
                    || live.experimental_distance.min_active_speakers
                        != experimental_distance.min_active_speakers
                    || live.experimental_distance.max_active_speakers
                        != experimental_distance.max_active_speakers
                    || live.experimental_distance.position_error_floor
                        != experimental_distance.position_error_floor
                    || live.experimental_distance.position_error_nearest_scale
                        != experimental_distance.position_error_nearest_scale
                    || live.experimental_distance.position_error_span_scale
                        != experimental_distance.position_error_span_scale
                {
                    live.experimental_distance = experimental_distance;
                    requires_rebuild = true;
                }
            }
        }
        if requires_rebuild {
            if let Some(plan) = control.prepare_topology_rebuild() {
                let topology = plan.build_topology()?;
                control.publish_topology(topology);
            }
        }
    } else if let Some(mut experimental_distance) = experimental_distance_cfg {
        if experimental_distance.max_active_speakers < experimental_distance.min_active_speakers {
            experimental_distance.max_active_speakers = experimental_distance.min_active_speakers;
        }
        renderer
            .renderer_control()
            .live
            .write()
            .unwrap()
            .experimental_distance = experimental_distance;
    }

    Ok(renderer)
}
