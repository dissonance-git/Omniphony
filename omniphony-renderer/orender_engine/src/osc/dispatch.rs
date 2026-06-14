use std::collections::HashMap;
use std::net::{SocketAddr, UdpSocket};
use std::path::{Path, PathBuf};
use std::sync::Arc;

use renderer::backend_files;
use renderer::backend_params::ParamValue;
use renderer::live_params::RendererControl;
use rosc::{OscMessage, OscType};
use runtime_control::HostControlHandler;
use runtime_control::command::{RuntimeCommand, parse_process_command};
use runtime_control::context::RuntimeControlContext;
use runtime_control::osc::{
    BroadcastUpdate, BroadcastValue, ControlEffects, apply_simple_osc_control,
    gaintable_chunk_broadcasts,
};
use runtime_control::osc_contract;

use super::client_registry::OscClientRegistry;
use super::export::{build_live_state_bundle, export_current_layout, save_live_config};
use super::gaintable::GaintableCache;
use super::recompute::trigger_layout_recompute;
use super::transport::{
    broadcast_blob, broadcast_fff, broadcast_float, broadcast_int, broadcast_string,
    resolve_register_addr, send_diag_state, send_message_to_client, send_metering_state,
    send_update_to_client,
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
    host: Option<&Arc<dyn HostControlHandler>>,
    realtime_seq: &mut RealtimeSeqState,
    socket: &Arc<UdpSocket>,
    clients: &Arc<OscClientRegistry>,
    gaintable_cache: &Arc<GaintableCache>,
) {
    let addr = msg.addr.as_str();

    // Monitoring cadences live on RendererControl (the source of truth): both
    // CLI and embedded engine read them, they persist to config, and they are
    // broadcast in the live-state bundle. Changing them marks the config dirty.
    if addr == osc_contract::CONTROL_METERING_RATE_HZ {
        if let Some(hz) = first_rate_hz_arg(msg) {
            control.set_meter_rate_hz(hz);
            control.mark_dirty();
            broadcast_int(socket, clients, osc_contract::STATE_CONFIG_SAVED, 0);
            log::info!("OSC metering rate set to {:.1} Hz", control.meter_rate_hz());
        }
        return;
    }
    if addr == osc_contract::CONTROL_DIAG_RATE_HZ {
        if let Some(hz) = first_rate_hz_arg(msg) {
            control.set_diag_rate_hz(hz);
            control.mark_dirty();
            broadcast_int(socket, clients, osc_contract::STATE_CONFIG_SAVED, 0);
            log::info!("OSC diag rate set to {:.1} Hz", control.diag_rate_hz());
        }
        return;
    }

    // Channel render mode for non-object content (host / direct / virtual).
    // Live-tunable from Studio; persists to config so it survives a restart.
    if addr == osc_contract::CONTROL_CHANNEL_RENDER_MODE {
        if let Some(OscType::String(s)) = msg.args.first() {
            if let Some(mode) = renderer::live_params::ChannelRenderMode::from_str(s) {
                control.live.write().channel_render_mode = mode;
                control.mark_dirty();
                broadcast_int(socket, clients, osc_contract::STATE_CONFIG_SAVED, 0);
                log::info!("OSC channel render mode set to {}", mode.as_str());
            } else {
                log::warn!("OSC channel render mode: unknown value '{}'", s);
            }
        }
        return;
    }

    // mpv overlay configuration. The overlay itself is generated in-process by
    // the `overlay` module and pulled over FFI; Studio only configures it here
    // (it no longer transports overlay frames). These are transient view
    // preferences — not persisted, so no `mark_dirty`.
    if addr == osc_contract::CONTROL_OVERLAY_ENABLED {
        let enabled = match msg.args.first() {
            Some(OscType::Int(i)) => *i != 0,
            Some(OscType::Float(f)) => *f != 0.0,
            Some(OscType::Bool(b)) => *b,
            _ => return,
        };
        crate::overlay::set_enabled(enabled);
        return;
    }
    if addr == osc_contract::CONTROL_OVERLAY_LABELS {
        let enabled = match msg.args.first() {
            Some(OscType::Int(i)) => *i != 0,
            Some(OscType::Float(f)) => *f != 0.0,
            Some(OscType::Bool(b)) => *b,
            _ => return,
        };
        crate::overlay::set_labels_enabled(enabled);
        return;
    }
    if addr == osc_contract::CONTROL_OVERLAY_OBJECTS {
        let visible = match msg.args.first() {
            Some(OscType::Int(i)) => *i != 0,
            Some(OscType::Float(f)) => *f != 0.0,
            Some(OscType::Bool(b)) => *b,
            _ => return,
        };
        crate::overlay::set_objects_visible(visible);
        return;
    }
    if addr == osc_contract::CONTROL_OVERLAY_HEATMAP_ENABLED {
        let enabled = match msg.args.first() {
            Some(OscType::Int(i)) => *i != 0,
            Some(OscType::Float(f)) => *f != 0.0,
            Some(OscType::Bool(b)) => *b,
            _ => return,
        };
        crate::overlay::set_heatmap_enabled(enabled);
        return;
    }
    if addr == osc_contract::CONTROL_OVERLAY_HEATMAP_CUSTOM_STOPS {
        // Flat [pos, r, g, b, …] floats → grouped stops for the custom gradient.
        let flat: Vec<f32> = msg
            .args
            .iter()
            .filter_map(|a| match a {
                OscType::Float(f) => Some(*f),
                OscType::Int(i) => Some(*i as f32),
                _ => None,
            })
            .collect();
        let stops: Vec<[f32; 4]> = flat
            .chunks_exact(4)
            .map(|c| [c[0], c[1], c[2], c[3]])
            .collect();
        crate::overlay::set_heatmap_custom_stops(stops);
        return;
    }
    if addr == osc_contract::CONTROL_OVERLAY_HEATMAP_BANDS {
        let count = match msg.args.first() {
            Some(OscType::Int(i)) if *i > 0 => *i as usize,
            Some(OscType::Float(f)) if *f > 0.0 => *f as usize,
            _ => return,
        };
        crate::overlay::set_heatmap_bands(count);
        return;
    }
    if addr == osc_contract::CONTROL_OVERLAY_HEATMAP_COLORMAP {
        let idx = match msg.args.first() {
            Some(OscType::Int(i)) if *i >= 0 => *i as usize,
            Some(OscType::Float(f)) if *f >= 0.0 => *f as usize,
            _ => return,
        };
        crate::overlay::set_heatmap_colormap(idx);
        return;
    }
    if addr == osc_contract::CONTROL_OVERLAY_TRAILS {
        // Args mirror Studio's former wire fields: enabled, ttl_ms, mode, teleport.
        let enabled = match msg.args.first() {
            Some(OscType::Int(i)) => *i != 0,
            Some(OscType::Float(f)) => *f != 0.0,
            Some(OscType::Bool(b)) => *b,
            _ => return,
        };
        let ttl_ms = match msg.args.get(1) {
            Some(OscType::Int(i)) if *i >= 0 => *i as u32,
            Some(OscType::Float(f)) if *f >= 0.0 => *f as u32,
            _ => 7000,
        };
        let diffuse = matches!(
            msg.args.get(2),
            Some(OscType::String(s)) if s.eq_ignore_ascii_case("diffuse")
        );
        let teleport = match msg.args.get(3) {
            Some(OscType::Float(f)) => *f as f64,
            Some(OscType::Int(i)) => *i as f64,
            _ => 0.0,
        };
        crate::overlay::set_trail_config(enabled, ttl_ms, diffuse, teleport);
        return;
    }
    if addr == osc_contract::CONTROL_OVERLAY_TAG {
        // [id, tag]: tag "A"/"B" sets an override colour, anything else clears it.
        let Some(id) = msg.args.first().and_then(|a| match a {
            OscType::Int(v) if *v >= 0 => Some(*v as u32),
            OscType::Float(v) if *v >= 0.0 => Some(*v as u32),
            OscType::String(s) => s.parse::<u32>().ok(),
            _ => None,
        }) else {
            return;
        };
        let tag = match msg.args.get(1) {
            Some(OscType::String(s)) => s
                .chars()
                .next()
                .filter(|c| matches!(c, 'A' | 'a' | 'B' | 'b')),
            _ => None,
        };
        crate::overlay::set_tag(id, tag);
        return;
    }
    let runtime_ctx = RuntimeControlContext::new(Arc::clone(control));

    // Speaker gain-table pub/sub. A client subscribes for one speaker (the heatmap
    // shows one), carrying the version it has cached; the renderer pushes that
    // speaker's per-band field only if the version differs, and keeps pushing on
    // every topology rebuild while subscribed (see `recompute.rs`). Args:
    // [Int have_version, Int speaker_index].
    if addr == osc_contract::CONTROL_DEBUG_SPEAKER_GAINTABLE_SUBSCRIBE {
        let have_version = match msg.args.first() {
            Some(OscType::Int(i)) if *i >= 0 => Some(*i as u32),
            _ => None,
        };
        let speaker = match msg.args.get(1) {
            Some(OscType::Int(i)) if *i >= 0 => *i as usize,
            _ => 0,
        };
        let client = resolve_register_addr(src, &[]);
        // Ensure the client exists in the registry (refreshes liveness) so the
        // subscribe flag sticks and the 5 s heartbeat keeps it alive.
        clients.register(client);
        clients.set_gaintable(client, true);
        clients.set_gaintable_speaker(client, speaker);
        push_gaintable_subscribe(
            socket,
            clients,
            gaintable_cache,
            &runtime_ctx,
            client,
            speaker,
            have_version,
        );
        return;
    }

    if addr == osc_contract::CONTROL_DEBUG_SPEAKER_GAINTABLE_UNSUBSCRIBE {
        let client = resolve_register_addr(src, &[]);
        clients.set_gaintable(client, false);
        return;
    }

    if addr == osc_contract::CONTROL_DEBUG_SPEAKER_GAINTABLE_NACK {
        // Args: Int version, Int missing_index… — resend just the lost chunks for
        // the client's subscribed speaker.
        let mut ints = msg.args.iter().filter_map(|a| match a {
            OscType::Int(i) if *i >= 0 => Some(*i as u32),
            _ => None,
        });
        if let Some(version) = ints.next() {
            let missing: Vec<u32> = ints.collect();
            if !missing.is_empty() {
                let client = resolve_register_addr(src, &[]);
                let speaker = clients.gaintable_speaker(client).unwrap_or(0);
                if let Some((_v, bytes)) = gaintable_cache.bytes_for_speaker(&runtime_ctx, speaker)
                {
                    for update in gaintable_chunk_broadcasts(&bytes, Some((version, missing))) {
                        send_update_to_client(socket, client, &update);
                    }
                }
            }
        }
        return;
    }

    if addr == osc_contract::CONTROL_METERING {
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

    if addr == osc_contract::CONTROL_DIAG_ENABLED {
        let enabled = match msg.args.first() {
            Some(OscType::Int(i)) => *i != 0,
            Some(OscType::Float(f)) => *f != 0.0,
            Some(OscType::Bool(b)) => *b,
            _ => return,
        };
        let client = resolve_register_addr(src, &[]);
        if clients.set_diag(client, enabled) {
            send_diag_state(socket, client, enabled);
        }
        return;
    }

    if addr == osc_contract::CONTROL_INPUT_REFRESH {
        let state_bytes = build_live_state_bundle(control, host);
        super::transport::send_raw(socket, clients, &state_bytes);
        log::info!("OSC: input state refresh requested");
        return;
    }

    if addr == osc_contract::CONTROL_REALTIME_MASTER_GAIN {
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
        control.live.write().master_gain = value;
        control.mark_dirty();
        broadcast_int(socket, clients, osc_contract::STATE_CONFIG_SAVED, 0);
        if let Ok(bytes) = rosc::encoder::encode(&rosc::OscPacket::Message(rosc::OscMessage {
            addr: osc_contract::STATE_REALTIME_MASTER_GAIN.to_string(),
            args: vec![OscType::Float(value), OscType::Int(seq)],
        })) {
            super::transport::send_raw(socket, clients, &bytes);
        }
        return;
    }

    if addr == osc_contract::CONTROL_REALTIME_SPEAKER_GAIN {
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
        control.live.write().speakers.entry(idx).or_default().gain = value;
        control.mark_speaker_params_dirty();
        control.mark_dirty();
        broadcast_int(socket, clients, osc_contract::STATE_CONFIG_SAVED, 0);
        if let Ok(bytes) = rosc::encoder::encode(&rosc::OscPacket::Message(rosc::OscMessage {
            addr: osc_contract::STATE_REALTIME_SPEAKER_GAIN.to_string(),
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

    if addr == osc_contract::CONTROL_REALTIME_OBJECT_GAIN {
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
        control.live.write().objects.entry(idx).or_default().gain = clamped;
        control.mark_object_params_dirty();
        control.mark_dirty();
        broadcast_int(socket, clients, osc_contract::STATE_CONFIG_SAVED, 0);
        if let Ok(bytes) = rosc::encoder::encode(&rosc::OscPacket::Message(rosc::OscMessage {
            addr: osc_contract::STATE_REALTIME_OBJECT_GAIN.to_string(),
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

    if addr == osc_contract::CONTROL_RENDER_BRIDGE_PATH {
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
            broadcast_int(socket, clients, osc_contract::STATE_CONFIG_SAVED, 0);
            let state_value = next
                .as_ref()
                .map(|path| path.display().to_string())
                .unwrap_or_default();
            broadcast_string(
                socket,
                clients,
                osc_contract::STATE_RENDER_BRIDGE_PATH,
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

    if addr == osc_contract::CONTROL_RENDER_INPUT_PIPE {
        let value = match msg.args.first() {
            Some(OscType::String(s)) => s.trim(),
            _ => return,
        };
        let next = if value.is_empty() {
            None
        } else {
            Some(value.to_string())
        };
        if control.input_path() != next {
            control.set_input_path(next.clone());
            control.mark_dirty();
            broadcast_int(socket, clients, osc_contract::STATE_CONFIG_SAVED, 0);
            broadcast_string(
                socket,
                clients,
                osc_contract::STATE_INPUT_PIPE,
                &next.clone().unwrap_or_default(),
            );
            log::info!(
                "OSC: render.input_pipe → {}",
                next.as_deref().unwrap_or("<default>")
            );
        }
        return;
    }

    if let Some(command) = parse_process_command(msg) {
        match command {
            RuntimeCommand::SaveConfig => save_live_config(control, host, socket, clients),
            RuntimeCommand::ReloadConfig => {
                log::info!("OSC reload_config requested");
                sys::shutdown::request_restart_from_config();
            }
            RuntimeCommand::Quit => {
                log::info!("OSC quit requested");
                sys::shutdown::request_shutdown();
            }
            RuntimeCommand::YieldPort => {
                if sys::shutdown::is_yieldable() {
                    // Instead of shutting down, allocate a dynamic resume port,
                    // tell the requester (mpv) about it, and enter standby: the
                    // render loop releases the RX port + audio output and idles
                    // until a `resume` arrives on that port (mpv exit).
                    match crate::osc::prepare_standby_resume_port() {
                        Some(resume_port) => {
                            let reply = OscMessage {
                                addr: crate::osc::STANDBY_RESUME_REPLY.to_string(),
                                args: vec![OscType::Int(resume_port as i32)],
                            };
                            if let Ok(bytes) =
                                rosc::encoder::encode(&rosc::OscPacket::Message(reply))
                            {
                                let _ = socket.send_to(&bytes, src);
                            }
                            log::info!(
                                "OSC yield_port: entering standby; resume port {resume_port}"
                            );
                            sys::shutdown::request_standby();
                        }
                        None => {
                            log::warn!(
                                "OSC yield_port: could not allocate a resume port; shutting down"
                            );
                            sys::shutdown::request_shutdown();
                        }
                    }
                } else {
                    log::info!("OSC yield_port ignored (instance not yieldable)");
                }
            }
            RuntimeCommand::Resume => {
                log::info!("OSC resume requested");
                sys::shutdown::request_resume();
            }
            RuntimeCommand::SetLogLevel(requested) => {
                sys::live_log::set_runtime_level(requested);
                broadcast_string(
                    socket,
                    clients,
                    osc_contract::STATE_LOG_LEVEL,
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

    // Editable backend files (e.g. the scriptable backend's `.lua`). The content
    // is owned by the renderer, so the editor reads/writes it here over OSC; these
    // reply point-to-point to the requester (`src`) rather than broadcasting.
    if addr == osc_contract::CONTROL_BACKEND_FILE_GET {
        handle_backend_file_get(msg, src, control, socket);
        return;
    }
    if addr == osc_contract::CONTROL_BACKEND_FILE_LIST {
        handle_backend_file_list(msg, src, control, socket);
        return;
    }
    if addr == osc_contract::CONTROL_BACKEND_FILE_PUT {
        handle_backend_file_put(msg, src, control, host, socket, clients, gaintable_cache);
        return;
    }

    if let Some(effects) = apply_simple_osc_control(msg, &runtime_ctx) {
        apply_control_effects(effects, control, host, socket, clients, gaintable_cache);
        return;
    }

    // Core didn't handle it — delegate to the host (audio output/input).
    if let Some(effects) = host.and_then(|h| h.handle(addr, msg)) {
        apply_control_effects(effects, control, host, socket, clients, gaintable_cache);
        return;
    }

    if addr == osc_contract::CONTROL_LAYOUT_EXPORT {
        let requested_name = match msg.args.first() {
            Some(OscType::String(s)) if !s.trim().is_empty() => Some(s.trim()),
            _ => None,
        };
        export_current_layout(control, requested_name);
        return;
    }
}

fn first_rate_hz_arg(msg: &OscMessage) -> Option<f32> {
    msg.args.first().and_then(|arg| match arg {
        OscType::Float(v) => Some(*v),
        OscType::Int(v) => Some(*v as f32),
        _ => None,
    })
}

fn set_dirty(control: &Arc<RendererControl>, socket: &UdpSocket, clients: &OscClientRegistry) {
    control.mark_dirty();
    broadcast_int(socket, clients, osc_contract::STATE_CONFIG_SAVED, 0);
}

fn apply_control_effects(
    effects: ControlEffects,
    control: &Arc<RendererControl>,
    host: Option<&Arc<dyn HostControlHandler>>,
    socket: &Arc<UdpSocket>,
    clients: &Arc<OscClientRegistry>,
    gaintable_cache: &Arc<GaintableCache>,
) {
    if effects.mark_dirty {
        set_dirty(control, socket, clients);
        let state_bytes = build_live_state_bundle(control, host);
        super::transport::send_raw(socket, clients, &state_bytes);
    }
    for update in effects.broadcasts {
        match update.value {
            BroadcastValue::Int(value) => broadcast_int(socket, clients, &update.addr, value),
            BroadcastValue::Float(value) => broadcast_float(socket, clients, &update.addr, value),
            BroadcastValue::Fff(a, b, c) => broadcast_fff(socket, clients, &update.addr, a, b, c),
            BroadcastValue::String(value) => {
                broadcast_string(socket, clients, &update.addr, &value)
            }
            BroadcastValue::Blob(bytes) => broadcast_blob(socket, clients, &update.addr, &bytes),
        }
    }
    if let Some(message) = effects.log_message {
        log::info!("{message}");
    }
    if effects.trigger_layout_recompute {
        // A change that affects the backend geometry (triangulation / decorator
        // metrics) bumps the geometry generation so the upcoming recompute rebuilds
        // the gain models. Evaluation-only changes (mode / grid resolution) leave it
        // untouched, letting the recompute reuse the existing models and rebuild
        // only the evaluation wrapper. Bump BEFORE triggering so the plan captures
        // the new generation.
        if !effects.evaluation_only {
            control.bump_geometry_generation();
        }
        trigger_layout_recompute(control, socket, clients, gaintable_cache);
    }
}

/// Max bytes for an editable backend file carried in one OSC datagram. Scripts
/// are tiny, so a save/load stays a single all-or-nothing message (no chunk
/// reassembly), well under the UDP datagram limit.
const BACKEND_FILE_MAX_BYTES: usize = 60_000;

fn str_arg(msg: &OscMessage, index: usize) -> Option<String> {
    match msg.args.get(index) {
        Some(OscType::String(s)) => Some(s.clone()),
        _ => None,
    }
}

/// The directory holding the YAML config, used to root the managed file store.
fn backend_file_config_dir(control: &RendererControl) -> Option<PathBuf> {
    control
        .config_path()
        .and_then(|path| path.parent().map(|dir| dir.to_path_buf()))
}

fn send_backend_file_error(
    socket: &UdpSocket,
    src: SocketAddr,
    backend_id: &str,
    key: &str,
    message: impl Into<String>,
) {
    let message = message.into();
    log::warn!("backend file {backend_id}.{key}: {message}");
    send_message_to_client(
        socket,
        src,
        osc_contract::STATE_BACKEND_FILE_ERROR,
        vec![
            OscType::String(backend_id.to_string()),
            OscType::String(key.to_string()),
            OscType::String(message),
        ],
    );
}

/// `get [backend_id, key, name?]` → read a file's content on the renderer and
/// reply STATE_BACKEND_FILE_CONTENT to the requester. With an explicit `name` the
/// editor previews any managed-store file; without it, the param's current handle
/// is read. An absolute handle is only honoured for a loopback caller (see
/// [`backend_files::resolve`]).
fn handle_backend_file_get(
    msg: &OscMessage,
    src: SocketAddr,
    control: &Arc<RendererControl>,
    socket: &UdpSocket,
) {
    let (Some(backend_id), Some(key)) = (str_arg(msg, 0), str_arg(msg, 1)) else {
        return;
    };
    let handle = match str_arg(msg, 2) {
        Some(name) if !name.trim().is_empty() => name,
        _ => control
            .backend_params_for(&backend_id)
            .get(&key)
            .and_then(|value| value.as_str().map(str::to_string))
            .unwrap_or_default(),
    };
    let config_dir = backend_file_config_dir(control);
    let allow_absolute = src.ip().is_loopback();
    let Some(path) =
        backend_files::resolve(config_dir.as_deref(), &backend_id, &handle, allow_absolute)
    else {
        send_backend_file_error(socket, src, &backend_id, &key, "no file selected");
        return;
    };
    match std::fs::read_to_string(&path) {
        Ok(content) => send_message_to_client(
            socket,
            src,
            osc_contract::STATE_BACKEND_FILE_CONTENT,
            vec![
                OscType::String(backend_id),
                OscType::String(key),
                OscType::String(handle),
                OscType::String(content),
            ],
        ),
        Err(e) => {
            send_backend_file_error(socket, src, &backend_id, &key, format!("read failed: {e}"))
        }
    }
}

/// `list [backend_id]` → reply STATE_BACKEND_FILE_LIST with the managed store's
/// file names as a JSON array, so the editor can offer them when the renderer is
/// remote (no native Browse).
fn handle_backend_file_list(
    msg: &OscMessage,
    src: SocketAddr,
    control: &Arc<RendererControl>,
    socket: &UdpSocket,
) {
    let Some(backend_id) = str_arg(msg, 0) else {
        return;
    };
    let config_dir = backend_file_config_dir(control);
    let names = backend_files::list(config_dir.as_deref(), &backend_id);
    let json = serde_json::to_string(&names).unwrap_or_else(|_| "[]".to_string());
    send_message_to_client(
        socket,
        src,
        osc_contract::STATE_BACKEND_FILE_LIST,
        vec![OscType::String(backend_id), OscType::String(json)],
    );
}

/// `put [backend_id, key, name, content]` → write the content into the managed
/// store (or, for a loopback caller, an absolute path), persist the handle, and
/// rebuild the backend. Replies STATE_BACKEND_FILE_CONTENT as a save ack; build
/// errors surface through the usual recompute-error banner.
fn handle_backend_file_put(
    msg: &OscMessage,
    src: SocketAddr,
    control: &Arc<RendererControl>,
    host: Option<&Arc<dyn HostControlHandler>>,
    socket: &Arc<UdpSocket>,
    clients: &Arc<OscClientRegistry>,
    gaintable_cache: &Arc<GaintableCache>,
) {
    let (Some(backend_id), Some(key), Some(name)) =
        (str_arg(msg, 0), str_arg(msg, 1), str_arg(msg, 2))
    else {
        return;
    };
    let content = str_arg(msg, 3).unwrap_or_default();
    if content.len() > BACKEND_FILE_MAX_BYTES {
        send_backend_file_error(
            socket,
            src,
            &backend_id,
            &key,
            format!(
                "file too large ({} bytes, max {BACKEND_FILE_MAX_BYTES})",
                content.len()
            ),
        );
        return;
    }
    let config_dir = backend_file_config_dir(control);
    let allow_absolute = src.ip().is_loopback();
    let Some(path) =
        backend_files::resolve(config_dir.as_deref(), &backend_id, &name, allow_absolute)
    else {
        send_backend_file_error(socket, src, &backend_id, &key, "invalid file name");
        return;
    };
    // The handle we persist must resolve back to `path` at build time (which
    // always allows absolute paths): keep an allowed absolute name as-is,
    // otherwise the safe store basename.
    let stored_handle = if allow_absolute && Path::new(name.trim()).is_absolute() {
        name.trim().to_string()
    } else {
        match backend_files::sanitize_name(&name) {
            Some(basename) => basename,
            None => {
                send_backend_file_error(socket, src, &backend_id, &key, "invalid file name");
                return;
            }
        }
    };
    if let Some(parent) = path.parent() {
        if let Err(e) = std::fs::create_dir_all(parent) {
            send_backend_file_error(
                socket,
                src,
                &backend_id,
                &key,
                format!("cannot create store dir: {e}"),
            );
            return;
        }
    }
    if let Err(e) = std::fs::write(&path, content.as_bytes()) {
        send_backend_file_error(socket, src, &backend_id, &key, format!("write failed: {e}"));
        return;
    }
    control.set_backend_param(&backend_id, &key, ParamValue::Text(stored_handle.clone()));
    // Ack the save back to the editor.
    send_message_to_client(
        socket,
        src,
        osc_contract::STATE_BACKEND_FILE_CONTENT,
        vec![
            OscType::String(backend_id.clone()),
            OscType::String(key.clone()),
            OscType::String(stored_handle),
            OscType::String(content),
        ],
    );
    // Republish state and rebuild the backend with the new content; a bad script
    // surfaces via the recompute-error path like any other build failure.
    apply_control_effects(
        ControlEffects {
            mark_dirty: true,
            trigger_layout_recompute: true,
            log_message: Some(format!("OSC: backend file {backend_id}.{key} saved")),
            ..Default::default()
        },
        control,
        host,
        socket,
        clients,
        gaintable_cache,
    );
}

/// Reply to a gain-table subscribe: push the full chunked table if the client's
/// cached `have_version` is stale (or absent), ack `uptodate` if it already has
/// the current version, or `unavailable` if the active backend has no table.
fn push_gaintable_subscribe(
    socket: &UdpSocket,
    clients: &OscClientRegistry,
    gaintable_cache: &GaintableCache,
    ctx: &RuntimeControlContext,
    client: SocketAddr,
    speaker: usize,
    have_version: Option<u32>,
) {
    match gaintable_cache.bytes_for_speaker(ctx, speaker) {
        Some((version, bytes)) => {
            if have_version == Some(version) {
                send_update_to_client(
                    socket,
                    client,
                    &BroadcastUpdate {
                        addr: osc_contract::STATE_DEBUG_SPEAKER_GAINTABLE_UPTODATE.to_string(),
                        value: BroadcastValue::Int(version as i32),
                    },
                );
            } else {
                for update in gaintable_chunk_broadcasts(&bytes, None) {
                    send_update_to_client(socket, client, &update);
                }
                clients.set_gaintable_version(client, version);
            }
        }
        None => send_update_to_client(
            socket,
            client,
            &BroadcastUpdate {
                addr: osc_contract::STATE_DEBUG_SPEAKER_GAINTABLE_UNAVAILABLE.to_string(),
                value: BroadcastValue::String(
                    "{\"reason\":\"no precomputed gain table for the active backend\"}".to_string(),
                ),
            },
        ),
    }
}
