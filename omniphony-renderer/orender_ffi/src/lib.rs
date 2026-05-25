//! C ABI for the `orender` Atmos renderer — built as `liborender.so`.
//!
//! A thin, panic-safe shim over [`orender_engine::Engine`]: the host (mpv via
//! `ad_orender.c`, or any C program) creates a session from a config, pushes
//! raw TrueHD packets, and receives interleaved multichannel `f32` PCM. No
//! audio output happens here — the host owns that.
//!
//! Every entry point catches Rust panics at the boundary (a panic crossing into
//! C is undefined behaviour) and the C caller owns all output buffers.

#![allow(clippy::missing_safety_doc)]

use orender_engine::Engine;

use anyhow::{Result, anyhow};
use std::ffi::CStr;
use std::os::raw::{c_char, c_int};
use std::panic::{AssertUnwindSafe, catch_unwind};
use std::path::Path;
use std::ptr;

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
    /// Path to the omniphony YAML config (drives the speaker layout + all render
    /// params). NULL → built-in defaults.
    pub config_yaml_path: *const c_char,
    /// Optional speaker-layout YAML path overriding the config. NULL → use the
    /// config's embedded layout, else the 7.1.4 preset.
    pub speaker_layout_path: *const c_char,
    /// Path to the decoder bridge plugin (e.g. truehd_bridge.so). REQUIRED:
    /// library hosts cannot use the exe-relative search.
    pub bridge_path: *const c_char,
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
const VERSION_MINOR: u32 = 1;

unsafe fn opt_str<'a>(p: *const c_char) -> Option<&'a str> {
    if p.is_null() {
        return None;
    }
    CStr::from_ptr(p).to_str().ok()
}

fn build_engine(cfg: &OrenderConfig) -> Result<Engine> {
    let bridge_path =
        unsafe { opt_str(cfg.bridge_path) }.ok_or_else(|| anyhow!("bridge_path is required"))?;
    let config_path = unsafe { opt_str(cfg.config_yaml_path) };
    let layout_path = unsafe { opt_str(cfg.speaker_layout_path) };
    let sample_rate = if cfg.sample_rate == 0 {
        48_000
    } else {
        cfg.sample_rate
    };

    let mut engine = Engine::from_paths(
        config_path.map(Path::new),
        layout_path.map(Path::new),
        Path::new(bridge_path),
        sample_rate,
    )?;

    if cfg.osc_enabled != 0 {
        let host = unsafe { opt_str(cfg.osc_host) }
            .unwrap_or("127.0.0.1")
            .to_string();
        engine.enable_osc(orender_engine::OscOptions {
            host,
            port_out: cfg.osc_port_out,
            port_in: cfg.osc_port_in,
        })?;
    }

    Ok(engine)
}

/// Create a session. Returns NULL on failure (bad config, missing bridge, etc.).
#[no_mangle]
pub unsafe extern "C" fn orender_create(cfg: *const OrenderConfig) -> *mut OrenderRenderer {
    catch_unwind(AssertUnwindSafe(|| {
        if cfg.is_null() {
            return ptr::null_mut();
        }
        match build_engine(&*cfg) {
            Ok(engine) => Box::into_raw(Box::new(engine)) as *mut OrenderRenderer,
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
    }));
}

/// 1 if the current presentation may contain spatial objects (Atmos), 0 if not
/// (plain TrueHD — the host should fall back to its standard decoder), <0 on
/// error. Meaningful after at least one [`orender_process`] call.
#[no_mangle]
pub unsafe extern "C" fn orender_is_spatial(r: *const OrenderRenderer) -> c_int {
    catch_unwind(AssertUnwindSafe(|| {
        if r.is_null() {
            return -1;
        }
        let engine = &*(r as *const Engine);
        if engine.is_spatial() { 1 } else { 0 }
    }))
    .unwrap_or(-1)
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

/// Push one raw TrueHD packet and render whatever frames it yields.
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
