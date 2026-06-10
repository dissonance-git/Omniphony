//! Binaural (headphone) output controls: output-mode toggle, HRIR source, and
//! the SensorsOSC head-tracking settings (address, format, smoothing, invert,
//! recenter).
//!
//! Each command forwards a value to the renderer over OSC.

use crate::osc_listener::OscControlMsg;
use crate::{send_control, SharedState};
use tauri::State;

#[tauri::command]
pub fn control_output_mode(state: State<SharedState>, value: String) {
    let normalized = value.trim().to_ascii_lowercase();
    if !matches!(normalized.as_str(), "speaker" | "binaural") {
        return;
    }
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: "/omniphony/control/output_mode".to_string(),
            value: normalized,
        },
    );
}

#[tauri::command]
pub fn control_hrir_source(state: State<SharedState>, value: String) {
    // "synthetic" | "saf"/"kemar" | "sofa:<path>".
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: "/omniphony/control/binaural/hrir_source".to_string(),
            value: value.trim().to_string(),
        },
    );
}

#[tauri::command]
pub fn control_head_recenter(state: State<SharedState>) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendInt {
            address: "/omniphony/control/head/recenter".to_string(),
            value: 1,
        },
    );
}

#[tauri::command]
pub fn control_head_tracking_address(state: State<SharedState>, value: String) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: "/omniphony/control/head/tracking/address".to_string(),
            value: value.trim().to_string(),
        },
    );
}

#[tauri::command]
pub fn control_head_tracking_format(state: State<SharedState>, value: String) {
    let normalized = value.trim().to_ascii_lowercase();
    if !matches!(normalized.as_str(), "auto" | "quat" | "rotvec" | "euler") {
        return;
    }
    send_control(
        &state.osc_tx,
        OscControlMsg::SendString {
            address: "/omniphony/control/head/tracking/format".to_string(),
            value: normalized,
        },
    );
}

#[tauri::command]
pub fn control_head_tracking_smoothing(state: State<SharedState>, value: f32) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendFloat {
            address: "/omniphony/control/head/tracking/smoothing".to_string(),
            value: value.clamp(0.0, 0.999),
        },
    );
}

#[tauri::command]
pub fn control_head_tracking_invert(state: State<SharedState>, enable: i32) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendInt {
            address: "/omniphony/control/head/tracking/invert".to_string(),
            value: if enable != 0 { 1 } else { 0 },
        },
    );
}
