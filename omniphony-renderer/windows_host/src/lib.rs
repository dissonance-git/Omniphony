//! Reusable Windows platform adapters for the Omniphony native host.
//!
//! The executable shell remains in `main.rs`. This library target exists so
//! platform ingress contracts can be compiled and regression-tested without
//! coupling them to tray/device lifecycle code.

pub mod spatial_ingress;
pub mod spatial_source_frame;
pub mod spatial_source_slots;
