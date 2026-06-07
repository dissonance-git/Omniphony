// Prevents an additional console window on Windows in release builds.
#![cfg_attr(not(debug_assertions), windows_subsystem = "windows")]

mod app_state;
mod commands;
mod config;
mod layouts;
mod osc_listener;
mod osc_parser;

use std::path::PathBuf;
use std::sync::atomic::{AtomicI32, Ordering};
use std::sync::{Arc, Mutex};

use app_state::AppState;
use config::{load_config, save_config, OscConfig};
use layouts::Layout;
use osc_listener::{spawn_osc_task, OscControlMsg};
use rfd::FileDialog;
use tauri::{Manager, State};
use tokio::sync::mpsc::UnboundedSender;

// Tauri command handlers grouped into themed modules under `commands/`. Glob-
// imported so `generate_handler!` can keep referring to them by bare name.
use commands::audio::*;
use commands::input::*;
use commands::mpv_overlay::*;
use commands::orender::*;
use commands::render::*;
use commands::resampling::*;
use commands::speakers::*;

// ── shared state wrapper ──────────────────────────────────────────────────

pub(crate) struct SharedState {
    pub(crate) inner: Arc<Mutex<AppState>>,
    pub(crate) osc_tx: Arc<Mutex<Option<UnboundedSender<OscControlMsg>>>>,
    pub(crate) config_dir: PathBuf,
    pub(crate) listen_port: Arc<Mutex<u16>>,
    pub(crate) realtime_seq: AtomicI32,
    pub(crate) auto_tune_snapshot: Arc<Mutex<Option<serde_json::Value>>>,
}

// ── helper ────────────────────────────────────────────────────────────────

pub(crate) fn send_control(
    tx: &Arc<Mutex<Option<UnboundedSender<OscControlMsg>>>>,
    msg: OscControlMsg,
) {
    if let Some(tx) = tx.lock().unwrap().as_ref() {
        let _ = tx.send(msg);
    }
}

pub(crate) fn send_json_control(
    tx: &Arc<Mutex<Option<UnboundedSender<OscControlMsg>>>>,
    address: &str,
    payload: serde_json::Value,
) {
    let Ok(value) = serde_json::to_string(&payload) else {
        return;
    };
    send_control(
        tx,
        OscControlMsg::SendString {
            address: address.to_string(),
            value,
        },
    );
}

pub(crate) fn send_distance_metric(state: &State<SharedState>, address: &str, value: String) {
    let normalized = value.trim().to_ascii_lowercase();
    if !matches!(normalized.as_str(), "spherical" | "chebyshev") {
        return;
    }
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: address.to_string(),
            value: normalized,
        },
    );
}

#[derive(serde::Serialize)]
struct AboutInfo {
    name: &'static str,
    version: &'static str,
    license: &'static str,
    repository_url: &'static str,
    description: &'static str,
}

// ── Tauri commands ────────────────────────────────────────────────────────

#[tauri::command]
fn get_state(state: State<SharedState>) -> serde_json::Value {
    let s = state.inner.lock().unwrap();
    serde_json::to_value(&*s).unwrap_or(serde_json::Value::Null)
}

#[tauri::command]
fn get_osc_config(state: State<SharedState>) -> OscConfig {
    load_config(&state.config_dir)
}

#[tauri::command]
fn get_about_info() -> AboutInfo {
    AboutInfo {
        name: "Omniphony Studio",
        version: env!("CARGO_PKG_VERSION"),
        license: "GPL-3.0-only",
        repository_url: "https://github.com/mgth/Omniphony",
        description: "Omniphony is an open spatial-audio project built around realtime rendering, transport, control, and monitoring tools for object-based audio workflows. Omniphony Studio is the visual control surface of that ecosystem.",
    }
}

#[tauri::command]
fn save_osc_config(state: State<SharedState>, config: OscConfig) -> Result<(), String> {
    save_config(&state.config_dir, &config)?;
    state.inner.lock().unwrap().osc_metering_enabled =
        Some(if config.osc_metering_enabled { 1 } else { 0 });
    send_control(
        &state.osc_tx,
        OscControlMsg::SetMeteringEnabled {
            enabled: config.osc_metering_enabled,
        },
    );
    let listen_port = *state.listen_port.lock().unwrap();
    send_control(
        &state.osc_tx,
        OscControlMsg::Reconnect {
            host: config.host,
            rx_port: config.osc_rx_port,
            listen_port,
        },
    );
    Ok(())
}

