use abi_stable::library::RootModule;
use abi_stable::std_types::RStr;
use anyhow::{Context, Result, bail};
use bridge_api::{
    BridgeHostLogSink, BridgeLibRef, FormatBridgeBox, RLogLevel, RVbapCartesianDefaults,
    RVbapTableMode,
};
use std::path::{Path, PathBuf};

/// Loaded bridge library + live bridge instance.
///
/// Both fields must be kept alive together: `lib` holds the reference-count
/// that prevents the `.so` from being unloaded while `bridge` is in use.
pub struct LoadedBridge {
    /// Keeps the `.so` resident in memory.
    pub lib: BridgeLibRef,
    /// The live bridge instance (stateful, called per chunk).
    pub bridge: FormatBridgeBox,
}

impl LoadedBridge {
    /// Load a bridge plugin from `path` and create one instance.
    ///
    /// Format-specific options (e.g. presentation index) are applied afterwards via
    /// [`FormatBridgeBox::configure`] before the first [`FormatBridgeBox::push_packet`].
    pub fn load_with_params(path: &Path) -> Result<Self> {
        let lib = BridgeLibRef::load_from_file(path)
            .with_context(|| format!("Failed to load bridge plugin from {}", path.display()))?;
        install_bridge_host_log_sink(&lib);
        let new_bridge = lib.new_bridge();
        // strict mode removed from the host; bridges ignore the flag. The ABI
        // parameter is kept for compatibility and always passed as `false`.
        let bridge = new_bridge(false);
        Ok(Self { lib, bridge })
    }

    /// Set a bridge configuration option. Must be called before the first packet.
    pub fn configure(&mut self, key: &str, value: &str) -> bool {
        self.bridge.configure(key.into(), value.into())
    }

    /// Default Cartesian VBAP grid dimensions suggested by the bridge.
    pub fn vbap_cartesian_defaults(&self) -> RVbapCartesianDefaults {
        self.bridge.vbap_cartesian_defaults()
    }

    /// Preferred VBAP table mode suggested by the bridge.
    pub fn preferred_vbap_table_mode(&self) -> RVbapTableMode {
        self.bridge.preferred_vbap_table_mode()
    }
}

pub fn install_bridge_host_log_sink(lib: &BridgeLibRef) {
    let Some(set_host_log_sink) = lib.set_host_log_sink() else {
        return;
    };
    set_host_log_sink(forward_bridge_log_to_host as BridgeHostLogSink as usize);
}

extern "C" fn forward_bridge_log_to_host(level: RLogLevel, target: RStr<'_>, message: RStr<'_>) {
    let level = match level {
        RLogLevel::Error => log::Level::Error,
        RLogLevel::Warn => log::Level::Warn,
        RLogLevel::Info => log::Level::Info,
        RLogLevel::Debug => log::Level::Debug,
        RLogLevel::Trace => log::Level::Trace,
    };
    sys::live_log::emit_external_record(level, target.as_str(), message.as_str());
}

/// Resolve the path to the bridge plugin.
///
/// Search order:
/// 1. `--bridge-path` / config-provided explicit file path
/// 2. Any file matching `*_bridge.so` / `.dll` / `.dylib` next to the executable
///
/// The exe-relative fallback applies to *any* host: the `orender` CLI, but
/// also library hosts like mpv loading `liborender.dll`/`.so`. On Windows in
/// particular the typical install pattern (extract a release zip into a single
/// folder) lands `mpv.exe`, `orender.dll` and the bridge `.dll` side by side,
/// so `current_exe()` -> mpv.exe's parent dir is exactly where the bridge
/// sits. On systems where the host binary is in a system path that won't
/// contain bridge plugins (e.g. `/usr/bin/mpv` on Linux), the fallback simply
/// finds nothing and the caller gets the regular missing-bridge error.
pub fn resolve_bridge_path(explicit: Option<&Path>) -> Result<PathBuf> {
    resolve_bridge(explicit, None)
}

