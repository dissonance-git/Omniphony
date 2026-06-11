use anyhow::Result;
use rosc::{OscMessage, OscPacket};
use std::net::{SocketAddr, SocketAddrV4, UdpSocket};
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::{Arc, Mutex};
use std::thread::JoinHandle;
use std::time::Duration;

use renderer::live_params::RendererControl;
use runtime_control::HostControlHandler;

mod client_registry;
mod dispatch;
mod export;
mod gaintable;
mod metadata_emit;
mod recompute;
mod state_emit;
mod transport;

use self::client_registry::OscClientRegistry;
use self::dispatch::{RealtimeSeqState, handle_control_message};
use self::export::build_live_state_bundle;
use self::gaintable::GaintableCache;
use self::transport::{
    flush_pending_logs, resolve_register_addr, send_buffered_logs_to_client, send_metering_state,
    send_raw_filtered,
};

/// Timeout after which a registered client (one that must heartbeat) is considered dead.
const CLIENT_TIMEOUT: Duration = Duration::from_secs(15);

/// How long a starting instance waits for a yieldable holder of the OSC RX
/// port to shut down after `/omniphony/control/yield_port`. Generous on
/// purpose: the standby flushes its audio output and joins its threads before
/// the port is released.
const YIELD_REBIND_BUDGET: Duration = Duration::from_secs(5);

/// Poll interval while waiting for the RX port to free up after a yield request.
const YIELD_REBIND_POLL: Duration = Duration::from_millis(50);

/// Fire-and-forget `/omniphony/control/yield_port` to the local holder of `rx_port`.
fn send_yield_request(rx_port: u16) {
    let Ok(socket) = UdpSocket::bind("127.0.0.1:0") else {
        return;
    };
    let msg = OscMessage {
        addr: runtime_control::osc_contract::CONTROL_YIELD_PORT.to_string(),
        args: vec![],
    };
    if let Ok(bytes) = rosc::encoder::encode(&OscPacket::Message(msg)) {
        let _ = socket.send_to(&bytes, ("127.0.0.1", rx_port));
    }
}

/// Reservation socket from [`negotiate_rx_port`]: keeps the RX port HELD
/// between the pre-flight negotiation and the real listener bind. Without it
/// the port would sit free during the whole engine build (bridge load, table
/// generation — seconds), long enough for Studio's auto-start watchdog to
/// probe it, spawn a fresh standby, and have that standby steal the
/// live-state sidecar meant for this instance.
static PORT_RESERVATION: Mutex<Option<(u16, UdpSocket)>> = Mutex::new(None);

/// Bind the OSC RX socket. On `AddrInUse` with `request_yield`, ask the local
/// holder to yield (honoured only by `--osc-yield` instances) and poll for the
/// port to free up within `budget`. The non-conflict path is a single bind.
/// Releases this process's own port reservation first.
fn bind_rx_socket(
    rx_port: u16,
    request_yield: bool,
    budget: Duration,
) -> std::io::Result<UdpSocket> {
    {
        let mut reservation = PORT_RESERVATION.lock().unwrap();
        if reservation
            .as_ref()
            .is_some_and(|(port, _)| *port == rx_port)
        {
            *reservation = None;
        }
    }
    let first_err = match UdpSocket::bind(("0.0.0.0", rx_port)) {
        Ok(socket) => return Ok(socket),
        Err(e) if e.kind() == std::io::ErrorKind::AddrInUse && request_yield => e,
        Err(e) => return Err(e),
    };
    log::info!(
        "OSC RX port {} is busy; asking the holder to yield",
        rx_port
    );
    send_yield_request(rx_port);
    let start = std::time::Instant::now();
    let mut resent = false;
    loop {
        std::thread::sleep(YIELD_REBIND_POLL);
        match UdpSocket::bind(("0.0.0.0", rx_port)) {
            Ok(socket) => {
                log::info!("OSC RX port {} acquired after yield", rx_port);
                return Ok(socket);
            }
            Err(e) if e.kind() == std::io::ErrorKind::AddrInUse => {
                if start.elapsed() >= budget {
                    return Err(first_err);
                }
                // One UDP retry in case the first request was lost.
                if !resent && start.elapsed() >= budget / 2 {
                    resent = true;
                    send_yield_request(rx_port);
                }
            }
            Err(e) => return Err(e),
        }
    }
}

