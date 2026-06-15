//! C ABI for the `orender` spatial audio renderer — built as `liborender.so`.
//!
//! A thin, panic-safe shim over [`orender_engine::Engine`]: the host (mpv via
//! `ad_orender.c`, or any C program) creates a session from a config, pushes
//! raw encoded packets, and receives interleaved multichannel `f32` PCM. No
//! audio output happens here — the host owns that.
//!
//! Every entry point catches Rust panics at the boundary (a panic crossing into
//! C is undefined behaviour) and the C caller owns all output buffers.

#![allow(clippy::missing_safety_doc)]

use orender_engine::{start_degraded_reporter, DegradedReporter, Engine, OscOptions};

use anyhow::Result;
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::panic::{catch_unwind, AssertUnwindSafe};
use std::path::{Path, PathBuf};
use std::ptr;
use std::sync::atomic::{AtomicBool, Ordering};
use std::sync::Mutex;

// Process-global decoder-less OSC reporter, brought up when the bridge can't be
// loaded so Studio can show a red banner (orender_create still returns NULL, so
// mpv falls back to its native decoder). One per process; lives until a real
// engine starts (which reclaims the OSC port) or the host exits.
static DEGRADED_REPORTER: Mutex<Option<DegradedReporter>> = Mutex::new(None);
static DEGRADED_ACTIVE: AtomicBool = AtomicBool::new(false);

// Bring up the degraded reporter once. The renderer build (VBAP table) takes a
// moment, so do it on a detached thread — the caller returns NULL immediately
// and mpv falls back without waiting.
fn start_degraded_reporter_global(
    config_path: Option<PathBuf>,
    sample_rate: u32,
    opts: OscOptions,
    message: String,
) {
    // Claim the single slot; bail if a reporter is already active/starting.
    if DEGRADED_ACTIVE
        .compare_exchange(false, true, Ordering::SeqCst, Ordering::SeqCst)
        .is_err()
    {
        return;
    }
    std::thread::spawn(move || {
        match start_degraded_reporter(config_path.as_deref(), sample_rate, opts, message) {
            Ok(reporter) => {
                // A real engine may have started while we were building; only
                // keep ours if the slot is still claimed (else drop → free port).
                if DEGRADED_ACTIVE.load(Ordering::SeqCst) {
                    *DEGRADED_REPORTER.lock().unwrap() = Some(reporter);
                }
            }
            Err(e) => {
                eprintln!("degraded reporter failed to start: {e:#}");
                DEGRADED_ACTIVE.store(false, Ordering::SeqCst);
            }
        }
    });
}

// Tear down the degraded reporter (releases its OSC port) when a real engine is
// coming up.
fn stop_degraded_reporter_global() {
    if DEGRADED_ACTIVE.swap(false, Ordering::SeqCst) {
        *DEGRADED_REPORTER.lock().unwrap() = None;
    }
}

// Resolve OSC options from the C override → config → defaults, or None when OSC
// is off. Shared by the normal path and the degraded reporter.
fn resolve_osc_opts(
    cfg: &OrenderConfig,
    render_cfg: Option<&orender_engine::RenderConfig>,
) -> Option<OscOptions> {
    let osc_on = cfg.osc_enabled != 0 || render_cfg.and_then(|c| c.osc).unwrap_or(false);
    if !osc_on {
        return None;
    }
    let host = unsafe { opt_str(cfg.osc_host) }
        .map(str::to_string)
        .or_else(|| render_cfg.and_then(|c| c.osc_host.clone()))
        .unwrap_or_else(|| "127.0.0.1".to_string());
    let port_out = if cfg.osc_port_out != 0 {
        cfg.osc_port_out
    } else {
        render_cfg.and_then(|c| c.osc_port).unwrap_or(9000)
    };
    let port_in = if cfg.osc_port_in != 0 {
        cfg.osc_port_in
    } else {
        render_cfg.and_then(|c| c.osc_rx_port).unwrap_or(9000)
    };
    Some(OscOptions {
        host,
        port_out,
        port_in,
    })
}

/// Opaque handle to a decode→render session. Created by [`orender_create`],
/// freed by [`orender_destroy`]. Internally a boxed [`Engine`].
#[repr(C)]
pub struct OrenderRenderer {
    _private: [u8; 0],
}

