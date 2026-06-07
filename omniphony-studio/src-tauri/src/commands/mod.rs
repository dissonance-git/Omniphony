//! Tauri command handlers, grouped into themed modules.
//!
//! Each command is a thin glue function: it takes the shared [`SharedState`] and
//! forwards a value to the renderer over OSC. They live here, split by feature
//! area, instead of in one large `main.rs`. `main.rs` glob-imports each module so
//! `tauri::generate_handler!` can keep listing commands by bare name.
//!
//! [`SharedState`]: crate::SharedState

pub mod resampling;
