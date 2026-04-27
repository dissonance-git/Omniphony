use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::sync::Arc;

use audio_input::InputControl;
use audio_output::AudioControl;
use renderer::live_params::RendererControl;
use rosc::{OscMessage, OscType};
use runtime_control::command::{RuntimeCommand, parse_process_command};
use runtime_control::context::RuntimeControlContext;
use runtime_control::osc::{
    BroadcastValue, ControlEffects, SpeakerPatch, apply_simple_osc_control,
    apply_speaker_osc_control,
};

use super::client_registry::OscClientRegistry;
use super::export::{build_live_state_bundle, export_current_layout, save_live_config};
use super::recompute::trigger_layout_recompute;
use super::transport::{
    broadcast_fff, broadcast_float, broadcast_int, broadcast_speaker_config, broadcast_string,
    resolve_register_addr, send_metering_state,
};

#[derive(Default)]
pub(crate) struct RealtimeSeqState {
    pub master_gain: Option<i32>,
    pub speaker_gain: HashMap<usize, i32>,
    pub object_gain: HashMap<String, i32>,
}

pub(crate) fn handle_control_message(
    msg: &OscMessage,
    src: SocketAddr,
    control: &Arc<RendererControl>,
    audio_control: Option<&Arc<AudioControl>>,
    input_control: Option<&Arc<InputControl>>,
    pending_speakers: &mut HashMap<usize, SpeakerPatch>,
    realtime_seq: &mut RealtimeSeqState,
    socket: &Arc<UdpSocket>,
    clients: &Arc<OscClientRegistry>,
) {
    let addr = msg.addr.as_str();
    let runtime_ctx = RuntimeControlContext::new(
        Arc::clone(control),
        audio_control.cloned(),
        input_control.cloned(),
    );

    if addr == "/omniphony/control/metering" {
        let enabled = match msg.args.first() {
            Some(OscType::Int(i)) => *i != 0,
            Some(OscType::Float(f)) => *f != 0.0,
            _ => return,
        };
        let client = resolve_register_addr(src, &[]);
        if clients.set_metering(client, enabled) {
            send_metering_state(socket, client, enabled);
        }
        return;
    }

    if addr == "/omniphony/control/input/refresh" {
        let state_bytes = build_live_state_bundle(control, audio_control, input_control);
        super::transport::send_raw(socket, clients, &state_bytes);
        log::info!("OSC: input state refresh requested");
        return;
    }

    if addr == "/omniphony/control/realtime/master_gain" {
        let Some(value) = msg.args.first().and_then(|arg| match arg {
            OscType::Float(v) => Some(*v),
            OscType::Int(v) => Some(*v as f32),
            _ => None,
        }) else {
            return;
        };
        let Some(seq) = msg.args.get(1).and_then(|arg| match arg {
            OscType::Int(v) => Some(*v),
            _ => None,
        }) else {
            return;
        };
        if realtime_seq.master_gain.is_some_and(|last| seq < last) {
            return;
        }
        realtime_seq.master_gain = Some(seq);
        control.live.write().unwrap().master_gain = value;
        control.mark_dirty();
        broadcast_int(socket, clients, "/omniphony/state/config/saved", 0);
        if let Ok(bytes) = rosc::encoder::encode(&rosc::OscPacket::Message(rosc::OscMessage {
            addr: "/omniphony/state/realtime/master_gain".to_string(),
            args: vec![OscType::Float(value), OscType::Int(seq)],
        })) {
            super::transport::send_raw(socket, clients, &bytes);
        }
        return;
    }

    if addr == "/omniphony/control/realtime/speaker_gain" {
        let Some(idx) = msg.args.first().and_then(|arg| match arg {
            OscType::Int(v) if *v >= 0 => Some(*v as usize),
            OscType::Float(v) if *v >= 0.0 => Some(*v as usize),
            _ => None,
        }) else {
            return;
        };
        let Some(value) = msg.args.get(1).and_then(|arg| match arg {
            OscType::Float(v) => Some(*v),
            OscType::Int(v) => Some(*v as f32),
            _ => None,
        }) else {
            return;
        };
        let Some(seq) = msg.args.get(2).and_then(|arg| match arg {
            OscType::Int(v) => Some(*v),
            _ => None,
        }) else {
            return;
        };
        if realtime_seq
            .speaker_gain
            .get(&idx)
            .copied()
            .is_some_and(|last| seq < last)
        {
            return;
        }
        realtime_seq.speaker_gain.insert(idx, seq);
        control
            .live
            .write()
            .unwrap()
            .speakers
            .entry(idx)
            .or_default()
            .gain = value;
        control.mark_speaker_params_dirty();
        control.mark_dirty();
        broadcast_int(socket, clients, "/omniphony/state/config/saved", 0);
        if let Ok(bytes) = rosc::encoder::encode(&rosc::OscPacket::Message(rosc::OscMessage {
            addr: "/omniphony/state/realtime/speaker_gain".to_string(),
            args: vec![
                OscType::Int(idx as i32),
                OscType::Float(value),
                OscType::Int(seq),
            ],
        })) {
            super::transport::send_raw(socket, clients, &bytes);
        }
        return;
    }

    if addr == "/omniphony/control/realtime/object_gain" {
        let Some(id) = msg.args.first().and_then(|arg| match arg {
            OscType::String(v) => {
                let trimmed = v.trim();
                if trimmed.is_empty() {
                    None
                } else {
                    Some(trimmed.to_string())
                }
            }
            OscType::Int(v) if *v >= 0 => Some(v.to_string()),
            OscType::Float(v) if *v >= 0.0 => Some((*v as i32).to_string()),
            _ => None,
        }) else {
            return;
        };
        let Some(value) = msg.args.get(1).and_then(|arg| match arg {
            OscType::Float(v) => Some(*v),
            OscType::Int(v) => Some(*v as f32),
            _ => None,
        }) else {
            return;
        };
        let Some(seq) = msg.args.get(2).and_then(|arg| match arg {
            OscType::Int(v) => Some(*v),
            _ => None,
        }) else {
            return;
        };
        if realtime_seq
            .object_gain
            .get(&id)
            .copied()
            .is_some_and(|last| seq < last)
        {
            return;
        }
        let Ok(idx) = id.parse::<usize>() else {
            return;
        };
        let clamped = value.clamp(0.0, 2.0);
        realtime_seq.object_gain.insert(id.clone(), seq);
        control
            .live
            .write()
            .unwrap()
            .objects
            .entry(idx)
            .or_default()
            .gain = clamped;
        control.mark_object_params_dirty();
        control.mark_dirty();
        broadcast_int(socket, clients, "/omniphony/state/config/saved", 0);
        if let Ok(bytes) = rosc::encoder::encode(&rosc::OscPacket::Message(rosc::OscMessage {
            addr: "/omniphony/state/realtime/object_gain".to_string(),
            args: vec![
                OscType::String(id.clone()),
                OscType::Float(clamped),
                OscType::Int(seq),
            ],
        })) {
            super::transport::send_raw(socket, clients, &bytes);
        }
        broadcast_float(
            socket,
            clients,
            &format!("/omniphony/state/object/{idx}/gain"),
            clamped,
        );
        return;
    }

    if addr == "/omniphony/control/render/bridge_path" {
        let value = match msg.args.first() {
            Some(OscType::String(s)) => s.trim(),
            _ => return,
        };
        let next = if value.is_empty() {
            None
        } else {
            Some(std::path::PathBuf::from(value))
        };
        if control.bridge_path() != next {
            control.set_bridge_path(next.clone());
            control.mark_dirty();
            broadcast_int(socket, clients, "/omniphony/state/config/saved", 0);
            let state_value = next
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            broadcast_string(
                socket,
                clients,
                "/omniphony/state/render/bridge_path",
                &state_value,
            );
            log::info!(
                "OSC: render.bridge_path → {}",
                next.as_ref()
                    .map(|path| path.display().to_string())
                    .unwrap_or_else(|| "<auto>".to_string())
            );
        }
        return;
    }

    if let Some(command) = parse_process_command(msg) {
        match command {
            RuntimeCommand::SaveConfig => {
                save_live_config(control, audio_control, input_control, socket, clients)
            }
            RuntimeCommand::ReloadConfig => {
                log::info!("OSC reload_config requested");
                sys::shutdown::request_restart_from_config();
            }
            RuntimeCommand::Quit => {
                log::info!("OSC quit requested");
                sys::shutdown::request_shutdown();
            }
            RuntimeCommand::SetLogLevel(requested) => {
                sys::live_log::set_runtime_level(requested);
                broadcast_string(
                    socket,
                    clients,
                    "/omniphony/state/log_level",
                    sys::live_log::current_runtime_level_name(),
                );
                log::info!(
                    "OSC: log_level → {}",
                    sys::live_log::current_runtime_level_name()
                );
            }
        }
        return;
    }

    if let Some(effects) = apply_simple_osc_control(msg, &runtime_ctx) {
        apply_control_effects(
            effects,
            control,
            audio_control,
            input_control,
            socket,
            clients,
        );
        return;
    }

    if let Some(effects) = apply_speaker_osc_control(msg, &runtime_ctx, pending_speakers) {
        apply_control_effects(
            effects,
            control,
            audio_control,
            input_control,
            socket,
            clients,
        );
        return;
    }

    if addr == "/omniphony/control/layout/export" {
        let requested_name = match msg.args.first() {
            Some(OscType::String(s)) if !s.trim().is_empty() => Some(s.trim()),
            _ => None,
        };
        export_current_layout(control, requested_name);
        return;
    }
}