#[tauri::command]
fn control_osc_metering(state: State<SharedState>, enable: i32) -> Result<(), String> {
    let enabled = enable != 0;
    let mut cfg = load_config(&state.config_dir);
    cfg.osc_metering_enabled = enabled;
    save_config(&state.config_dir, &cfg)?;
    state.inner.lock().unwrap().osc_metering_enabled = Some(if enabled { 1 } else { 0 });
    send_control(&state.osc_tx, OscControlMsg::SetMeteringEnabled { enabled });
    Ok(())
}

#[tauri::command]
fn select_layout(state: State<SharedState>, key: String) -> bool {
    let mut s = state.inner.lock().unwrap();
    let exists = s.layouts.iter().any(|l| l.key == key);
    if exists {
        s.selected_layout_key = Some(key);
    }
    exists
}

#[tauri::command]
fn import_layout_from_path(
    state: State<SharedState>,
    path: String,
) -> Result<serde_json::Value, String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("empty layout path".to_string());
    }
    let mut layout = layouts::load_layout_file(std::path::Path::new(trimmed))
        .ok_or_else(|| "failed to parse layout file".to_string())?;

    let mut s = state.inner.lock().unwrap();
    let base_key = layout.key.clone();
    let mut suffix = 1usize;
    while s.layouts.iter().any(|l| l.key == layout.key) {
        layout.key = format!("{base_key}-{}", suffix);
        suffix += 1;
    }
    s.selected_layout_key = Some(layout.key.clone());
    s.layouts.push(layout);
    s.layouts
        .sort_by(|a, b| a.name.to_lowercase().cmp(&b.name.to_lowercase()));

    Ok(serde_json::json!({
        "layouts": s.layouts,
        "selectedLayoutKey": s.selected_layout_key
    }))
}

#[tauri::command]
fn pick_import_layout_path() -> Option<String> {
    FileDialog::new()
        .add_filter("Layout", &["json", "yaml", "yml"])
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
fn pick_export_layout_path(suggested_name: Option<String>) -> Option<String> {
    let file_name = suggested_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            let lowered = s.to_ascii_lowercase();
            if lowered.ends_with(".yaml") || lowered.ends_with(".yml") || lowered.ends_with(".json")
            {
                s.to_string()
            } else {
                format!("{s}.yaml")
            }
        })
        .unwrap_or_else(|| "layout.yaml".to_string());

    FileDialog::new()
        .add_filter("Layout YAML", &["yaml", "yml"])
        .add_filter("Layout JSON", &["json"])
        .set_file_name(&file_name)
        .save_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
