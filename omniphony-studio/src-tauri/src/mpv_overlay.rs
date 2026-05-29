//! Bridge from Studio to a running mpv instance over its JSON IPC socket.
//!
//! Studio decimates audio object positions/levels at ~30 Hz and pushes them
//! to the `omniphony-overlay` lua script in mpv via `script-message`. The
//! lua script then draws the live X/Z + Y-color overlay on top of the
//! video.
//!
//! The socket path is whatever mpv was launched with
//! (`--input-ipc-server=…`):
//!  - Unix: a filesystem path to a Unix domain socket (e.g.
//!    `/tmp/omniphony-mpv.sock`).
//!  - Windows: a named pipe path (e.g. `\\.\pipe\omniphony-mpv`).
//! No normalisation — the user types the literal mpv was launched with,
//! same convention as the audio input pipe.

use std::io::{Read, Write};
use std::path::{Path, PathBuf};
use std::sync::{Arc, Mutex};
use std::time::{Duration, Instant};

#[cfg(unix)]
use std::os::unix::net::UnixStream;

const PREFS_FILENAME: &str = "mpv_overlay.json";

#[cfg(unix)]
pub const DEFAULT_SOCKET_PATH: &str = "/tmp/omniphony-mpv.sock";
#[cfg(windows)]
pub const DEFAULT_SOCKET_PATH: &str = r"\\.\pipe\omniphony-mpv";

/// Open the mpv IPC endpoint at `path` and return a (reader, writer) pair
/// of owned byte streams. On Unix the path is a Unix domain socket; on
/// Windows it's a named pipe (`\\.\pipe\<name>`) — `OpenOptions` opens it
/// in byte mode, and `try_clone()` duplicates the underlying HANDLE so
/// the reader and writer threads each own their half.
fn open_ipc(
    path: &str,
) -> std::io::Result<(Box<dyn Read + Send>, Box<dyn Write + Send>)> {
    #[cfg(unix)]
    {
        let stream = UnixStream::connect(path)?;
        let read = stream.try_clone()?;
        Ok((Box::new(read), Box::new(stream)))
    }
    #[cfg(windows)]
    {
        let file = std::fs::OpenOptions::new()
            .read(true)
            .write(true)
            .open(path)?;
        let read = file.try_clone()?;
        Ok((Box::new(read), Box::new(file)))
    }
    #[cfg(not(any(unix, windows)))]
    {
        let _ = path;
        Err(std::io::Error::new(
            std::io::ErrorKind::Unsupported,
            "mpv overlay IPC requires Unix sockets or Windows named pipes",
        ))
    }
}

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

enum WriterMsg {
    Send(String),
    Shutdown,
}

/// Holds the writer-thread channel. A `None` value means "not connected".
///
/// `reconnect_path` is the path of the last user-initiated connect. It is
/// kept across a transient connection loss (mpv restart) so the tick thread
/// can re-establish the link without bothering the user, and cleared only
/// on a user-initiated disconnect.
#[derive(Default)]
pub struct MpvOverlayState {
    inner: Mutex<Option<std::sync::mpsc::Sender<WriterMsg>>>,
    reconnect_path: Mutex<Option<String>>,
    last_reconnect_at: Mutex<Option<Instant>>,
    /// Last trail prefs pushed by JS. Re-sent on every successful
    /// (re)connect so a fresh mpv picks them up without needing JS to
    /// re-push.
    trail_prefs: Mutex<Option<TrailPrefs>>,
}

#[derive(Clone, Debug)]
pub struct TrailPrefs {
    pub enabled: bool,
    pub ttl_ms: u32,
    /// "diffuse" or "line"; anything else falls back to "line" lua-side.
    pub mode: String,
    /// Max XYZ displacement (normalised units) between two consecutive
    /// trail points before the connecting segment is treated as a
    /// teleport and skipped lua-side.
    pub teleport_threshold: f32,
}

impl MpvOverlayState {
    pub fn new() -> Self {
        Self::default()
    }

    /// Open the mpv IPC socket at `path`. Disconnects any prior session
    /// and stores the path so the tick thread can auto-reconnect if mpv
    /// later restarts.
    pub fn connect(&self, path: &str) -> Result<(), String> {
        *self.reconnect_path.lock().unwrap() = Some(path.to_string());
        self.connect_inner(path)
    }