/// Pre-flight port negotiation for hosts that must settle ownership of the RX
/// port *before* loading config (the FFI host consumes the live-state sidecar
/// a yielded instance writes on shutdown). On success the bound socket is kept
/// as a process-wide reservation, released when the real listener (or the
/// degraded reporter) binds via [`bind_rx_socket`] — so the port is never
/// observably free between negotiation and the listener coming up.
pub fn negotiate_rx_port(rx_port: u16) -> bool {
    match bind_rx_socket(rx_port, true, YIELD_REBIND_BUDGET) {
        Ok(socket) => {
            *PORT_RESERVATION.lock().unwrap() = Some((rx_port, socket));
            true
        }
        Err(_) => false,
    }
}

/// Generic description of a single spatial audio object for OSC broadcast.
/// Built by the caller from whatever source format it uses.
pub struct ObjectMeta {
    pub name: String,
    pub x: f32,
    pub y: f32,
    pub z: f32,
    pub coord_mode: String,
    pub direct_speaker_index: Option<u32>,
    /// Gain in dB (integer, -128 = silent).
    pub gain: i32,
    pub priority: f32,
    /// Per-axis object spatial extent (w, d, h), each in [0.0, 1.0].
    /// `[0.0, 0.0, 0.0]` denotes a point source.
    pub size: [f32; 3],
}

/// Epsilon for position/float comparison in delta OSC sending.
const OBJECT_EPSILON: f32 = 1e-6;

/// Snapshot of an object's comparable fields for delta detection.
#[derive(Clone)]
struct ObjectSnapshot {
    name: String,
    x: f32,
    y: f32,
    z: f32,
    coord_mode: String,
    direct_speaker_index: Option<u32>,
    gain: i32,
    priority: f32,
    size: [f32; 3],
}

impl ObjectSnapshot {
    fn from_meta(o: &ObjectMeta) -> Self {
        Self {
            name: o.name.clone(),
            x: o.x,
            y: o.y,
            z: o.z,
            coord_mode: o.coord_mode.clone(),
            direct_speaker_index: o.direct_speaker_index,
            gain: o.gain,
            priority: o.priority,
            size: o.size,
        }
    }

    fn matches_position(&self, o: &ObjectMeta) -> bool {
        self.name == o.name
            && self.gain == o.gain
            && self.coord_mode == o.coord_mode
            && self.direct_speaker_index == o.direct_speaker_index
            && (self.x - o.x).abs() < OBJECT_EPSILON
            && (self.y - o.y).abs() < OBJECT_EPSILON
            && (self.z - o.z).abs() < OBJECT_EPSILON
            && (self.priority - o.priority).abs() < OBJECT_EPSILON
    }

    fn matches_size(&self, o: &ObjectMeta) -> bool {
        (self.size[0] - o.size[0]).abs() < OBJECT_EPSILON
            && (self.size[1] - o.size[1]).abs() < OBJECT_EPSILON
            && (self.size[2] - o.size[2]).abs() < OBJECT_EPSILON
    }
}