/// Session configuration passed to [`orender_create`]. All `*const c_char`
/// fields are UTF-8, nul-terminated, and may be NULL (treated as "unset").
#[repr(C)]
pub struct OrenderConfig {
    /// Output/host sample rate in Hz. 0 → 48000.
    pub sample_rate: u32,
    /// Path to the omniphony YAML config (drives bridge path, speaker layout +
    /// all render params). NULL → the shared default config used by the orender
    /// CLI + studio (`~/.config/omniphony/config.yaml`).
    pub config_yaml_path: *const c_char,
    /// Optional speaker-layout YAML path overriding the config. NULL → use the
    /// config's embedded layout, else the 7.1.4 preset.
    pub speaker_layout_path: *const c_char,
    /// Optional decoder bridge plugin path (the `*_bridge.so` produced by
    /// the input format's bridge crate) overriding the config. NULL → taken
    /// from the config YAML's `render.bridge_path` (the source of truth;
    /// library hosts have no exe-relative search).
    pub bridge_path: *const c_char,
    /// Codec identifier of the raw access units the host will feed (matches
    /// the bridge's supported codec IDs, e.g. as used in FFmpeg/IEC958).
    /// Disambiguates the bridge's raw transport (which carries no data-type
    /// byte). NULL → the bridge sniffs the sync word.
    pub codec: *const c_char,
    /// Enable the OSC live-control server. (Not yet wired in this build.)
    pub osc_enabled: c_int,
    /// Incoming OSC port (0 = auto).
    pub osc_port_in: u16,
    /// Outgoing/monitoring OSC port.
    pub osc_port_out: u16,
    /// OSC bind address (default "127.0.0.1").
    pub osc_bind: *const c_char,
    /// OSC monitoring target host.
    pub osc_host: *const c_char,
}

const VERSION_MAJOR: u32 = 0;
// 2: added orender_overlay_ass / orender_overlay_set_enabled (in-process overlay).
// 3: added orender_overlay_heatmap_bgra (BGRA energy-field bitmap for overlay-add).
// 4: added overlay toggles (labels/objects/trails/heatmap) + heatmap band/colormap cycling.
const VERSION_MINOR: u32 = 4;

unsafe fn opt_str<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    CStr::from_ptr(p).to_str().ok()
}

/// Diagnostics about how the *host* process (mpv) was launched, appended to the
/// degraded bridge-error so Studio can show it. liborender runs inside mpv, so
/// `current_dir()`/`args()` are mpv's own CWD and argv — exactly what's needed
/// to explain a relative `bridge_path` that resolves against the wrong folder.
///
/// Caveat: `args()` only sees the actual command line. Options coming from
/// `mpv.conf` / portable_config are read internally by mpv and won't appear
/// here — hence we surface the CWD too (which explains relative-path failures
/// regardless of where the path was set).
fn host_launch_diagnostics() -> String {
    let cwd = std::env::current_dir()
        .map(|p| p.display().to_string())
        .unwrap_or_else(|_| "<unknown>".to_string());
    let cmdline = std::env::args().collect::<Vec<_>>().join(" ");
    format!("\n\nWorking dir: {cwd}\nCommand line: {cmdline}")
}