/// Unified, strict bridge-path resolution shared by the CLI (`resolve_bridge_path`)
/// and the FFI/mpv host (`Engine::from_paths`), so both behave identically.
///
/// Order of precedence — an *explicitly requested* path (CLI `--bridge-path`,
/// FFI param, or `render.bridge_path` in the config) is taken **as a strict
/// instruction**: it must point at an existing file (the *named* one — we never
/// substitute a different bridge), else error. We do not fall through to the
/// auto-discovery glob when a path was requested. Only when no path is requested
/// at all do we auto-discover a `*_bridge.{so,dll,dylib}` next to the host
/// executable (the "drop the bundle in one folder" install).
///
/// A requested path that is **relative** is resolved CWD-independently — against
/// the process working dir first, then the host executable's directory (see
/// [`resolve_requested`]) — so a bare `harletty_bridge.dll` next to `mpv.exe`
/// works regardless of which folder the host was launched from. This is the
/// common real-world footgun: a relative `render.bridge_path` / `--bridge-path`
/// that previously only resolved against the CWD.
///
/// 1. `explicit` set → must resolve to a file, else error.
/// 2. else `config` (`render.bridge_path`) set → must resolve to a file, else error.
/// 3. else → [`find_bridge_next_to_exe`].
pub fn resolve_bridge(explicit: Option<&Path>, config: Option<&Path>) -> Result<PathBuf> {
    if let Some(path) = explicit {
        if let Some(found) = resolve_requested(path) {
            return Ok(found);
        }
        bail!(
            "bridge path '{}' does not exist or is not a file{}. \
             Give an absolute path to the decoder bridge, or drop a \
             *_bridge.{{so,dll,dylib}} next to the host binary and remove the \
             explicit path.",
            path.display(),
            searched_locations_hint(path),
        );
    }
    if let Some(path) = config {
        if let Some(found) = resolve_requested(path) {
            return Ok(found);
        }
        bail!(
            "render.bridge_path '{}' (from config) does not exist or is not a file{}. \
             Fix it to an existing file (an absolute path is safest), or remove it \
             and drop a *_bridge.{{so,dll,dylib}} next to the host binary.",
            path.display(),
            searched_locations_hint(path),
        );
    }
    find_bridge_next_to_exe().context(
        "no decoder bridge requested (no explicit path, no render.bridge_path) and \
         none found next to the host binary",
    )
}

/// Resolve a *requested* bridge path (CLI `--bridge-path`, FFI param, or
/// `render.bridge_path`). An absolute path is taken verbatim. A **relative**
/// path is resolved deterministically and CWD-independently: tried against the
/// process working dir first (back-compat) then the host executable's directory,
/// so a bare filename like `harletty_bridge.dll` dropped next to `mpv.exe` /
/// `orender` is found no matter which folder the host was launched from. Returns
/// the first candidate that is an existing file, else `None`.
fn resolve_requested(path: &Path) -> Option<PathBuf> {
    if path.is_absolute() {
        return path.is_file().then(|| path.to_path_buf());
    }
    requested_search_bases()
        .into_iter()
        .map(|base| base.join(path))
        .find(|cand| cand.is_file())
}

/// Base directories a *relative* requested bridge path is resolved against, in
/// priority order: the process CWD, then the host executable's directory.
fn requested_search_bases() -> Vec<PathBuf> {
    let mut bases = Vec::new();
    if let Ok(cwd) = std::env::current_dir() {
        bases.push(cwd);
    }
    if let Ok(exe) = std::env::current_exe() {
        if let Some(dir) = exe.parent() {
            bases.push(dir.to_path_buf());
        }
    }
    bases
}

/// For error messages: list the absolute candidates a *relative* path was tried
/// at (empty for absolute paths, which are self-explanatory).
fn searched_locations_hint(path: &Path) -> String {
    if path.is_absolute() {
        return String::new();
    }
    let tried: Vec<String> = requested_search_bases()
        .iter()
        .map(|base| base.join(path).display().to_string())
        .collect();
    if tried.is_empty() {
        String::new()
    } else {
        format!(" (searched: {})", tried.join(", "))
    }
}