pub struct OscSender {
    socket: Arc<UdpSocket>,
    /// Maps client address → last heartbeat time.
    /// `None`       = permanent client (the fixed `--osc-host` target), never times out.
    /// `Some(t)`    = registered via `/omniphony/register`, must send `/omniphony/heartbeat`
    ///                every <CLIENT_TIMEOUT/2 seconds or it will be dropped.
    clients: Arc<OscClientRegistry>,
    /// Shared live parameters + pending VBAP swap.
    /// Set by `attach_renderer_control` before `start_listener` is called.
    control: Option<Arc<RendererControl>>,
    /// Optional host-owned control handler (audio output/input). Set by hosts
    /// that bring their own audio layer (the CLI's `host_audio::HostAudio`);
    /// unset for the embedded liborender host so the core stays audio-free.
    /// Receives /control/{audio,input}/* messages the core doesn't handle and
    /// contributes /state/audio + /state/input to the live-state bundle.
    host_handler: Option<Arc<dyn HostControlHandler>>,
    /// Previous frame's object snapshots for delta detection.
    prev_objects: Option<Vec<ObjectSnapshot>>,
    /// Force next send_object_frame call to emit all objects.
    force_full_next: Arc<AtomicBool>,
    /// Monotonic identifier for the current logical content generation.
    content_generation: u64,
    /// Stop flag for the background OSC listener thread.
    listener_stop: Arc<AtomicBool>,
    /// Join handle for the background OSC listener thread.
    listener_thread: Mutex<Option<JoinHandle<()>>>,
}

impl OscSender {
    pub fn new(default_target: SocketAddrV4) -> Result<Self> {
        let socket = UdpSocket::bind("0.0.0.0:0")?;
        let clients = Arc::new(OscClientRegistry::new(CLIENT_TIMEOUT));
        clients.insert_permanent(SocketAddr::V4(default_target));
        Ok(Self {
            socket: Arc::new(socket),
            clients,
            control: None,
            host_handler: None,
            prev_objects: None,
            force_full_next: Arc::new(AtomicBool::new(true)),
            content_generation: 0,
            listener_stop: Arc::new(AtomicBool::new(false)),
            listener_thread: Mutex::new(None),
        })
    }

    /// Attach the renderer control object so the OSC listener can read/write live params
    /// and trigger VBAP recomputes.  Must be called **before** `start_listener`.
    pub fn attach_renderer_control(&mut self, control: Arc<RendererControl>) {
        self.control = Some(control);
    }

    /// Attach a host control handler (audio output/input layer). Hosts that
    /// own audio (the CLI) register their `host_audio::HostAudio` here; the
    /// embedded liborender host registers nothing so the core stays audio-free.
    pub fn attach_host_handler(&mut self, handler: Arc<dyn HostControlHandler>) {
        self.host_handler = Some(handler);
    }

