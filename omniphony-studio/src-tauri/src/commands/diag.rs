//! Diagnostics & metering controls: publication rates, the diag publication
//! toggle, and the speaker gain-table subscription handshake.
//!
//! Each command forwards a value to the renderer over OSC.

use crate::osc_listener::OscControlMsg;
use crate::{send_control, SharedState};
use tauri::State;

#[tauri::command]
pub fn control_metering_rate_hz(state: State<SharedState>, value: f32) {
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
pub fn control_diag_rate_hz(state: State<SharedState>, value: f32) {
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
pub fn control_diag_publication_enabled(state: State<SharedState>, enable: i32) {
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
pub fn subscribe_speaker_gaintable(
    state: State<SharedState>,
    have_version: i32,
    speaker_index: i32,
) {
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
pub fn unsubscribe_speaker_gaintable(state: State<SharedState>) {
    send_control(
        &state.osc_tx,
        OscControlMsg::SendNoArgs {
            address: "/omniphony/control/debug/speaker_gaintable/unsubscribe".to_string(),
        },
    );
}