/// Look for a `*_bridge.{so,dll,dylib}` next to the current executable.
/// Used as a fallback both by [`resolve_bridge_path`] and by
/// [`crate::engine::Engine::from_paths`] when no explicit / config-provided
/// path exists or the one provided no longer points at a real file.
pub fn find_bridge_next_to_exe() -> Result<PathBuf> {
    let exe = std::env::current_exe().context("Cannot determine executable path")?;
    let dir = exe.parent().context("Executable has no parent directory")?;
    let mut matches = find_bridge_candidates(dir)?;
    matches.sort();
    matches.into_iter().next().ok_or_else(|| {
        anyhow::anyhow!(
            "No bridge plugin found.\n\
             Searched in: {}\n\
             Expected one file matching: *_bridge.so / *_bridge.dll / *_bridge.dylib",
            dir.display()
        )
    })
}

fn find_bridge_candidates(dir: &Path) -> Result<Vec<PathBuf>> {
    let mut out = Vec::new();
    for entry in std::fs::read_dir(dir)
        .with_context(|| format!("Failed to read executable directory {}", dir.display()))?
    {
        let entry = entry?;
        let path = entry.path();
        if !path.is_file() {
            continue;
        }
        let Some(name) = path.file_name().and_then(|s| s.to_str()) else {
            continue;
        };
        if is_bridge_filename(name) {
            out.push(path);
        }
    }
    Ok(out)
}

fn is_bridge_filename(name: &str) -> bool {
    name.ends_with("_bridge.so") || name.ends_with("_bridge.dll") || name.ends_with("_bridge.dylib")
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::fs;
    use std::sync::atomic::{AtomicU32, Ordering};

    // Unique temp file per call so parallel tests don't collide.
    fn tmp_bridge(stem: &str) -> PathBuf {
        static N: AtomicU32 = AtomicU32::new(0);
        let mut p = std::env::temp_dir();
        p.push(format!(
            "orender_{}_{}_{}_bridge.so",
            std::process::id(),
            N.fetch_add(1, Ordering::Relaxed),
            stem
        ));
        fs::write(&p, b"x").unwrap();
        p
    }

    #[test]
    fn explicit_existing_is_used() {
        let f = tmp_bridge("expl");
        assert_eq!(resolve_bridge(Some(&f), None).unwrap(), f);
        fs::remove_file(&f).ok();
    }

    #[test]
    fn explicit_missing_errors() {
        let missing = Path::new("definitely_nonexistent_bridge.so");
        assert!(resolve_bridge(Some(missing), None).is_err());
    }

    #[test]
    fn config_used_when_no_explicit() {
        let f = tmp_bridge("cfg");
        assert_eq!(resolve_bridge(None, Some(&f)).unwrap(), f);
        fs::remove_file(&f).ok();
    }

    #[test]
    fn config_missing_errors() {
        let missing = Path::new("nonexistent_cfg_bridge.so");
        assert!(resolve_bridge(None, Some(missing)).is_err());
    }

    #[test]
    fn explicit_wins_over_config() {
        let expl = tmp_bridge("prec_expl");
        let cfg = tmp_bridge("prec_cfg");
        assert_eq!(resolve_bridge(Some(&expl), Some(&cfg)).unwrap(), expl);
        fs::remove_file(&expl).ok();
        fs::remove_file(&cfg).ok();
    }

    // Strict: a requested-but-missing explicit path is an error, NOT a cue to
    // silently fall back to a valid config path.
    #[test]
    fn missing_explicit_does_not_fall_through_to_config() {
        let cfg = tmp_bridge("ft_cfg");
        let missing = Path::new("missing_explicit_bridge.so");
        assert!(resolve_bridge(Some(missing), Some(&cfg)).is_err());
        fs::remove_file(&cfg).ok();
    }

    // A *relative* requested name (the real-world footgun: `harletty_bridge.dll`)
    // must resolve against the host executable's directory, not just the CWD —
    // so it's found wherever the host was launched from. We drop a uniquely-named
    // file next to the test binary and request it by bare relative name.
    #[test]
    fn relative_resolves_next_to_exe() {
        let dir = std::env::current_exe()
            .unwrap()
            .parent()
            .unwrap()
            .to_path_buf();
        let name = format!("orender_{}_reltest_bridge.so", std::process::id());
        let full = dir.join(&name);
        fs::write(&full, b"x").unwrap();
        let got = resolve_bridge(Some(Path::new(&name)), None);
        fs::remove_file(&full).ok();
        assert_eq!(got.unwrap(), full);
    }
}