    /// Start the OSC registration listener on `rx_port`.
    ///
    /// Clients send `/omniphony/register [i listen_port?]` from their listening socket.
    /// If the optional `Int` arg is present it overrides the source port (useful when
    /// the client's send and receive ports differ).
    /// On registration the client immediately receives the current live-state bundle.
    ///
    /// `request_yield`: on a port conflict, ask the local holder to yield
    /// (honoured only by `--osc-yield` standby instances) and retry. If the
    /// port still can't be bound the engine keeps running without a listener
    /// (loud error, no audio regression) — a port squatter must never cost the
    /// listener spatial audio.
    pub fn start_listener(&mut self, rx_port: u16, request_yield: bool) -> Result<()> {
        let socket = Arc::clone(&self.socket);
        let clients = Arc::clone(&self.clients);
        let control = self.control.clone();
        let host_handler = self.host_handler.clone();
        let force_full_next = Arc::clone(&self.force_full_next);
        let stop = Arc::clone(&self.listener_stop);

        if let Some(handle) = self.listener_thread.lock().unwrap().take() {
            self.listener_stop.store(true, Ordering::Relaxed);
            let _ = handle.join();
            self.listener_stop.store(false, Ordering::Relaxed);
        }

        let rx_socket = match bind_rx_socket(rx_port, request_yield, YIELD_REBIND_BUDGET) {
            Ok(socket) => socket,
            Err(e) => {
                log::error!(
                    "OSC listener: failed to bind port {} ({}); running without OSC control",
                    rx_port,
                    e
                );
                return Ok(());
            }
        };
        let _ = rx_socket.set_read_timeout(Some(Duration::from_millis(200)));
        log::info!("OSC listener ready on port {}", rx_port);

        let handle = std::thread::Builder::new()
            .name("osc-listener".into())
            .spawn(move || {
                let mut realtime_seq = RealtimeSeqState::default();
                // Serialized gain table is cached here and shared with the
                // recompute threads this loop spawns, so it's re-serialized only
                // when the topology actually changes (not per push/heartbeat).
                let gaintable_cache = Arc::new(GaintableCache::new());
                let mut last_log_seq = sys::live_log::records_since(0)
                    .last()
                    .map(|record| record.seq)
                    .unwrap_or(0);
                let mut last_host_state_generation =
                    host_handler.as_ref().map(|h| h.state_generation());
                let mut last_live_state_generation =
                    control.as_ref().map(|c| c.live_state_generation());

                let mut buf = [0u8; 4096];
                loop {
                    if stop.load(Ordering::Relaxed) {
                        break;
                    }
                    flush_pending_logs(&socket, &clients, &mut last_log_seq);
                    if let Some(host) = host_handler.as_ref() {
                        let generation = host.state_generation();
                        if last_host_state_generation != Some(generation) {
                            last_host_state_generation = Some(generation);
                            if let Some(ref ctrl) = control {
                                let state_bytes = build_live_state_bundle(ctrl, Some(host));
                                send_raw_filtered(&socket, &clients, &state_bytes, |_| true);
                            }
                        }
                    }
                    // Re-broadcast when core live state changed asynchronously on the
                    // audio thread (e.g. auto-gain lowering the master gain). Coalesced
                    // to this loop's poll cadence (≤200 ms) so loud passages can't flood.
                    if let Some(ref ctrl) = control {
                        let generation = ctrl.live_state_generation();
                        if last_live_state_generation != Some(generation) {
                            last_live_state_generation = Some(generation);
                            let state_bytes = build_live_state_bundle(ctrl, host_handler.as_ref());
                            send_raw_filtered(&socket, &clients, &state_bytes, |_| true);
                        }
                        // One-shot clip notification carrying the offending speaker
                        // index (set on the audio thread on any detected clip,
                        // regardless of auto-gain). Coalesced to the poll cadence so a
                        // loud passage emits at most one per tick.
                        if let Some(speaker_idx) = ctrl.take_clip_pending() {
                            if let Ok(bytes) =
                                rosc::encoder::encode(&OscPacket::Message(OscMessage {
                                    addr: "/omniphony/state/clip".to_string(),
                                    args: vec![rosc::OscType::Int(speaker_idx as i32)],
                                }))
                            {
                                send_raw_filtered(&socket, &clients, &bytes, |_| true);
                            }
                        }
                    }
                    match rx_socket.recv_from(&mut buf) {
                        Ok((len, src)) => {
                            match rosc::decoder::decode_udp(&buf[..len]) {
                                Ok((_, OscPacket::Message(msg)))
                                    if msg.addr == "/omniphony/register" =>
                                {
                                    let client = resolve_register_addr(src, &msg.args);
                                    let (is_new, metering_enabled) = clients.register(client);
                                    if is_new {
                                        log::info!("OSC client registered: {}", client);
                                    }
                                    // A new/reconnected client needs a complete object snapshot.
                                    force_full_next.store(true, Ordering::Relaxed);
                                    // Send the current state bundle, including layout and speakers.
                                    if let Some(ref ctrl) = control {
                                        let state_bytes =
                                            build_live_state_bundle(ctrl, host_handler.as_ref());
                                        if let Err(e) = socket.send_to(&state_bytes, client) {
                                            log::warn!(
                                                "Failed to send live state to {}: {}",
                                                client,
                                                e
                                            );
                                        }
                                    }
                                    send_buffered_logs_to_client(&socket, client, 0);
                                    send_metering_state(&socket, client, metering_enabled);
                                }
                                Ok((_, OscPacket::Message(msg)))
                                    if msg.addr == "/omniphony/heartbeat" =>
                                {
                                    let client = resolve_register_addr(src, &msg.args);
                                    let is_known = clients.heartbeat(client);
                                    let reply_addr = if is_known {
                                        log::trace!("OSC heartbeat/ack → {}", client);
                                        "/omniphony/heartbeat/ack"
                                    } else {
                                        "/omniphony/heartbeat/unknown"
                                    };
                                    let reply = OscMessage {
                                        addr: reply_addr.to_string(),
                                        args: vec![],
                                    };
                                    match rosc::encoder::encode(&OscPacket::Message(reply)) {
                                        Ok(bytes) => {
                                            if let Err(e) = socket.send_to(&bytes, client) {
                                                log::warn!(
                                                    "Failed to send heartbeat reply to {}: {}",
                                                    client,
                                                    e
                                                );
                                            }
                                        }
                                        Err(e) => {
                                            log::warn!("Failed to encode heartbeat reply: {}", e)
                                        }
                                    }
                                }

                                // ── Live-parameter control messages ─────────────────────────────────
                                Ok((_, OscPacket::Message(msg)))
                                    if msg.addr.starts_with("/omniphony/control/") =>
                                {
                                    if let Some(ref ctrl) = control {
                                        handle_control_message(
                                            &msg,
                                            src,
                                            ctrl,
                                            host_handler.as_ref(),
                                            &mut realtime_seq,
                                            &socket,
                                            &clients,
                                            &gaintable_cache,
                                        );
                                    }
                                }

                                // Any other packet (incl. bundles) may be a
                                // head-tracking feed on a user-configured address
                                // (e.g. SensorsOSC `/android/rotationvector`).
                                Ok((_, packet)) => {
                                    if let Some(ref ctrl) = control {
                                        if apply_head_tracking_packet(&packet, ctrl) {
                                            maybe_broadcast_head_pose(ctrl, &socket, &clients);
                                        }
                                    }
                                }
                                Err(e) => {
                                    log::debug!("OSC decode error from {}: {}", src, e)
                                }
                            }
                        }
                        Err(e)
                            if matches!(
                                e.kind(),
                                std::io::ErrorKind::WouldBlock | std::io::ErrorKind::TimedOut
                            ) => {}
                        Err(e) => log::warn!("OSC recv error: {}", e),
                    }
                }
            })?;

        *self.listener_thread.lock().unwrap() = Some(handle);

        Ok(())
    }

