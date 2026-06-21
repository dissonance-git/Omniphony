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