    pub fn set_trail_prefs(&self, prefs: TrailPrefs) -> Result<(), String> {
        // Best-effort push to mpv. Ignore the error — if we're not
        // connected, the next successful (re)connect will resend the
        // stashed copy.
        let _ = self.send_trail_prefs_now(&prefs);
        *self.trail_prefs.lock().unwrap() = Some(prefs);
        Ok(())
    }

    fn send_trail_prefs_now(&self, prefs: &TrailPrefs) -> Result<(), String> {
        // Sanitise the mode to a known token to keep the lua parser strict.
        let mode = match prefs.mode.as_str() {
            "diffuse" => "diffuse",
            _ => "line",
        };
        // Clamp the threshold to the same range Studio uses, then format
        // with a few decimals — the lua parser is strict about the wire
        // format and won't accept scientific notation.
        let threshold = prefs.teleport_threshold.clamp(0.0, 4.0);
        let line = format!(
            r#"{{"command":["set_property","user-data/omniphony/overlay/trail-config","{}|{}|{}|{:.3}"]}}"#,
            if prefs.enabled { 1 } else { 0 },
            prefs.ttl_ms,
            mode,
            threshold
        );
        self.send_line(line)
    }

    fn connect_inner(&self, path: &str) -> Result<(), String> {
        self.drop_connection();

        let (mut read_handle, mut write_handle) =
            open_ipc(path).map_err(|e| format!("connect {path}: {e}"))?;
        // mpv writes a JSON response for every command we send. If we
        // don't read them, mpv's reply buffer fills (~25 s at 20 Hz) and
        // its main thread blocks trying to write the next reply — which
        // silently freezes the whole IPC. The dedicated reader thread
        // just drains.
        std::thread::Builder::new()
            .name("mpv-overlay-reader".into())
            .spawn(move || {
                let mut buf = [0u8; 4096];
                loop {
                    match read_handle.read(&mut buf) {
                        Ok(0) => break, // EOF — mpv closed the endpoint
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
                            if write_handle.write_all(line.as_bytes()).is_err()
                                || write_handle.write_all(b"\n").is_err()
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
        // Re-push the last trail prefs so a fresh mpv (or one whose
        // user-data was wiped on restart) lines up with what Studio
        // thinks they are.
        let prefs_snapshot = self.trail_prefs.lock().unwrap().clone();
        if let Some(prefs) = prefs_snapshot {
            let _ = self.send_trail_prefs_now(&prefs);
        }
        Ok(())
    }

    /// User-initiated disconnect: close the writer thread AND wipe the
    /// stored path so the auto-reconnect loop stops trying.
    pub fn disconnect(&self) {
        *self.reconnect_path.lock().unwrap() = None;
        self.drop_connection();
    }

    /// Close the writer thread but keep `reconnect_path` so the tick
    /// thread can re-establish the link transparently when the other end
    /// comes back.
    fn drop_connection(&self) {
        if let Some(tx) = self.inner.lock().unwrap().take() {
            let _ = tx.send(WriterMsg::Shutdown);
        }
    }

    /// Attempt to reconnect to the stored path. Called by the tick thread
    /// when the connection has been lost but the user hasn't disabled the
    /// overlay. Rate-limited internally to avoid hammering the socket.
    pub fn try_reconnect(&self) -> bool {
        if self.is_connected() {
            return false;
        }
        let path = match self.reconnect_path.lock().unwrap().clone() {
            Some(p) => p,
            None => return false,
        };
        let now = Instant::now();
        {
            let mut last = self.last_reconnect_at.lock().unwrap();
            if let Some(prev) = *last {
                if now.duration_since(prev) < Duration::from_secs(2) {
                    return false;
                }
            }
            *last = Some(now);
        }
        // `connect` would clobber `reconnect_path`; call the inner path
        // directly so a transient failure here does not erase it.
        self.connect_inner(&path).is_ok()
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
            // Keep `reconnect_path` so the tick thread can re-establish the
            // link when mpv comes back.
            self.drop_connection();
            return Err("writer thread gone".into());
        }
        Ok(())
    }

    pub fn is_connected(&self) -> bool {
        self.inner.lock().unwrap().is_some()
    }

    /// Send used by the OSC thread to push overlay frames. Pacing is up
    /// to the caller (driven by the renderer's metering rate) — no rate
    /// limit applied here.
    pub fn try_send_throttled(&self, line: String) -> bool {
        if !self.is_connected() {
            return false;
        }
        self.send_line(line).is_ok()
    }
}

pub type SharedOverlay = Arc<MpvOverlayState>;