fn build_engine(cfg: &OrenderConfig) -> Result<Engine> {
    // Seed the machine-wide Windows config from the legacy per-user location
    // once (no-op on Linux/macOS), before resolving the default path.
    orender_engine::migrate_legacy_windows_config();

    // Optional override; NULL → taken from the config YAML's render.bridge_path.
    let bridge_path = unsafe { opt_str(cfg.bridge_path) };
    // NULL config → the shared omniphony config (same as the CLI + studio),
    // so one config drives all hosts.
    let config_path = unsafe { opt_str(cfg.config_yaml_path) }
        .map(PathBuf::from)
        .or_else(orender_engine::default_config_path);
    let layout_path = unsafe { opt_str(cfg.speaker_layout_path) };
    let codec = unsafe { opt_str(cfg.codec) };
    let sample_rate = if cfg.sample_rate == 0 {
        48_000
    } else {
        cfg.sample_rate
    };

    // Load the shared config once: drives OSC settings (C override → config →
    // defaults) and seeds the degraded reporter below. The `_with_live`
    // variant keeps this pre-load consistent with `Engine::from_paths` when a
    // live-handoff sidecar was already consumed in this process (the OSC
    // fields themselves are never live-modified, so either source is correct).
    let render_cfg = config_path
        .as_deref()
        .map(|p| orender_engine::Config::load_or_default_with_live(p).0)
        .and_then(|c| c.render);
    // Resolve OSC up front so we can also reach Studio with a degraded reporter
    // if the bridge fails. `None` when OSC is off.
    let osc_opts = resolve_osc_opts(cfg, render_cfg.as_ref());

    // Settle OSC port ownership BEFORE `Engine::from_paths` reads the config:
    // a yieldable standby renderer writes its live-state sidecar while still
    // holding the port, so negotiating first guarantees `from_paths` sees the
    // handed-over state. Gated on a successful bridge pre-resolution (same
    // strict policy as `from_paths`): a host that is about to fall back to the
    // degraded reporter must not evict a healthy standby.
    let config_bridge = render_cfg.as_ref().and_then(|c| c.bridge_path.clone());
    let bridge_resolvable = orender_engine::bridge_loader::resolve_bridge(
        bridge_path.map(Path::new),
        config_bridge.as_deref(),
    )
    .is_ok();
    if bridge_resolvable {
        // A same-process degraded reporter may itself hold the port.
        stop_degraded_reporter_global();
        if let Some(opts) = osc_opts.as_ref() {
            if !orender_engine::osc::negotiate_rx_port(opts.port_in) {
                log::warn!(
                    "OSC RX port {} still busy after yield negotiation; \
                     the engine will run without an OSC listener",
                    opts.port_in
                );
            }
        }
    }

    let mut engine = match Engine::from_paths(
        config_path.as_deref(),
        layout_path.map(Path::new),
        bridge_path.map(Path::new),
        codec,
        sample_rate,
    ) {
        Ok(engine) => engine,
        Err(e) => {
            // The decoder bridge couldn't be resolved/loaded. Returning the
            // error makes orender_create yield NULL, so mpv falls back to its
            // native decoder (audio keeps working). But bring up a decoder-less
            // OSC reporter so Studio can show *why* spatial didn't engage —
            // Studio registers normally, so its address is known (no guessing).
            if let Some(opts) = osc_opts {
                start_degraded_reporter_global(
                    config_path.clone(),
                    sample_rate,
                    opts,
                    format!("{e:#}{}", host_launch_diagnostics()),
                );
            }
            return Err(e);
        }
    };

    // A real engine is starting; release any degraded reporter holding the OSC
    // port before we bind it.
    stop_degraded_reporter_global();

    if let Some(opts) = osc_opts {
        engine.enable_osc(opts)?;
    }

    Ok(engine)
}

/// Initialise the `log` backend once, so the engine's `log::*` diagnostics
/// (bridge-load time, "VBAP table generated in Xs", clip warnings, engine-ready
/// time) surface BOTH on stderr and over OSC to connected clients (Studio's log
/// panel).
///
/// This installs the engine's shared live-log logger — the same one the `orender`
/// CLI uses — rather than a plain `env_logger`, which only wrote to stderr and so
/// left the OSC log stream empty in the mpv/liborender build. Initial verbosity
/// comes from `RUST_LOG` (a bare level like `info`/`warn`/`debug`), defaulting to
/// `info`; it stays OSC-adjustable at runtime. Falls back to plain `env_logger`
/// only if the host already installed a global logger.
fn init_logging() {
    use std::sync::Once;
    static INIT: Once = Once::new();
    INIT.call_once(|| {
        let level = std::env::var("RUST_LOG")
            .ok()
            .and_then(|s| s.trim().parse::<log::LevelFilter>().ok())
            .unwrap_or(log::LevelFilter::Info);
        if orender_engine::init_live_logging(level, false).is_err() {
            let _ =
                env_logger::Builder::from_env(env_logger::Env::default().default_filter_or("warn"))
                    .try_init();
        }
    });
}

/// Create a session. Returns NULL on failure (bad config, missing bridge, etc.).
#[no_mangle]
pub unsafe extern "C" fn orender_create(cfg: *const OrenderConfig) -> *mut OrenderRenderer {
    init_logging();
    catch_unwind(AssertUnwindSafe(|| {
        if cfg.is_null() {
            return ptr::null_mut();
        }
        match build_engine(&*cfg) {
            Ok(engine) => {
                // Arm the spatial overlay: it stays blank until a session exists,
                // so a host that loads the overlay shim without selecting
                // `--ad=orender` (no session created) draws nothing.
                orender_engine::overlay::session_started();
                Box::into_raw(Box::new(engine)) as *mut OrenderRenderer
            }
            Err(e) => {
                eprintln!("orender_create failed: {e:#}");
                ptr::null_mut()
            }
        }
    }))
    .unwrap_or(ptr::null_mut())
}

