//! Bridge from Studio to a running mpv instance over its JSON IPC socket.
//!
//! Studio decimates audio object positions/levels at ~30 Hz and pushes them
//! to the `omniphony-overlay` lua script in mpv via `script-message`. The
//! lua script then draws the live X/Z + Y-color overlay on top of the
//! video.
//!
//! The socket path is whatever mpv was launched with
//! (`--input-ipc-server=…`). Studio remembers it in `localStorage`.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::net::UnixStream;

const PREFS_FILENAME: &str = "mpv_overlay.json";
pub const DEFAULT_SOCKET_PATH: &str = "/tmp/omniphony-mpv.sock";

#[derive(Clone, Debug, serde::Serialize, serde::Deserialize)]
pub struct OverlayPrefs {
    pub enabled: bool,
    pub socket_path: String,
}

impl Default for OverlayPrefs {
    fn default() -> Self {
        Self {
            enabled: false,
            socket_path: DEFAULT_SOCKET_PATH.to_string(),
        }
    }
}

fn prefs_path(config_dir: &Path) -> PathBuf {
    config_dir.join(PREFS_FILENAME)
}

pub fn load_prefs(config_dir: &Path) -> OverlayPrefs {
    let Ok(data) = std::fs::read_to_string(prefs_path(config_dir)) else {
        return OverlayPrefs::default();
    };
    serde_json::from_str(&data).unwrap_or_default()
}

pub fn save_prefs(config_dir: &Path, prefs: &OverlayPrefs) -> Result<(), String> {
    std::fs::create_dir_all(config_dir).map_err(|e| e.to_string())?;
    let data = serde_json::to_string_pretty(prefs).map_err(|e| e.to_string())?;
    std::fs::write(prefs_path(config_dir), data).map_err(|e| e.to_string())
}

/// Minimum gap between two pushed frames. Keeps IPC traffic tame when the
/// engine streams positions faster than the eye can follow. ~60 Hz cap.
const MIN_FRAME_GAP: Duration = Duration::from_millis(16);

enum WriterMsg {
    Send(String),
    Shutdown,
}

/// Holds the writer-thread channel. A `None` value means "not connected".
#[derive(Default)]
pub struct MpvOverlayState {
    inner: Mutex<Option<std::sync::mpsc::Sender<WriterMsg>>>,
    last_send: Mutex<Option<Instant>>,
}

impl MpvOverlayState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the mpv IPC socket at `path`. Disconnects any prior session.
    pub fn connect(&self, path: &str) -> Result<(), String> {
        self.disconnect();

        #[cfg(unix)]
        {
            let mut stream =
                UnixStream::connect(path).map_err(|e| format!("connect {path}: {e}"))?;
            // mpv writes a JSON response for every command we send. If we
            // don't read them, mpv's reply socket buffer fills (~25 s at
            // 20 Hz) and its main thread blocks trying to write the next
            // reply — which silently freezes the whole IPC. We share the
            // fd via try_clone and spawn a reader that just drains.
            let mut read_stream = stream
                .try_clone()
                .map_err(|e| format!("clone stream: {e}"))?;
            std::thread::Builder::new()
                .name("mpv-overlay-reader".into())
                .spawn(move || {
                    let mut buf = [0u8; 4096];
                    loop {
                        match read_stream.read(&mut buf) {
                            Ok(0) => break, // EOF — mpv closed the socket
                            Ok(_) => {}
                            Err(_) => break,
                        }
                    }
                })
                .map_err(|e| format!("spawn reader thread: {e}"))?;
            let (tx, rx) = std::sync::mpsc::channel::<WriterMsg>();
            std::thread::Builder::new()
                .name("mpv-overlay-writer".into())
                .spawn(move || {
                    for msg in rx {
                        match msg {
                            WriterMsg::Send(line) => {
                                if stream.write_all(line.as_bytes()).is_err()
                                    || stream.write_all(b"\n").is_err()
                                {
                                    break;
                                }
                            }
                            WriterMsg::Shutdown => break,
                        }
                    }
                })
                .map_err(|e| format!("spawn writer thread: {e}"))?;
            *self.inner.lock().unwrap() = Some(tx);
            Ok(())
        }

        #[cfg(not(unix))]
        {
            let _ = path;
            Err("mpv overlay IPC is only supported on Unix sockets right now".into())
        }
    }

    /// Close the writer thread. No-op if already disconnected.
    pub fn disconnect(&self) {
        if let Some(tx) = self.inner.lock().unwrap().take() {
            let _ = tx.send(WriterMsg::Shutdown);
        }
    }

    /// Push one already-serialized JSON IPC line. Drops it silently if not
    /// connected — overlay traffic is best-effort. If the writer thread is
    /// gone (mpv closed the socket), we tear down the connection state so
    /// the tick thread stops queuing into a dead channel.
    pub fn send_line(&self, line: String) -> Result<(), String> {
        let send_res = {
            let guard = self.inner.lock().unwrap();
            let Some(tx) = guard.as_ref() else {
                return Err("not connected".into());
            };
            tx.send(WriterMsg::Send(line))
        };
        if send_res.is_err() {
            // Writer is dead — drop our half so future calls short-circuit.
            self.disconnect();
            return Err("writer thread gone".into());
        }
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.inner.lock().unwrap().is_some()
    }

    /// Throttled send, used by the OSC thread to push overlay frames at
    /// ~60 Hz max. Returns `true` if the line was actually queued.
    pub fn try_send_throttled(&self, line: String) -> bool {
        if !self.is_connected() {
            return false;
        }
        let now = Instant::now();
        let mut last = self.last_send.lock().unwrap();
        if let Some(prev) = *last {
            if now.duration_since(prev) < MIN_FRAME_GAP {
                return false;
            }
        }
        *last = Some(now);
        drop(last);
        self.send_line(line).is_ok()
    }
}

pub type SharedOverlay = Arc<MpvOverlayState>;
