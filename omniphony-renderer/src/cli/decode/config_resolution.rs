use crate::cli::command::{
    Cli, EvaluationModeArg, LogFormat, LogLevel, OutputBackend, RampModeArg, RenderArgSources,
    RenderArgs,
};
use anyhow::Result;

pub(super) fn merge_render_config(
    cfg: &renderer::config::RenderConfig,
    args: &mut RenderArgs,
    arg_sources: &RenderArgSources<'_>,
) {
    use std::str::FromStr;

    // --- Option fields: fill only when None ---
    if args.speaker_layout.is_none() {
        args.speaker_layout = cfg.speaker_layout.clone();
    }
    if args.vbap_table.is_none() {
        args.vbap_table = cfg.vbap_table.clone();
    }
    if args.output_sample_rate.is_none() {
        args.output_sample_rate = cfg.output_sample_rate;
    }
    if !arg_sources.is_explicit("ramp_mode") {
        if let Some(ref v) = cfg.ramp_mode {
            if let Some(mode) = renderer::live_params::RampMode::from_str(v) {
                args.ramp_mode = match mode {
                    renderer::live_params::RampMode::Off => RampModeArg::Off,
                    renderer::live_params::RampMode::Frame => RampModeArg::Frame,
                    renderer::live_params::RampMode::Sample => RampModeArg::Sample,
                };
            }
        }
    }
    if args.bridge_path.is_none() {
        args.bridge_path = cfg.bridge_path.clone();
    }
    // In continuous (studio bridge) mode the config is the source of truth for the
    // input pipe, overriding the positional default the studio passes at launch.
    if args.continuous {
        if let Some(ref input_pipe) = cfg.input_pipe {
            args.input = Some(input_pipe.clone());
        }
    }
    // Note: drc_mode currently doesn't have a CLI arg, it's OSC/config only.
    // --- Fields with defaults: apply config only when value equals the clap default ---
    // (If the user explicitly passes the default value, config is ignored — acceptable edge case.)
    if !arg_sources.is_explicit("output_backend") {
        if let Some(ref s) = cfg.output_backend {
            if let Ok(f) = OutputBackend::from_str(s) {
                args.output_backend = Some(f);
            }
        }
    }
    if !arg_sources.is_explicit("presentation") {
        if let Some(p) = cfg.presentation {
            args.presentation = p.to_string();
        }
    }
    if !arg_sources.is_explicit("osc_host") {
        if let Some(ref h) = cfg.osc_host {
            args.osc_host = h.clone();
        }
    }
    if !arg_sources.is_explicit("osc_port") {
        if let Some(p) = cfg.osc_port {
            args.osc_port = p;
        }
    }
    if !arg_sources.is_explicit("evaluation_polar_azimuth_resolution") {
        if let Some(v) = renderer::config_fields::vbap_azimuth_resolution::get(cfg) {
            args.evaluation_polar_azimuth_resolution = v;
        }
    }
    if !arg_sources.is_explicit("evaluation_polar_elevation_resolution") {
        if let Some(v) = renderer::config_fields::vbap_elevation_resolution::get(cfg) {
            args.evaluation_polar_elevation_resolution = v;
        }
    }
    if !arg_sources.is_explicit("vbap_spread") {
        if let Some(v) = cfg.vbap_spread {
            args.vbap_spread = v;
        }
    }
    if !arg_sources.is_explicit("evaluation_polar_distance_res") {
        if let Some(v) = renderer::config_fields::vbap_distance_res::get(cfg) {
            args.evaluation_polar_distance_res = v;
        }
    }
    if !arg_sources.is_explicit("evaluation_polar_distance_max") {
        if let Some(v) = renderer::config_fields::vbap_distance_max::get(cfg) {
            args.evaluation_polar_distance_max = v;
        }
    }
    if !arg_sources.is_explicit("render_evaluation_position_interpolation")
        && !arg_sources.is_explicit("no_render_evaluation_position_interpolation")
    {
        args.render_evaluation_position_interpolation =
            renderer::config_fields::render_evaluation_position_interpolation::get(cfg).unwrap_or(
                renderer::config_fields::render_evaluation_position_interpolation::DEFAULT,
            );
    } else if args.no_render_evaluation_position_interpolation {
        args.render_evaluation_position_interpolation = false;
    }
    if !arg_sources.is_explicit("render_evaluation_mode") {
        if let Some(ref v) = cfg.render_evaluation_mode {
            if v.eq_ignore_ascii_case("precomputed_cartesian")
                || v.eq_ignore_ascii_case("cartesian")
            {
                args.render_evaluation_mode = EvaluationModeArg::Cartesian;
            } else if v.eq_ignore_ascii_case("precomputed_polar") || v.eq_ignore_ascii_case("polar")
            {
                args.render_evaluation_mode = EvaluationModeArg::Polar;
            }
        }
    }
    if args.evaluation_cartesian_x_size.is_none() {
        args.evaluation_cartesian_x_size = cfg.evaluation_cartesian_x_size;
    }
    if args.evaluation_cartesian_y_size.is_none() {
        args.evaluation_cartesian_y_size = cfg.evaluation_cartesian_y_size;
    }
    if args.evaluation_cartesian_z_size.is_none() {
        args.evaluation_cartesian_z_size = cfg.evaluation_cartesian_z_size;
    }
    if args.evaluation_cartesian_z_neg_size.is_none() {
        args.evaluation_cartesian_z_neg_size = cfg.evaluation_cartesian_z_neg_size;
    }
    if !arg_sources.is_explicit("vbap_allow_negative_z")
        && !arg_sources.is_explicit("no_vbap_allow_negative_z")
    {
        match cfg.vbap_allow_negative_z {
            Some(true) => args.vbap_allow_negative_z = true,
            Some(false) => args.no_vbap_allow_negative_z = true,
            None => {}
        }
    }
    if !arg_sources.is_explicit("vbap_distance_model") {
        if let Some(ref v) = cfg.vbap_distance_model {
            args.vbap_distance_model = v.clone();
        }
    }
    if !arg_sources.is_explicit("master_gain") {
        if let Some(v) = cfg.master_gain {
            args.master_gain = v;
        }
    }
    if !arg_sources.is_explicit("room_ratio") {
        if let Some(ref v) = cfg.room_ratio {
            args.room_ratio = v.clone();
        }
    }
    if args.room_ratio_rear.is_none() {
        args.room_ratio_rear = cfg.room_ratio_rear;
    }
    if args.room_ratio_lower.is_none() {
        args.room_ratio_lower = cfg.room_ratio_lower;
    }
    if args.room_ratio_center_blend.is_none() {
        args.room_ratio_center_blend = cfg.room_ratio_center_blend;
    }
    if !arg_sources.is_explicit("spread_distance_range") {
        if let Some(v) = cfg.spread_distance_range {
            args.spread_distance_range = v;
        }
    }
    if !arg_sources.is_explicit("spread_distance_curve") {
        if let Some(v) = cfg.spread_distance_curve {
            args.spread_distance_curve = v;
        }
    }
    if !arg_sources.is_explicit("vbap_spread_min") {
        if let Some(v) = renderer::config_fields::vbap_spread_min::get(cfg) {
            args.vbap_spread_min = v;
        }
    }
    if !arg_sources.is_explicit("vbap_spread_max") {
        if let Some(v) = renderer::config_fields::vbap_spread_max::get(cfg) {
            args.vbap_spread_max = v;
        }
    }

    // Platform-specific Option fields
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    if args.latency_target_ms.is_none() {
        args.latency_target_ms = cfg.latency_target;
    }
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    if args.output_device.is_none() {
        if let Some(ref s) = cfg.output_device {
            args.output_device = Some(s.clone());
        }
    }

    // --- Bool fields: CLI enable/disable flags override config; absent → use config ---
    // enable_vbap
    if !arg_sources.is_explicit("enable_vbap") && !arg_sources.is_explicit("disable_vbap") {
        args.enable_vbap = cfg.enable_vbap.unwrap_or(false);
    } else if args.disable_vbap {
        args.enable_vbap = false;
    }
    // osc
    if !arg_sources.is_explicit("osc") && !arg_sources.is_explicit("no_osc") {
        args.osc = cfg.osc.unwrap_or(false);
    } else if args.no_osc {
        args.osc = false;
    }
    // osc_rx_port (config can override the default 9000)
    if !arg_sources.is_explicit("osc_rx_port") {
        if let Some(p) = cfg.osc_rx_port {
            args.osc_rx_port = p;
        }
    }
    // continuous
    if !arg_sources.is_explicit("continuous") && !arg_sources.is_explicit("no_continuous") {
        args.continuous = cfg.continuous.unwrap_or(false);
    } else if args.no_continuous {
        args.continuous = false;
    }
    // use_loudness
    if !arg_sources.is_explicit("use_loudness") && !arg_sources.is_explicit("no_loudness") {
        args.use_loudness = cfg.use_loudness.unwrap_or(false);
    } else if args.no_loudness {
        args.use_loudness = false;
    }
    // auto_gain
    if !arg_sources.is_explicit("auto_gain") && !arg_sources.is_explicit("no_auto_gain") {
        args.auto_gain = cfg.auto_gain.unwrap_or(false);
    } else if args.no_auto_gain {
        args.auto_gain = false;
    }
    // bed_conform
    if !arg_sources.is_explicit("bed_conform") && !arg_sources.is_explicit("no_bed_conform") {
        args.bed_conform = cfg.bed_conform.unwrap_or(false);
    } else if args.no_bed_conform {
        args.bed_conform = false;
    }
    // enable_adaptive_resampling
    if !arg_sources.is_explicit("enable_adaptive_resampling")
        && !arg_sources.is_explicit("disable_adaptive_resampling")
    {
        args.enable_adaptive_resampling = cfg.enable_adaptive_resampling.unwrap_or(false);
    } else if args.disable_adaptive_resampling {
        args.enable_adaptive_resampling = false;
    }
    // spread_from_distance
    if !arg_sources.is_explicit("spread_from_distance")
        && !arg_sources.is_explicit("no_spread_from_distance")
    {
        args.spread_from_distance = cfg.spread_from_distance.unwrap_or(false);
    } else if args.no_spread_from_distance {
        args.spread_from_distance = false;
    }
    // distance_diffuse (bool flag — no --no- override needed, just the flag)
    if !arg_sources.is_explicit("distance_diffuse") {
        args.distance_diffuse = cfg.distance_diffuse.unwrap_or(false);
    }
    if args.no_vbap_allow_negative_z {
        args.vbap_allow_negative_z = false;
    }
    if !arg_sources.is_explicit("distance_diffuse_threshold") {
        if let Some(v) = cfg.distance_diffuse_threshold {
            args.distance_diffuse_threshold = v;
        }
    }
    if !arg_sources.is_explicit("distance_diffuse_curve") {
        if let Some(v) = cfg.distance_diffuse_curve {
            args.distance_diffuse_curve = v;
        }
    }
}

