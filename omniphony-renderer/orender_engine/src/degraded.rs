//! Degraded "no decoder" reporter.
//!
//! When the decoder bridge can't be resolved or loaded, the embedded (mpv) host
//! returns NULL from `orender_create` so mpv falls back to its own native
//! decoder (audio keeps working). To still tell Studio *why* spatial rendering
//! didn't engage, we bring up a minimal renderer + OSC server with **no
//! decoder**, whose live-state snapshot carries the bridge error. Studio
//! registers the normal way (so its address is known — no guessing) and shows a
//! red banner.
//!
//! `orender_ffi` keeps one of these alive process-globally for the lifetime of
//! the host. Nothing ever calls into it; it exists only to serve OSC state.

use crate::engine::OscOptions;
use crate::osc::OscSender;
use crate::renderer_build::{SpatialRendererParams, build_spatial_renderer};
use anyhow::Result;
use renderer::config::Config;
use renderer::spatial_renderer::SpatialRenderer;
use renderer::speaker_layout::SpeakerLayout;
use std::net::SocketAddrV4;
use std::path::Path;
use std::str::FromStr;

// VBAP grid for the decoder-less reporter (mirrors the CLI's idle-bridge
// constants in `cli/decode/session_run.rs`). We never decode or render through
// it; the grid only has to be valid for `build_spatial_renderer` to succeed.
const REPORTER_VBAP_DEFAULTS: bridge_api::RVbapCartesianDefaults =
    bridge_api::RVbapCartesianDefaults {
        x_size: 62,
        y_size: 62,
        z_size: 15,
        allow_negative_z: false,
    };
const REPORTER_PREFERRED_MODE: bridge_api::RVbapTableMode = bridge_api::RVbapTableMode::Cartesian;

/// A decoder-less renderer + OSC server, kept alive only to report the bridge
/// error to Studio. Dropping it shuts the OSC server down.
pub struct DegradedReporter {
    // Kept alive; never called into. `_osc` owns the listener thread that
    // answers Studio's registration with the live-state bundle (which carries
    // the bridge error). `_renderer` owns the `RendererControl` that bundle
    // reads from.
    _osc: OscSender,
    _renderer: SpatialRenderer,
}

/// Build and start the degraded reporter. `config_yaml_path` seeds the layout /
/// room / OSC settings exactly as the real engine would, so Studio sees a
/// coherent (if non-decoding) state alongside `bridge_error`. The returned value
/// must be kept alive to keep the OSC server up.
pub fn start_degraded_reporter(
    config_yaml_path: Option<&Path>,
    sample_rate: u32,
    osc: OscOptions,
    bridge_error: String,
) -> Result<DegradedReporter> {
    let render_cfg = config_yaml_path
        .map(Config::load_or_default)
        .and_then(|c| c.render);

    let layout = match render_cfg.as_ref().and_then(|c| c.current_layout.clone()) {
        Some(layout) => layout,
        None => SpeakerLayout::preset("7.1.4")?,
    };

    let params = SpatialRendererParams::from_render_config(render_cfg.as_ref());
    let renderer = build_spatial_renderer(
        &params,
        layout,
        sample_rate,
        REPORTER_VBAP_DEFAULTS,
        REPORTER_PREFERRED_MODE,
        render_cfg.as_ref(),
    )?;

    let control = renderer.renderer_control();
    control.set_bridge_error(Some(bridge_error));
    // Mirror the real engine so About's config diagnostics stay meaningful.
    if let Some(path) = config_yaml_path {
        control.set_config_path(path.to_path_buf());
        control.set_config_status(Some(Config::load_status(path).as_str().to_string()));
    }

    let target = SocketAddrV4::from_str(&format!("{}:{}", osc.host, osc.port_out))?;
    let mut sender = OscSender::new(target)?;
    sender.attach_renderer_control(control);
    sender.start_listener(osc.port_in)?;

    Ok(DegradedReporter {
        _osc: sender,
        _renderer: renderer,
    })
}