    /// Send bytes to every live client.
    ///
    /// Clients with a timed entry (`Some(t)`) are dropped if `t.elapsed() >= CLIENT_TIMEOUT`.
    /// Permanent clients (`None`) are never dropped.
    fn send_to_all(&self, bytes: &[u8]) {
        send_raw_filtered(&self.socket, &self.clients, bytes, |_| true);
    }

    fn send_to_metering_clients(&self, bytes: &[u8]) {
        send_raw_filtered(&self.socket, &self.clients, bytes, |client| {
            client.metering_enabled
        });
    }

    pub(crate) fn send_to_diag_clients(&self, bytes: &[u8]) {
        send_raw_filtered(&self.socket, &self.clients, bytes, |client| {
            client.diag_enabled
        });
    }

    pub fn has_osc_clients(&self) -> bool {
        self.clients.is_any_live()
    }

    pub fn has_metering_clients(&self) -> bool {
        self.clients.is_any_metering_live()
    }

    /// Pre-enable (or disable) metering on the permanent default target so that
    /// `--osc-metering` / `render.osc_metering` makes meter bundles flow to the
    /// configured OSC host without requiring a runtime enable message.
    pub fn set_default_metering(&self, enabled: bool) {
        self.clients.set_metering_for_permanent(enabled);
    }

    pub fn has_diag_clients(&self) -> bool {
        self.clients.is_any_diag_live()
    }
}