pub(super) fn effective_to_config(
    args: &RenderArgs,
    cli: &Cli,
    existing_render_cfg: Option<&renderer::config::RenderConfig>,
) -> Result<renderer::config::Config> {
    use renderer::config::{Config, GlobalConfig};
    use renderer::speaker_layout::SpeakerLayout;

    let global = GlobalConfig {
        loglevel: if cli.loglevel != LogLevel::default() {
            Some(format!("{:?}", cli.loglevel).to_lowercase())
        } else {
            None
        },
        log_format: if cli.log_format != LogFormat::default() {
            Some(format!("{:?}", cli.log_format).to_lowercase())
        } else {
            None
        },
        strict: if cli.strict { Some(true) } else { None },
        extra: Default::default(),
    };

    // Start from the existing config so every field the CLI cannot express
    // (live_input, embedded current_layout, render_backend, hybrid_*,
    // experimental_*, distance metrics, adaptive tuning, DRC, monitoring
    // cadences, size_to_spread, and any unknown `extra` keys) is preserved
    // verbatim instead of being erased. `merge_render_config` has already
    // folded the on-disk config into `args`, so re-storing the args value
    // re-persists anything the user did not explicitly override on the CLI.
    let mut render = existing_render_cfg.cloned().unwrap_or_default();

    render.input_pipe = if args.continuous {
        args.input.clone()
    } else {
        None
    };
    render.output_backend = match args.output_backend {
        Some(value) if Some(value) != OutputBackend::platform_default() => {
            Some(format!("{:?}", value).to_lowercase())
        }
        _ => None,
    };
    render.presentation = if args.presentation != "best" {
        args.presentation.parse::<u8>().ok()
    } else {
        None
    };
    render.bridge_path = args.bridge_path.clone();
    render.enable_vbap = if args.enable_vbap { Some(true) } else { None };
    // Persist the embedded layout instead of a path link. Only override when a
    // layout path is supplied on the CLI; otherwise keep the config's existing
    // embedded `current_layout` (Studio-saved) intact.
    if let Some(ref layout_path) = args.speaker_layout {
        render.current_layout = Some(SpeakerLayout::from_file(layout_path)?);
        render.speaker_layout = None;
    }
    render.vbap_table = args.vbap_table.clone();
    renderer::config_fields::vbap_azimuth_resolution::store(
        &mut render,
        args.evaluation_polar_azimuth_resolution,
    );
    renderer::config_fields::vbap_elevation_resolution::store(
        &mut render,
        args.evaluation_polar_elevation_resolution,
    );
    render.vbap_spread = if args.vbap_spread != 0.0 {
        Some(args.vbap_spread)
    } else {
        None
    };
    renderer::config_fields::vbap_distance_res::store(
        &mut render,
        args.evaluation_polar_distance_res,
    );
    renderer::config_fields::vbap_distance_max::store(
        &mut render,
        args.evaluation_polar_distance_max,
    );
    renderer::config_fields::render_evaluation_position_interpolation::store(
        &mut render,
        args.render_evaluation_position_interpolation,
    );
    render.render_evaluation_mode = if args.render_evaluation_mode != EvaluationModeArg::Polar {
        Some(format!("{:?}", args.render_evaluation_mode).to_lowercase())
    } else {
        None
    };
    render.evaluation_cartesian_x_size = args.evaluation_cartesian_x_size;
    render.evaluation_cartesian_y_size = args.evaluation_cartesian_y_size;
    render.evaluation_cartesian_z_size = args.evaluation_cartesian_z_size;
    render.evaluation_cartesian_z_neg_size = args.evaluation_cartesian_z_neg_size;
    render.vbap_allow_negative_z = if args.vbap_allow_negative_z {
        Some(true)
    } else if args.no_vbap_allow_negative_z {
        Some(false)
    } else {
        None
    };
    render.vbap_distance_model = if args.vbap_distance_model != "none" {
        Some(args.vbap_distance_model.clone())
    } else {
        None
    };
    render.master_gain = if args.master_gain != 0.0 {
        Some(args.master_gain)
    } else {
        None
    };
    // The CLI works in ratios; metres are a save-time representation only, so
    // clear any metre fields a prior Studio save left behind to keep the ratio
    // representation authoritative on the next load.
    render.room_width_m = None;
    render.room_front_m = None;
    render.room_rear_m = None;
    render.room_height_m = None;
    render.room_lower_m = None;
    render.room_ratio = if args.room_ratio != "1.0,2.0,1.0" {
        Some(args.room_ratio.clone())
    } else {
        None
    };
    render.room_ratio_rear = args.room_ratio_rear;
    render.room_ratio_lower = args.room_ratio_lower;
    render.room_ratio_center_blend = args.room_ratio_center_blend;
    render.osc = if args.osc { Some(true) } else { None };
    render.osc_metering = if args.osc_metering { Some(true) } else { None };
    render.osc_rx_port = if args.osc_rx_port != 9000 {
        Some(args.osc_rx_port)
    } else {
        None
    };
    render.osc_host = if args.osc_host != "127.0.0.1" {
        Some(args.osc_host.clone())
    } else {
        None
    };
    render.osc_port = if args.osc_port != 9000 {
        Some(args.osc_port)
    } else {
        None
    };
    #[cfg(any(target_os = "linux", target_os = "windows"))]
    {
        render.output_device = args.output_device.clone();
        render.latency_target = args.latency_target_ms;
    }
    render.continuous = if args.continuous { Some(true) } else { None };
    render.use_loudness = if args.use_loudness { Some(true) } else { None };
    render.auto_gain = if args.auto_gain { Some(true) } else { None };
    render.bed_conform = if args.bed_conform { Some(true) } else { None };
    render.spread_from_distance = if args.spread_from_distance {
        Some(true)
    } else {
        None
    };
    render.spread_distance_range = if args.spread_distance_range != 1.0 {
        Some(args.spread_distance_range)
    } else {
        None
    };
    render.spread_distance_curve = if args.spread_distance_curve != 1.0 {
        Some(args.spread_distance_curve)
    } else {
        None
    };
    renderer::config_fields::vbap_spread_min::store(&mut render, args.vbap_spread_min);
    renderer::config_fields::vbap_spread_max::store(&mut render, args.vbap_spread_max);
    render.enable_adaptive_resampling = if args.enable_adaptive_resampling {
        Some(true)
    } else {
        None
    };
    render.adaptive_resampling_update_interval_callbacks =
        args.adaptive_resampling_update_interval_callbacks;
    render.output_sample_rate = args.output_sample_rate;
    render.ramp_mode = if args.ramp_mode != RampModeArg::Sample {
        Some(match args.ramp_mode {
            RampModeArg::Off => "off".to_string(),
            RampModeArg::Frame => "frame".to_string(),
            RampModeArg::Sample => "sample".to_string(),
        })
    } else {
        None
    };
    render.distance_diffuse = if args.distance_diffuse {
        Some(true)
    } else {
        None
    };
    render.distance_diffuse_threshold = if args.distance_diffuse_threshold != 1.0 {
        Some(args.distance_diffuse_threshold)
    } else {
        None
    };
    render.distance_diffuse_curve = if args.distance_diffuse_curve != 1.0 {
        Some(args.distance_diffuse_curve)
    } else {
        None
    };

    let global_opt =
        if global.loglevel.is_none() && global.log_format.is_none() && global.strict.is_none() {
            None
        } else {
            Some(global)
        };

    Ok(Config {
        global: global_opt,
        render: Some(render),
        extra: Default::default(),
    })
}