/// Free a session created by [`orender_create`]. NULL is ignored.
#[no_mangle]
pub unsafe extern "C" fn orender_destroy(r: *mut OrenderRenderer) {
    if r.is_null() {
        return;
    }
    let _ = catch_unwind(AssertUnwindSafe(|| {
        drop(Box::from_raw(r as *mut Engine));
        // Disarm the overlay as this session goes away (clears the scene when it
        // was the last one), so the box can't linger past the stream.
        orender_engine::overlay::session_ended();
    }));
}

/// 1 if the current presentation carries spatial objects, 0 if it is a plain
/// multichannel stream (the host should fall back to its standard decoder),
/// <0 on error. Meaningful after at least one [`orender_process`] call.
#[no_mangle]
pub unsafe extern "C" fn orender_is_spatial(r: *const OrenderRenderer) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        if r.is_null() {
            return -1;
        }
        let engine = &*(r as *const Engine);
        if engine.is_spatial() {
            1
        } else {
            0
        }
    }))
    .unwrap_or(-1)
}

/// Configured render mode for channel-based (non-object) content:
/// 0 = host, 1 = spatial; <0 on error. When this is `host` (0) and
/// [`orender_is_spatial`] reports 0, the host should decline this track and fall
/// back to its native decoder. Meaningful once the renderer is created (the mode
/// comes from config / live params, not from the stream).
#[no_mangle]
pub unsafe extern "C" fn orender_channel_mode(r: *const OrenderRenderer) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        if r.is_null() {
            return -1;
        }
        (*(r as *const Engine)).channel_render_mode_code() as c_int
    }))
    .unwrap_or(-1)
}

/// Override the channel render mode for non-object content at runtime (a
/// per-host override of the config value): 0 = host, 1 = spatial. No-op on a
/// NULL handle.
#[no_mangle]
pub unsafe extern "C" fn orender_set_channel_mode(r: *mut OrenderRenderer, mode: c_int) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if r.is_null() {
            return;
        }
        (*(r as *mut Engine)).set_channel_render_mode_code(mode as i32);
    }));
}

/// Number of output channels (speakers) the renderer produces, 0 on error.
#[no_mangle]
pub unsafe extern "C" fn orender_channel_count(r: *const OrenderRenderer) -> u32 {
    catch_unwind(AssertUnwindSafe(|| {
        if r.is_null() {
            return 0;
        }
        (*(r as *const Engine)).channel_count()
    }))
    .unwrap_or(0)
}

/// Write the active output layout's per-channel labels (one [`RChannelLabel`]
/// byte per speaker, in render order) so the host can build a channel map.
///
/// Returns the channel count `N`. If `out_labels` is non-NULL and `cap >= N`,
/// the first `N` bytes are filled with label discriminants; otherwise nothing is
/// written — call with `out_labels = NULL` to query `N`, size a buffer, then
/// call again. Each byte is an `RChannelLabel` value (255 = Unknown). Returns 0
/// on error/NULL handle.
#[no_mangle]
pub unsafe extern "C" fn orender_channel_layout(
    r: *const OrenderRenderer,
    out_labels: *mut u8,
    cap: u32,
) -> u32 {
    catch_unwind(AssertUnwindSafe(|| {
        if r.is_null() {
            return 0;
        }
        let labels = (*(r as *const Engine)).channel_layout();
        let n = labels.len() as u32;
        if !out_labels.is_null() && cap >= n {
            let out = std::slice::from_raw_parts_mut(out_labels, labels.len());
            for (dst, lbl) in out.iter_mut().zip(labels.iter()) {
                *dst = *lbl as u8;
            }
        }
        n
    }))
    .unwrap_or(0)
}

/// Reset after a seek/discontinuity (flushes decoder + renderer state, keeps
/// live params).
#[no_mangle]
pub unsafe extern "C" fn orender_reset(r: *mut OrenderRenderer) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        if r.is_null() {
            return;
        }
        (*(r as *mut Engine)).reset();
    }));
}