impl Drop for OscSender {
    fn drop(&mut self) {
        // Graceful-shutdown handoff, done while the RX port is still held so a
        // successor polling for the port is guaranteed to see the sidecar by
        // the time the port frees up. Skipped on reload_config, whose contract
        // is "discard live state and re-read the config".
        let reloading = sys::ShutdownHandle::is_restart_from_config_requested();
        if !reloading {
            if let Some(control) = self.control.as_ref() {
                // Only unsaved changes are worth handing over; a clean state
                // would just make the successor flag a phantom "unsaved" diff.
                if control.config_dirty.load(Ordering::Relaxed) {
                    let config_path = control.config_path.lock().as_ref().cloned();
                    if let Some(path) = config_path {
                        let sidecar = renderer::config::live_sidecar_path(&path);
                        match runtime_control::persist::save_live_config_to_path(
                            control,
                            self.host_handler.as_deref(),
                            &path,
                            &sidecar,
                        ) {
                            Ok(()) => {
                                // A fresh sidecar invalidates any overlay this
                                // process consumed earlier (destroy→create
                                // cycles of the FFI host re-read it).
                                renderer::config::clear_live_overlay_cache();
                                log::info!("live state handed off to {}", sidecar.display());
                            }
                            Err(e) => {
                                log::warn!("failed to write live-state sidecar: {e}")
                            }
                        }
                    }
                }
            }
            // Goodbye broadcast: lets clients reconnect to the next instance
            // immediately instead of waiting out their heartbeat timeout.
            let goodbye = OscMessage {
                addr: runtime_control::osc_contract::STATE_SHUTDOWN.to_string(),
                args: vec![rosc::OscType::String("shutdown".to_string())],
            };
            if let Ok(bytes) = rosc::encoder::encode(&OscPacket::Message(goodbye)) {
                send_raw_filtered(&self.socket, &self.clients, &bytes, |_| true);
            }
        }

        self.listener_stop.store(true, Ordering::Relaxed);
        if let Some(handle) = self.listener_thread.lock().unwrap().take() {
            let _ = handle.join();
        }
    }
}

/// Numeric OSC args → `f32`, accepting the common scalar types a sensor app may
/// emit (float, double, int).
fn collect_f32(args: &[rosc::OscType]) -> Vec<f32> {
    args.iter()
        .filter_map(|a| match a {
            rosc::OscType::Float(f) => Some(*f),
            rosc::OscType::Double(d) => Some(*d as f32),
            rosc::OscType::Int(i) => Some(*i as f32),
            _ => None,
        })
        .collect()
}

/// Apply a head-tracking packet if its address matches the configured tracking
/// address. Recurses into bundles (sensor apps often batch readings). Reads the
/// config under a short read lock and only takes the write lock on a match.
/// Returns `true` if the pose was updated.
fn apply_head_tracking_packet(packet: &OscPacket, ctrl: &RendererControl) -> bool {
    match packet {
        OscPacket::Message(msg) => {
            let format = {
                let live = ctrl.live.read();
                if !live.binaural.tracking.matches(&msg.addr) {
                    return false;
                }
                live.binaural.tracking.format
            };
            let args = collect_f32(&msg.args);
            if let Some(raw) = format.parse(&args) {
                {
                    let mut live = ctrl.live.write();
                    let current = live.binaural.head_pose;
                    live.binaural.head_pose = live.binaural.tracking.ingest(raw, current);
                }
                // Re-broadcast the live state so connected clients (Studio) see the
                // moving pose. Throttled to ~10 Hz so a 60–100 Hz sensor stream
                // doesn't resend the full state bundle on every packet. (The 3D
                // head view rides the lighter 30 Hz `/state/head_pose` channel —
                // see `maybe_broadcast_head_pose`.)
                static LAST_BUMP_MS: std::sync::atomic::AtomicU64 =
                    std::sync::atomic::AtomicU64::new(0);
                let now_ms = std::time::SystemTime::now()
                    .duration_since(std::time::UNIX_EPOCH)
                    .map(|d| d.as_millis() as u64)
                    .unwrap_or(0);
                let last = LAST_BUMP_MS.load(std::sync::atomic::Ordering::Relaxed);
                if now_ms.saturating_sub(last) >= 100 {
                    LAST_BUMP_MS.store(now_ms, std::sync::atomic::Ordering::Relaxed);
                    ctrl.bump_live_state();
                }
                return true;
            }
            false
        }
        OscPacket::Bundle(bundle) => {
            let mut updated = false;
            for inner in &bundle.content {
                updated |= apply_head_tracking_packet(inner, ctrl);
            }
            updated
        }
    }
}

