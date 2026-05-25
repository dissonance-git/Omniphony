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
    /// Load a bridge plugin from `path` and create one instance with the given strict-mode flag.
    ///
    /// Format-specific options (e.g. presentation index) are applied afterwards via
    /// [`FormatBridgeBox::configure`] before the first [`FormatBridgeBox::push_packet`].
    pub fn load_with_params(path: &Path, strict: bool) -> Result<Self> {
        let lib = BridgeLibRef::load_from_file(path)
            .with_context(|| format!("Failed to load bridge plugin from {}", path.display()))?;
        install_bridge_host_log_sink(&lib);
        let new_bridge = lib.new_bridge();
        let bridge = new_bridge(strict);
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
/// Note: the exe-relative fallback (2) only makes sense when the host is the
/// `orender` binary. Library hosts (e.g. mpv loading `liborender.so`) must pass
/// an explicit path, because `current_exe()` would resolve to the host program.
pub fn resolve_bridge_path(explicit: Option<&Path>) -> Result<PathBuf> {
    // 1. Explicit path from CLI/config
    if let Some(path) = explicit {
        if path.is_file() {
            return Ok(path.to_path_buf());
        }
        bail!(
            "Bridge path '{}' does not exist or is not a file",
            path.display()
        );
    }

    // 2. Search next to the executable
    let exe = std::env::current_exe().context("Cannot determine executable path")?;
    let dir = exe.parent().context("Executable has no parent directory")?;
    let mut matches = find_bridge_candidates(dir)?;
    matches.sort();
    if let Some(path) = matches.into_iter().next() {
        return Ok(path);
    }

    bail!(
        "No bridge plugin found.\n\
         Searched in: {}\n\
         Expected one file matching: *_bridge.so / *_bridge.dll / *_bridge.dylib\n\
         Provide --bridge-path <FILE> or set render.bridge_path in config.",
        dir.display(),
    )
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