fn pick_import_evaluation_artifact_path() -> Option<String> {
    FileDialog::new()
        .add_filter("Omniphony evaluator", &["oevl"])
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
fn pick_export_evaluation_artifact_path(suggested_name: Option<String>) -> Option<String> {
    let file_name = suggested_name
        .as_deref()
        .map(str::trim)
        .filter(|s| !s.is_empty())
        .map(|s| {
            let lowered = s.to_ascii_lowercase();
            if lowered.ends_with(".oevl") {
                s.to_string()
            } else {
                format!("{s}.oevl")
            }
        })
        .unwrap_or_else(|| "evaluation.oevl".to_string());

    FileDialog::new()
        .add_filter("Omniphony evaluator", &["oevl"])
        .set_file_name(&file_name)
        .save_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
fn pick_bridge_path() -> Option<String> {
    FileDialog::new()
        .set_title("Select bridge library")
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
fn pick_orender_path() -> Option<String> {
    FileDialog::new()
        .set_title("Select orender executable")
        .pick_file()
        .map(|path| path.to_string_lossy().to_string())
}

#[tauri::command]
fn export_layout_to_path(path: String, layout: Layout) -> Result<(), String> {
    let trimmed = path.trim();
    if trimmed.is_empty() {
        return Err("empty export path".to_string());
    }

    layouts::save_layout_file(std::path::Path::new(trimmed), &layout)
}

#[tauri::command]
fn control_speaker_gain(state: State<SharedState>, id: i32, gain: f32) {
    let clamped = gain.max(0.0).min(2.0);
    let seq = state.realtime_seq.fetch_add(1, Ordering::Relaxed) + 1;
    send_control(
        &state.osc_tx,
        OscControlMsg::SendArgs {
            address: "/omniphony/control/realtime/speaker_gain".to_string(),
            args: vec![
                rosc::OscType::Int(id),
                rosc::OscType::Float(clamped),
                rosc::OscType::Int(seq),
            ],
        },
    );
}

#[tauri::command]
fn control_object_mute(state: State<SharedState>, id: i32, muted: i32) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendInt {
            address: format!("/omniphony/control/object/{id}/mute"),
            value: if muted != 0 { 1 } else { 0 },
        },
    );
}

#[tauri::command]
fn control_object_gain(state: State<SharedState>, id: String, gain: f32) {
    let clamped = gain.clamp(0.0, 2.0);
    let seq = state.realtime_seq.fetch_add(1, Ordering::Relaxed) + 1;
    send_control(
        &state.osc_tx,
        OscControlMsg::SendArgs {
            address: "/omniphony/control/realtime/object_gain".to_string(),
            args: vec![
                rosc::OscType::String(id),
                rosc::OscType::Float(clamped),
                rosc::OscType::Int(seq),
            ],
        },
    );
}

#[tauri::command]
fn control_speaker_mute(state: State<SharedState>, id: i32, muted: i32) {
    send_json_control(
        &state.osc_tx,
        "/omniphony/control/config/speakers",
        serde_json::json!({
            "speakerEdits": [{
                "id": id.max(0),
                "muted": muted != 0
            }]
        }),
    );
}

#[tauri::command]
fn control_master_gain(state: State<SharedState>, gain: f32) {
    let clamped = gain.max(0.0).min(2.0);
    let seq = state.realtime_seq.fetch_add(1, Ordering::Relaxed) + 1;
    send_control(
        &state.osc_tx,
        OscControlMsg::SendArgs {
            address: "/omniphony/control/realtime/master_gain".to_string(),
            args: vec![rosc::OscType::Float(clamped), rosc::OscType::Int(seq)],
        },
    );
}

#[tauri::command]
fn control_loudness(state: State<SharedState>, enable: i32) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendInt {
            address: "/omniphony/control/loudness".to_string(),
            value: if enable != 0 { 1 } else { 0 },
        },
    );
}

#[tauri::command]
fn control_auto_gain(state: State<SharedState>, enable: i32) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendInt {
            address: "/omniphony/control/auto_gain".to_string(),
            value: if enable != 0 { 1 } else { 0 },
        },
    );
}

#[tauri::command]
fn control_auto_gain_ceiling(state: State<SharedState>, db: f32) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendFloat {
            address: "/omniphony/control/auto_gain_ceiling".to_string(),
            value: db.clamp(-12.0, 0.0),
        },
    );
}

#[tauri::command]
fn auto_tune_snapshot_save(state: State<SharedState>, snapshot: serde_json::Value) {
    *state.auto_tune_snapshot.lock().unwrap() = Some(snapshot);
}

#[tauri::command]
fn auto_tune_snapshot_take(state: State<SharedState>) -> Option<serde_json::Value> {
    state.auto_tune_snapshot.lock().unwrap().take()
}

#[tauri::command]
fn auto_tune_snapshot_peek(state: State<SharedState>) -> Option<serde_json::Value> {
    state.auto_tune_snapshot.lock().unwrap().clone()
}

#[tauri::command]
fn control_metering_rate_hz(state: State<SharedState>, value: f32) {
    let clamped = value.max(1.0).min(1000.0);
    send_control(
        &state.osc_tx,
        OscControlMsg::SendFloat {
            address: "/omniphony/control/metering/rate_hz".to_string(),
            value: clamped,
        },
    );
}

