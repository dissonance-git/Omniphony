//! Engine-level controls: renderer config save/reload, log level, ramp mode,
//! dynamic-range-control (DRC) tuning and the layout-export trigger.
//!
//! Each command forwards a value to the renderer over OSC.

use crate::osc_listener::OscControlMsg;
use crate::{send_control, SharedState};
use tauri::State;

#[tauri::command]
pub fn control_save_config(state: State<SharedState>) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendNoArgs {
            address: "/omniphony/control/save_config".to_string(),
        },
    );
}

#[tauri::command]
pub fn control_reload_config(state: State<SharedState>) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendNoArgs {
            address: "/omniphony/control/reload_config".to_string(),
        },
    );
}

#[tauri::command]
pub fn control_log_level(state: State<SharedState>, value: String) {
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
pub fn control_ramp_mode(state: State<SharedState>, value: String) {
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
pub fn control_channel_render_mode(state: State<SharedState>, value: String) {
    let trimmed = value.trim().to_ascii_lowercase();
    // `direct`/`virtual` are legacy aliases of `spatial`.
    if !matches!(
        trimmed.as_str(),
        "host" | "spatial" | "direct" | "virtual"
    ) {
        return;
    }
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: "/omniphony/control/channel_render_mode".to_string(),
            value: trimmed,
        },
    );
}

/// Set the surround placement for 4.x/5.x channel content: `side` or `back`.
#[tauri::command]
pub fn control_surround_placement(state: State<SharedState>, value: String) {
    let trimmed = value.trim().to_ascii_lowercase();
    if !matches!(trimmed.as_str(), "side" | "back") {
        return;
    }
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: "/omniphony/control/surround_placement".to_string(),
            value: trimmed,
        },
    );
}

/// Select the bed→height object generator (2D upmix): `none`, `copy_up`, `pad`,
/// or any contributor id. The renderer resolves the id and falls back to off for
/// an unknown one, so the id set is not restricted here (out-of-tree generators).
#[tauri::command]
pub fn control_object_generator(state: State<SharedState>, value: String) {
    let trimmed = value.trim().to_ascii_lowercase();
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: "/omniphony/control/object_generator".to_string(),
            value: trimmed,
        },
    );
}

/// Set a live object-generator parameter (PAD: `strength` / `hpf_hz` /
/// `gain_db`). Sent as `[key, value]`; the renderer clamps and applies it live.
#[tauri::command]
pub fn control_object_generator_param(state: State<SharedState>, key: String, value: f32) {
    let k = key.trim().to_ascii_lowercase();
    // Any non-empty key is accepted; the renderer validates it against the active
    // generator's declared schema and clamps the value.
    if k.is_empty() || !value.is_finite() {
        return;
    }
    send_control(
        &state.osc_tx,
        OscControlMsg::SendArgs {
            address: "/omniphony/control/object_generator/param".to_string(),
            args: vec![rosc::OscType::String(k), rosc::OscType::Float(value)],
        },
    );
}

/// Set the output channel mapping: `by_index` (positionless — port N = layout
/// speaker N) or `by_name` (positional).
#[tauri::command]
pub fn control_output_channel_mapping(state: State<SharedState>, value: String) {
    let trimmed = value.trim().to_ascii_lowercase();
    if !matches!(trimmed.as_str(), "by_index" | "by_name") {
        return;
    }
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: "/omniphony/control/output_channel_mapping".to_string(),
            value: trimmed,
        },
    );
}

/// Set the parametrable virtual bed (a YAML `SpeakerLayout`, one entry per
/// channel label). An empty string resets to the built-in canonical poses.
#[tauri::command]
pub fn control_virtual_bed(state: State<SharedState>, value: String) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: "/omniphony/control/virtual_bed".to_string(),
            value,
        },
    );
}

#[tauri::command]
pub fn control_drc_mode(state: State<SharedState>, value: String) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: "/omniphony/control/input/drc_mode".to_string(),
            value,
        },
    );
}

#[tauri::command]
pub fn control_drc_weight(state: State<SharedState>, value: f32) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendFloat {
            address: "/omniphony/control/input/drc_weight".to_string(),
            value: value.clamp(0.0, 1.0),
        },
    );
}

#[tauri::command]
pub fn control_export_layout(state: State<SharedState>, name: Option<String>) {
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