/// Push one raw encoded packet and render whatever frames it yields.
///
/// The caller owns `out` (capacity `out_cap_samples` floats). On success the
/// rendered interleaved samples are written there and `*out_frames` /
/// `*out_channels` / `*out_pts_us` are set.
///
/// Returns: 0 = OK (may be 0 frames — need more data), >0 = output buffer too
/// small (nothing written; retry with a larger buffer), <0 = error.
#[no_mangle]
pub unsafe extern "C" fn orender_process(
    r: *mut OrenderRenderer,
    pkt: *const u8,
    pkt_len: usize,
    _pts_us: i64,
    out: *mut f32,
    out_cap_samples: usize,
    out_frames: *mut usize,
    out_channels: *mut u32,
    out_pts_us: *mut i64,
) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        if r.is_null() || pkt.is_null() || out.is_null() {
            return -1;
        }
        let engine = &mut *(r as *mut Engine);
        let data = std::slice::from_raw_parts(pkt, pkt_len);

        let chunks = match engine.process_raw(data) {
            Ok(c) => c,
            Err(e) => {
                eprintln!("orender_process error: {e:#}");
                return -2;
            }
        };

        let total_samples: usize = chunks.iter().map(|c| c.samples.len()).sum();
        if total_samples > out_cap_samples {
            if !out_frames.is_null() {
                *out_frames = 0;
            }
            return 1; // buffer too small; caller retries larger
        }

        let out_slice = std::slice::from_raw_parts_mut(out, out_cap_samples);
        let mut written = 0usize;
        let mut total_frames = 0usize;
        let mut n_channels = engine.channel_count();
        let mut first_sample_pos: Option<u64> = None;
        for chunk in &chunks {
            // An output-mode switch can land between blocks of one packet; a
            // mixed-layout copy would corrupt the frame geometry. Keep the
            // call single-layout and drop the tail (sub-millisecond of audio,
            // once per switch) — the next call carries the new layout.
            if total_frames > 0 && chunk.n_channels != n_channels {
                break;
            }
            out_slice[written..written + chunk.samples.len()].copy_from_slice(&chunk.samples);
            written += chunk.samples.len();
            total_frames += chunk.n_frames;
            n_channels = chunk.n_channels;
            first_sample_pos.get_or_insert(chunk.sample_pos);
        }

        if !out_frames.is_null() {
            *out_frames = total_frames;
        }
        if !out_channels.is_null() {
            *out_channels = n_channels;
        }
        if !out_pts_us.is_null() {
            let sr = engine.sample_rate().max(1) as i64;
            *out_pts_us = first_sample_pos
                .map(|p| (p as i64) * 1_000_000 / sr)
                .unwrap_or(0);
        }
        0
    }))
    .unwrap_or(-100)
}

/// Render the spatial overlay for the given OSD resolution and copy the ASS
/// `osd-overlay` payload into `out` (UTF-8, not nul-terminated).
///
/// This *is* the overlay redraw: each call rebuilds the scene and advances the
/// motion trails, so the host (the mpv Lua shim) must call it exactly once per
/// redraw — typically on a periodic timer and on OSD resize. It also marks the
/// overlay "active" so the engine starts feeding it (the engine does no overlay
/// work until the first pull).
///
/// Returns the number of bytes the payload needs. If `out` is non-NULL and
/// `cap >= len`, the first `len` bytes are written; otherwise nothing is written
/// (the host should grow its buffer and skip this redraw — the next one fits).
/// A handful of KiB is always enough; the output is bounded. Returns 0 when the
/// overlay is disabled, the resolution is zero, or there is nothing to draw.
///
/// Handle-less by design: the overlay is a process-global singleton, and the Lua
/// shim has no session handle (it `ffi.load`s this already-loaded library).
#[no_mangle]
pub unsafe extern "C" fn orender_overlay_ass(
    res_x: u32,
    res_y: u32,
    out: *mut u8,
    cap: usize,
) -> usize {
    catch_unwind(AssertUnwindSafe(|| {
        let ass = orender_engine::overlay::build_ass(res_x, res_y);
        let bytes = ass.as_bytes();
        let n = bytes.len();
        if !out.is_null() && cap >= n {
            let dst = std::slice::from_raw_parts_mut(out, n);
            dst.copy_from_slice(bytes);
        }
        n
    }))
    .unwrap_or(0)
}

/// Enable or disable the overlay (host keybind / script message). Disabling also
/// makes the engine stop feeding it. `0` = off, non-zero = on.
#[no_mangle]
pub extern "C" fn orender_overlay_set_enabled(enabled: c_int) {
    let _ = catch_unwind(AssertUnwindSafe(|| {
        orender_engine::overlay::set_enabled(enabled != 0);
    }));
}

// ── overlay toggles (host keybinds) ──────────────────────────────────────────
//
// Each flips the matching control inside the renderer and returns the *new*
// state (1 = on, 0 = off; the heatmap band/colormap variants return the new
// numeric value). Returning the result lets the mpv shim show it in the OSD
// without keeping a mirror that could drift from Studio's OSC pushes. On a panic
// the catch returns a safe default (0).

