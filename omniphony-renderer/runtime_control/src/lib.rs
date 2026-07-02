// The renderer-state `serde_json::json!` literal in `snapshot.rs` is large; raise
// the macro recursion limit so it expands.
#![recursion_limit = "256"]

pub mod command;
pub mod context;
pub mod host_control;
pub mod osc;
pub mod osc_contract;
pub mod persist;
pub mod snapshot;

pub use host_control::HostControlHandler;

/// Build fingerprint of this workspace build (`<git-describe> (built <ts>)`),
/// stamped by this crate's build.rs. Both renderer hosts (the `orender` CLI
/// and the `liborender` cdylib) link this crate, so the string identifies the
/// engine build regardless of packaging.
pub fn build_fingerprint() -> String {
    format!(
        "{} (built {})",
        env!("VERGEN_GIT_DESCRIBE"),
        env!("BUILD_TIMESTAMP"),
    )
}