#[tauri::command]
fn control_diag_rate_hz(state: State<SharedState>, value: f32) {
    let clamped = value.max(1.0).min(1000.0);
    send_control(
        &state.osc_tx,
        OscControlMsg::SendFloat {
            address: "/omniphony/control/diag/rate_hz".to_string(),
            value: clamped,
        },
    );
}

#[tauri::command]
fn control_diag_publication_enabled(state: State<SharedState>, enable: i32) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendInt {
            address: "/omniphony/control/diag/enabled".to_string(),
            value: if enable != 0 { 1 } else { 0 },
        },
    );
}

/// Subscribe to one speaker's per-band gain field. `have_version` is the version
/// already cached on this client (0 if none); `speaker_index` is the speaker to
/// display. The renderer pushes that speaker's field only if the version differs,
/// then on every topology rebuild while subscribed. Sent on first consumer, on
/// speaker change, and as a 5 s heartbeat (idempotent, self-healing).
#[tauri::command]
fn subscribe_speaker_gaintable(state: State<SharedState>, have_version: i32, speaker_index: i32) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendArgs {
            address: "/omniphony/control/debug/speaker_gaintable/subscribe".to_string(),
            args: vec![
                rosc::OscType::Int(have_version.max(0)),
                rosc::OscType::Int(speaker_index.max(0)),
            ],
        },
    );
}

/// Unsubscribe from the gain-table push stream (last consumer released). The
/// client keeps its cached table; a later re-subscribe negotiates by version.
#[tauri::command]
fn unsubscribe_speaker_gaintable(state: State<SharedState>) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendNoArgs {
            address: "/omniphony/control/debug/speaker_gaintable/unsubscribe".to_string(),
        },
    );
}

#[tauri::command]
fn control_distance_diffuse_enabled(state: State<SharedState>, enable: i32) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendInt {
            address: "/omniphony/control/distance_diffuse/enabled".to_string(),
            value: if enable != 0 { 1 } else { 0 },
        },
    );
}

#[tauri::command]
fn control_distance_diffuse_threshold(state: State<SharedState>, value: f32) {
    let v = value.max(0.01);
    send_control(
        &state.osc_tx,
        OscControlMsg::SendFloat {
            address: "/omniphony/control/distance_diffuse/threshold".to_string(),
            value: v,
        },
    );
}

#[tauri::command]
fn control_distance_diffuse_curve(state: State<SharedState>, value: f32) {
    let v = value.max(0.0);
    send_control(
        &state.osc_tx,
        OscControlMsg::SendFloat {
            address: "/omniphony/control/distance_diffuse/curve".to_string(),
            value: v,
        },
    );
}

#[tauri::command]
fn control_save_config(state: State<SharedState>) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendNoArgs {
            address: "/omniphony/control/save_config".to_string(),
        },
    );
}

#[tauri::command]
fn control_reload_config(state: State<SharedState>) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendNoArgs {
            address: "/omniphony/control/reload_config".to_string(),
        },
    );
}

#[tauri::command]
fn control_log_level(state: State<SharedState>, value: String) {
    let trimmed = value.trim().to_ascii_lowercase();
    if !matches!(
        trimmed.as_str(),
        "off" | "error" | "warn" | "info" | "debug" | "trace"
    ) {
        return;
    }
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: "/omniphony/control/log_level".to_string(),
            value: trimmed,
        },
    );
}

#[tauri::command]
fn control_ramp_mode(state: State<SharedState>, value: String) {
    let trimmed = value.trim().to_ascii_lowercase();
    if !matches!(trimmed.as_str(), "off" | "frame" | "sample") {
        return;
    }
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: "/omniphony/control/ramp_mode".to_string(),
            value: trimmed,
        },
    );
}

#[tauri::command]
fn control_drc_mode(state: State<SharedState>, value: String) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: "/omniphony/control/input/drc_mode".to_string(),
            value,
        },
    );
}

#[tauri::command]
fn control_drc_weight(state: State<SharedState>, value: f32) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendFloat {
            address: "/omniphony/control/input/drc_weight".to_string(),
            value: value.clamp(0.0, 1.0),
        },
    );
}