/// Flip the master enable and return the new state (1 = on, 0 = off).
#[no_mangle]
pub extern "C" fn orender_overlay_toggle() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        orender_engine::overlay::toggle_enabled() as c_int
    }))
    .unwrap_or(0)
}

/// Flip object-label visibility and return the new state (1 = on, 0 = off).
#[no_mangle]
pub extern "C" fn orender_overlay_toggle_labels() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        orender_engine::overlay::toggle_labels() as c_int
    }))
    .unwrap_or(0)
}

/// Flip object visibility (markers + labels + trails + depth lines) and return
/// the new state (1 = on, 0 = off).
#[no_mangle]
pub extern "C" fn orender_overlay_toggle_objects() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        orender_engine::overlay::toggle_objects() as c_int
    }))
    .unwrap_or(0)
}

/// Flip whether motion trails are drawn and return the new state (1 = on,
/// 0 = off). Clears the trail buffers when disabling.
#[no_mangle]
pub extern "C" fn orender_overlay_toggle_trails() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        orender_engine::overlay::toggle_trails() as c_int
    }))
    .unwrap_or(0)
}

/// Flip the object energy heatmap and return the new state (1 = on, 0 = off).
#[no_mangle]
pub extern "C" fn orender_overlay_toggle_heatmap() -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        orender_engine::overlay::toggle_heatmap() as c_int
    }))
    .unwrap_or(0)
}

/// Advance the heatmap colour gradient to the next index (wraps 0..=4) and return
/// the new index.
#[no_mangle]
pub extern "C" fn orender_overlay_cycle_heatmap_colormap() -> u32 {
    catch_unwind(AssertUnwindSafe(|| {
        orender_engine::overlay::cycle_heatmap_colormap() as u32
    }))
    .unwrap_or(0)
}

/// Step the heatmap depth-plane count by `delta` (clamped to 1..=12) and return
/// the new count.
#[no_mangle]
pub extern "C" fn orender_overlay_adjust_heatmap_bands(delta: i32) -> u32 {
    catch_unwind(AssertUnwindSafe(|| {
        orender_engine::overlay::adjust_heatmap_bands(delta) as u32
    }))
    .unwrap_or(0)
}

/// Render the object energy heatmap as a single flattened BGRA bitmap
/// (premultiplied alpha) for mpv's `overlay-add`, drawn *under* the ASS overlay.
///
/// On success copies `w*h*4` BGRA bytes into `out` and writes the geometry into
/// `geom` (6 × i32: `[x, y, w, h, dw, dh]` — top-left position, source size, and
/// the on-screen display size mpv scales the source to), then returns the number
/// of bytes written. Returns 0 — and writes nothing — when the overlay is
/// disabled, the resolution is zero, the buffers are too small, or there is no
/// audible object. The bitmap is bounded (`FIELD_BITMAP_MAX²·4` ≈ 256 KiB).
///
/// Read-only with respect to the scene: unlike `orender_overlay_ass`, this does
/// not advance trails or the pull clock (the ASS pull already does), so the host
/// may call it alongside the ASS redraw.
#[no_mangle]
pub unsafe extern "C" fn orender_overlay_heatmap_bgra(
    res_x: u32,
    res_y: u32,
    out: *mut u8,
    cap: usize,
    geom: *mut i32,
) -> usize {
    catch_unwind(AssertUnwindSafe(|| {
        if out.is_null() || geom.is_null() {
            return 0;
        }
        let Some(bmp) = orender_engine::overlay::build_heatmap(res_x, res_y) else {
            return 0;
        };
        let n = bmp.pixels.len();
        if n == 0 || n > cap {
            return 0;
        }
        std::slice::from_raw_parts_mut(out, n).copy_from_slice(&bmp.pixels);
        let g = std::slice::from_raw_parts_mut(geom, 6);
        g[0] = bmp.x;
        g[1] = bmp.y;
        g[2] = bmp.w;
        g[3] = bmp.h;
        g[4] = bmp.dw;
        g[5] = bmp.dh;
        n
    }))
    .unwrap_or(0)
}

/// ABI major version. A bump means a breaking change (new soname).
#[no_mangle]
pub extern "C" fn orender_version_major() -> u32 {
    VERSION_MAJOR
}

/// ABI minor version (backwards-compatible additions).
#[no_mangle]
pub extern "C" fn orender_version_minor() -> u32 {
    VERSION_MINOR
}