#[cfg(test)]
mod tests {
    use super::effective_to_config;
    use crate::cli::command::{Commands, ParsedCli};

    /// Build a fully-defaulted `render` arg set + `Cli` straight from clap, so
    /// the test exercises the real defaults rather than a hand-rolled struct.
    fn default_render_invocation() -> (crate::cli::command::Cli, crate::cli::command::RenderArgs) {
        let parsed = ParsedCli::parse_from(["orender", "render"]).expect("parse defaults");
        let cli = parsed.cli;
        let args = match cli.command {
            Commands::Render(ref a) => a.clone(),
            _ => unreachable!("explicit render subcommand"),
        };
        (cli, args)
    }

    /// Regression guard for the report's §8.2: `--save-config` must NOT erase
    /// fields the CLI cannot express. Previously `effective_to_config` rebuilt
    /// the config from scratch and dropped these; now it mutates the existing
    /// config, so they survive a save with no `--speaker-layout`.
    #[test]
    fn save_config_preserves_cli_unexpressible_fields() {
        let (cli, args) = default_render_invocation();

        let existing = renderer::config::RenderConfig {
            live_input: Some(renderer::config::LiveInputConfig {
                node: Some("omniphony_live".to_string()),
                ..Default::default()
            }),
            input_mode: Some(renderer::config::InputModeConfig::Live),
            current_layout: Some(renderer::speaker_layout::SpeakerLayout {
                radius_m: 1.5,
                speakers: vec![],
            }),
            hybrid_external_backend: Some("cube".to_string()),
            experimental_distance_min_active_speakers: Some(3),
            distance_model_metric: Some("chebyshev".to_string()),
            render_backend: Some("barycenter".to_string()),
            ..Default::default()
        };

        let out = effective_to_config(&args, &cli, Some(&existing)).expect("build config");
        let render = out.render.expect("render section present");

        assert!(render.live_input.is_some(), "live_input erased");
        assert_eq!(
            render.input_mode,
            Some(renderer::config::InputModeConfig::Live)
        );
        assert_eq!(
            render.current_layout.map(|l| l.radius_m),
            Some(1.5),
            "embedded current_layout erased"
        );
        assert_eq!(render.hybrid_external_backend.as_deref(), Some("cube"));
        assert_eq!(render.experimental_distance_min_active_speakers, Some(3));
        assert_eq!(render.distance_model_metric.as_deref(), Some("chebyshev"));
        assert_eq!(render.render_backend.as_deref(), Some("barycenter"));
    }

    /// A CLI value still wins and is persisted (skip-if-default via the
    /// descriptor) even when starting from an existing config.
    #[test]
    fn save_config_persists_pilot_field_from_args() {
        let (cli, mut args) = default_render_invocation();
        args.evaluation_polar_distance_res = 12;

        let out = effective_to_config(&args, &cli, None).expect("build config");
        assert_eq!(out.render.unwrap().vbap_distance_res, Some(12));
    }
}