#[tauri::command]
fn control_export_layout(state: State<SharedState>, name: Option<String>) {
    if let Some(raw) = name {
        let trimmed = raw.trim();
        if !trimmed.is_empty() {
            send_control(
                &state.osc_tx,
                OscControlMsg::SendString {
                    address: "/omniphony/control/layout/export".to_string(),
                    value: trimmed.to_string(),
                },
            );
            return;
        }
    }
    send_control(
        &state.osc_tx,
        OscControlMsg::SendNoArgs {
            address: "/omniphony/control/layout/export".to_string(),
        },
    );
}

#[tauri::command]
fn control_render_bridge_path(state: State<SharedState>, value: String) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: "/omniphony/control/render/bridge_path".to_string(),
            value: value.trim().to_string(),
        },
    );
}

#[tauri::command]
fn control_render_input_pipe(state: State<SharedState>, value: String) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: "/omniphony/control/render/input_pipe".to_string(),
            value: value.trim().to_string(),
        },
    );
}

// ── main ─────────────────────────────────────────────────────────────────

fn main() {
    tauri::Builder::default()
        .setup(|app| {
            if let Some(window) = app.get_webview_window("main") {
                let decoded = image::load_from_memory(include_bytes!("../icons/icon.png"))
                    .expect("failed to decode window icon")
                    .into_rgba8();
                let (width, height) = decoded.dimensions();
                let window_icon = tauri::image::Image::new_owned(decoded.into_raw(), width, height);
                let _ = window.set_icon(window_icon);
            }

            let config_dir = app
                .path()
                .app_config_dir()
                .expect("could not resolve app config dir");

            let osc_cfg = load_config(&config_dir);

            // layouts dir: bundled as a resource
            let layouts_dir = app
                .path()
                .resource_dir()
                .map(|d| d.join("layouts"))
                .unwrap_or_else(|_| PathBuf::from("layouts"));

            let loaded_layouts = layouts::load_layouts(&layouts_dir);

            let mut initial_state = AppState::new(loaded_layouts);
            initial_state.osc_metering_enabled =
                Some(if osc_cfg.osc_metering_enabled { 1 } else { 0 });
            let app_state = Arc::new(Mutex::new(initial_state));
            let osc_tx: Arc<Mutex<Option<UnboundedSender<OscControlMsg>>>> =
                Arc::new(Mutex::new(None));
            let listen_port = Arc::new(Mutex::new(0u16));

            let (tx, rx) = tokio::sync::mpsc::unbounded_channel::<OscControlMsg>();
            *osc_tx.lock().unwrap() = Some(tx);

            let shared = SharedState {
                inner: app_state.clone(),
                osc_tx: osc_tx.clone(),
                config_dir,
                listen_port: listen_port.clone(),
                realtime_seq: AtomicI32::new(0),
                auto_tune_snapshot: Arc::new(Mutex::new(None)),
            };
            app.manage(shared);

            spawn_osc_task(
                app.handle().clone(),
                app_state,
                osc_cfg.host,
                osc_cfg.osc_port,
                osc_cfg.osc_rx_port,
                rx,
                listen_port.clone(),
            );

            Ok(())
        })
        .invoke_handler(tauri::generate_handler![
            get_state,
            get_osc_config,
            get_about_info,
            save_osc_config,
            launch_orender,
            stop_orender,
            get_orender_service_status,
            install_orender_service,
            uninstall_orender_service,
            start_orender_service,
            stop_orender_service,
            restart_orender_service,
            restart_pipewire_services,
            control_osc_metering,
            select_layout,
            import_layout_from_path,
            pick_import_layout_path,
            pick_export_layout_path,
            pick_import_evaluation_artifact_path,
            pick_export_evaluation_artifact_path,
            pick_bridge_path,
            pick_orender_path,
            export_layout_to_path,
            control_speaker_gain,
            control_object_mute,
            control_object_gain,
            control_speaker_mute,
            control_master_gain,
            control_loudness,
            control_auto_gain,
            control_auto_gain_ceiling,
            control_adaptive_resampling,
            control_adaptive_resampling_enable_far_mode,
            control_adaptive_resampling_force_silence_in_far_mode,
            control_adaptive_resampling_hard_recover_high_in_far_mode,
            control_adaptive_resampling_hard_recover_low_in_far_mode,
            control_adaptive_resampling_far_mode_return_fade_in_ms,
            control_latency_target,
            control_adaptive_resampling_kp_near,
            control_adaptive_resampling_ki,
            control_adaptive_resampling_integral_discharge_ratio,
            control_adaptive_resampling_max_adjust,
            control_adaptive_resampling_update_interval_callbacks,
            control_adaptive_resampling_high_recover_entry_margin_ms,
            control_adaptive_resampling_pause,
            control_adaptive_resampling_reset_ratio,
            control_metering_rate_hz,
            control_diag_rate_hz,
            control_diag_publication_enabled,
            control_spread_min,
            control_spread_max,
            control_spread_from_distance,
            control_spread_distance_range,
            control_spread_distance_curve,
            control_size_to_spread_mode,
            control_distance_model,
            control_distance_model_metric,
            control_distance_diffuse_metric,
            control_hybrid_external_backend,
            control_hybrid_internal_backend,
            control_hybrid_curve,
            control_hybrid_metric,
            control_hybrid_curve_smoothing,
            control_render_evaluation_cartesian_x_size,
            control_render_evaluation_cartesian_y_size,
            control_render_evaluation_cartesian_z_size,
            control_render_evaluation_cartesian_z_neg_size,
            control_render_backend,
            control_backend_param,
            control_restore_render_backend,
            control_render_evaluation_mode,
            control_render_evaluation_polar_azimuth_resolution,
            control_render_evaluation_polar_elevation_resolution,
            control_render_evaluation_polar_distance_res,
            control_render_evaluation_polar_distance_max,
            control_render_evaluation_position_interpolation,
            subscribe_speaker_gaintable,
            unsubscribe_speaker_gaintable,
            control_distance_diffuse_enabled,
            control_distance_diffuse_threshold,
            control_distance_diffuse_curve,
            control_room_ratio,
            control_room_ratio_rear,
            control_room_ratio_lower,
            control_room_ratio_center_blend,
            control_layout_radius_m,
            control_layout_config,
            control_layout_config_apply,
            control_speakers_config,
            control_speaker_az,
            control_speaker_el,
            control_speaker_distance,
            control_speaker_x,
            control_speaker_y,
            control_speaker_z,
            control_speaker_coord_mode,
            control_speaker_delay,
            control_speaker_spatialize,
            control_speaker_name,
            control_speaker_freq_low,
            control_speaker_freq_high,
            control_speakers_apply,
            control_speakers_add,
            control_speakers_remove,
            control_speakers_move,
            control_save_config,
            control_reload_config,
            control_log_level,
            control_ramp_mode,
            control_audio_config,
            control_audio_config_apply,
            control_audio_output_device,
            refresh_output_devices,
            control_input_config,
            control_input_config_apply,
            control_input_mode,
            control_input_live_backend,
            control_input_live_node,
            control_input_live_description,
            control_input_live_layout,
            import_input_layout_from_path,
            control_input_live_channels,
            control_input_live_sample_rate,
            control_input_live_format,
            control_input_live_clock_mode,
            control_input_live_map,
            control_input_live_lfe_mode,
            control_input_apply,
            control_render_bridge_path,
            control_render_input_pipe,
            control_export_layout,
            control_audio_sample_rate,
            control_drc_mode,
            control_drc_weight,
            auto_tune_snapshot_save,
            auto_tune_snapshot_take,
            auto_tune_snapshot_peek,
            mpv_overlay_set_trail_prefs,
            mpv_overlay_set_active,
            mpv_overlay_set_labels,
            mpv_overlay_set_objects,
            mpv_overlay_set_heatmap_enabled,
            mpv_overlay_set_heatmap_custom_stops,
            mpv_overlay_set_heatmap_bands,
            mpv_overlay_set_heatmap_colormap,
        ])
        .run(tauri::generate_context!())
        .expect("error running Tauri application");
}