fn set_dirty(control: &Arc<RendererControl>, socket: &UdpSocket, clients: &OscClientRegistry) {
    control.mark_dirty();
    broadcast_int(socket, clients, "/omniphony/state/config/saved", 0);
}

fn apply_control_effects(
    effects: ControlEffects,
    control: &Arc<RendererControl>,
    audio_control: Option<&Arc<AudioControl>>,
    input_control: Option<&Arc<InputControl>>,
    socket: &Arc<UdpSocket>,
    clients: &Arc<OscClientRegistry>,
) {
    if effects.mark_dirty {
        set_dirty(control, socket, clients);
        let state_bytes = build_live_state_bundle(control, audio_control, input_control);
        super::transport::send_raw(socket, clients, &state_bytes);
    }
    if let Some(layout) = effects.speaker_layout_broadcast.as_ref() {
        broadcast_speaker_config(socket, clients, layout);
    }
    for update in effects.broadcasts {
        match update.value {
            BroadcastValue::Int(value) => broadcast_int(socket, clients, &update.addr, value),
            BroadcastValue::Float(value) => broadcast_float(socket, clients, &update.addr, value),
            BroadcastValue::Fff(a, b, c) => broadcast_fff(socket, clients, &update.addr, a, b, c),
            BroadcastValue::String(value) => {
                broadcast_string(socket, clients, &update.addr, &value)
            }
        }
    }
    if let Some(message) = effects.log_message {
        log::info!("{message}");
    }
    if effects.trigger_layout_recompute {
        trigger_layout_recompute(control, socket, clients);
    }
}
