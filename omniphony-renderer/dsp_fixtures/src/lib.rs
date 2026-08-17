//! Deterministic fixtures and analysis for Omniphony DSP validation.
//!
//! Dev-only: this crate is a path dev-dependency of `renderer`, and nothing in
//! the dependency graph of `orender` or `liborender` references it, so release
//! builds never compile it.
//!
//! It exists so that the null test, the criterion benches, and the future
//! worst-case-block-time gate all measure *the same* scenes. Duplicating scene
//! generation between those consumers is how they silently drift apart.
//!
//! `renderer` also compiles this crate as a dev-dependency, which gives that
//! dependency its own Rust type identity. Re-export renderer-owned argument
//! types used by fixture-returned renderers here so tests never accidentally
//! mix them with an identically named type from the outer renderer test crate.

pub use renderer::live_params::BinauralMode;
pub use renderer::spatial_renderer::SpatialChannelEvent;

pub mod analysis;
pub mod binaural_block_size;
pub mod binaural_groove_fidelity;
pub mod diagnostic_signals;
pub mod dirs;
pub mod end_to_end_spatial;
pub mod golden;
pub mod orbit;
pub mod residual;
pub mod scene;
pub mod stream_reset;
