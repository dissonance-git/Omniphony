use std::collections::HashMap;
use std::net::SocketAddr;
use std::sync::Mutex;
use std::time::{Duration, Instant};

#[derive(Clone, Copy)]
pub(crate) struct OscClientState {
    pub(crate) last_seen: Option<Instant>,
    pub(crate) metering_enabled: bool,
    /// Whether this client wants `/omniphony/state/diag_*` updates. Decoupled
    /// from `metering_enabled` so a client can subscribe to diag traces
    /// without the audio-level meter bundle (and vice versa).
    pub(crate) diag_enabled: bool,
}

pub(crate) struct OscClientRegistry {
    clients: Mutex<HashMap<SocketAddr, OscClientState>>,
    timeout: Duration,
}

impl OscClientRegistry {
    pub(crate) fn new(timeout: Duration) -> Self {
        Self {
            clients: Mutex::new(HashMap::new()),
            timeout,
        }
    }

    pub(crate) fn insert_permanent(&self, addr: SocketAddr) {
        self.clients.lock().unwrap().insert(
            addr,
            OscClientState {
                last_seen: None,
                metering_enabled: false,
                diag_enabled: false,
            },
        );
    }

    pub(crate) fn register(&self, addr: SocketAddr) -> (bool, bool) {
        let mut clients = self.clients.lock().unwrap();
        let (metering_enabled, diag_enabled) = clients
            .get(&addr)
            .map(|entry| (entry.metering_enabled, entry.diag_enabled))
            .unwrap_or((false, false));
        let prev = clients.insert(
            addr,
            OscClientState {
                last_seen: Some(Instant::now()),
                metering_enabled,
                diag_enabled,
            },
        );
        (prev.is_none(), metering_enabled)
    }

    pub(crate) fn heartbeat(&self, addr: SocketAddr) -> bool {
        let mut clients = self.clients.lock().unwrap();
        if let Some(entry) = clients.get_mut(&addr) {
            if entry.last_seen.is_some() {
                entry.last_seen = Some(Instant::now());
            }
            true
        } else {
            false
        }
    }

    pub(crate) fn set_metering(&self, addr: SocketAddr, enabled: bool) -> bool {
        let mut clients = self.clients.lock().unwrap();
        if let Some(entry) = clients.get_mut(&addr) {
            entry.metering_enabled = enabled;
            true
        } else {
            false
        }
    }

    pub(crate) fn set_diag(&self, addr: SocketAddr, enabled: bool) -> bool {
        let mut clients = self.clients.lock().unwrap();
        if let Some(entry) = clients.get_mut(&addr) {
            entry.diag_enabled = enabled;
            true
        } else {
            false
        }
    }

    pub(crate) fn is_any_live(&self) -> bool {
        let clients = self.clients.lock().unwrap();
        let now = Instant::now();
        clients.values().any(|client| {
            client
                .last_seen
                .map(|t| now.duration_since(t) < self.timeout)
                .unwrap_or(true)
        })
    }

    pub(crate) fn is_any_metering_live(&self) -> bool {
        let clients = self.clients.lock().unwrap();
        let now = Instant::now();
        clients.values().any(|client| {
            client.metering_enabled
                && client
                    .last_seen
                    .map(|t| now.duration_since(t) < self.timeout)
                    .unwrap_or(true)
        })
    }

    pub(crate) fn is_any_diag_live(&self) -> bool {
        let clients = self.clients.lock().unwrap();
        let now = Instant::now();
        clients.values().any(|client| {
            client.diag_enabled
                && client
                    .last_seen
                    .map(|t| now.duration_since(t) < self.timeout)
                    .unwrap_or(true)
        })
    }

    pub(crate) fn send_filtered<F>(&self, socket: &std::net::UdpSocket, bytes: &[u8], predicate: F)
    where
        F: Fn(&OscClientState) -> bool,
    {
        let mut clients = self.clients.lock().unwrap();
        let now = Instant::now();
        clients.retain(|addr, client| match client.last_seen {
            None => true,
            Some(t) => {
                if now.duration_since(t) >= self.timeout {
                    log::info!("OSC client timed out, removing: {}", addr);
                    false
                } else {
                    true
                }
            }
        });
        for (addr, client) in clients.iter() {
            if predicate(client) {
                if let Err(e) = socket.send_to(bytes, *addr) {
                    log::warn!("OSC broadcast error to {}: {}", addr, e);
                }
            }
        }
    }
}