/// Lightweight head-pose channel: a 4-float `/omniphony/state/head_pose`
/// message at ~30 Hz, so the Studio 3D head can follow tracking with low
/// latency without re-sending the full state JSON (which stays at 10 Hz for
/// the text readout).
fn maybe_broadcast_head_pose(
    ctrl: &RendererControl,
    socket: &std::net::UdpSocket,
    clients: &OscClientRegistry,
) {
    static LAST_POSE_MS: std::sync::atomic::AtomicU64 = std::sync::atomic::AtomicU64::new(0);
    let now_ms = std::time::SystemTime::now()
        .duration_since(std::time::UNIX_EPOCH)
        .map(|d| d.as_millis() as u64)
        .unwrap_or(0);
    let last = LAST_POSE_MS.load(std::sync::atomic::Ordering::Relaxed);
    if now_ms.saturating_sub(last) < 33 {
        return;
    }
    LAST_POSE_MS.store(now_ms, std::sync::atomic::Ordering::Relaxed);
    let pose = ctrl.live.read().binaural.head_pose;
    transport::broadcast_ffff(
        socket,
        clients,
        "/omniphony/state/head_pose",
        pose.w as f32,
        pose.x as f32,
        pose.y as f32,
        pose.z as f32,
    );
}

#[cfg(test)]
mod yield_tests {
    use super::*;

    /// Grab a free UDP port by binding port 0, then release it.
    fn free_port() -> u16 {
        let s = UdpSocket::bind("127.0.0.1:0").unwrap();
        s.local_addr().unwrap().port()
    }

    #[test]
    fn bind_succeeds_on_free_port() {
        let port = free_port();
        let socket = bind_rx_socket(port, true, Duration::from_millis(200)).expect("free port");
        assert_eq!(socket.local_addr().unwrap().port(), port);
    }

    #[test]
    fn bind_fails_after_budget_when_holder_keeps_port_and_yield_was_sent() {
        let port = free_port();
        let holder = UdpSocket::bind(("0.0.0.0", port)).unwrap();
        holder
            .set_read_timeout(Some(Duration::from_millis(500)))
            .unwrap();

        let err = bind_rx_socket(port, true, Duration::from_millis(200))
            .expect_err("holder never releases the port");
        assert_eq!(err.kind(), std::io::ErrorKind::AddrInUse);

        // The holder must have received exactly the yield request.
        let mut buf = [0u8; 256];
        let (len, _) = holder.recv_from(&mut buf).expect("yield datagram");
        let (_, packet) = rosc::decoder::decode_udp(&buf[..len]).expect("valid OSC");
        match packet {
            OscPacket::Message(msg) => {
                assert_eq!(msg.addr, runtime_control::osc_contract::CONTROL_YIELD_PORT)
            }
            other => panic!("expected a message, got {other:?}"),
        }
    }

    #[test]
    fn negotiation_reservation_holds_the_port_until_the_listener_binds() {
        let port = free_port();
        assert!(negotiate_rx_port(port), "free port must negotiate");
        // The reservation keeps the port held: an external bind must fail …
        assert_eq!(
            UdpSocket::bind(("0.0.0.0", port)).unwrap_err().kind(),
            std::io::ErrorKind::AddrInUse
        );
        // … but this process's own listener bind releases it and succeeds.
        let socket = bind_rx_socket(port, false, Duration::from_millis(100))
            .expect("listener bind must reuse the reserved port");
        assert_eq!(socket.local_addr().unwrap().port(), port);
    }

    #[test]
    fn bind_recovers_when_holder_yields() {
        let port = free_port();
        let holder = UdpSocket::bind(("0.0.0.0", port)).unwrap();
        holder
            .set_read_timeout(Some(Duration::from_secs(5)))
            .unwrap();
        // Holder thread: release the port upon receiving the yield request.
        let t = std::thread::spawn(move || {
            let mut buf = [0u8; 256];
            let _ = holder.recv_from(&mut buf);
            drop(holder);
        });

        let socket =
            bind_rx_socket(port, true, Duration::from_secs(5)).expect("port freed after yield");
        assert_eq!(socket.local_addr().unwrap().port(), port);
        t.join().unwrap();
    }
}
