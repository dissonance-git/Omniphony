//! Single source of truth for `RenderConfig` option fields.
//!
//! Historically each option's default lived in 3–4 places (clap
//! `default_value_t`, the FFI `from_render_config` `unwrap_or(..)`, and the
//! "skip if == default" thresholds in *both* config writers), which drift
//! independently. Each option declared here owns its default exactly once and
//! exposes a symmetric `get` / `store` pair so every read and write site goes
//! through the same logic:
//!
//! - [`DEFAULT`](self) — the canonical default value, referenced by clap and
//!   by the FFI reader instead of a hard-coded literal.
//! - `get(&RenderConfig) -> Option<T>` — typed read.
//! - `store(&mut RenderConfig, T)` — writes `Some(v)` only when `v` differs
//!   from the default, `None` otherwise. Both the CLI (`effective_to_config`)
//!   and the live (`save_live_config`) writers call this, so their skip-if-
//!   default behaviour can no longer diverge.
//!
//! The descriptor centralises only the *config* side (key + default + get/
//! store). Extracting the value from the runtime source (`RenderArgs` for the
//! CLI, `LiveParams` for the live path) stays a one-liner at the call site,
//! because those types live in other crates.

use crate::config::RenderConfig;

/// Declare a config option as a `{ DEFAULT, get, store }` descriptor module.
///
/// `eq` is the equality used by `store` to decide whether a value equals the
/// default; pass `i32::eq` / `bool::eq` for exact types or a tolerance closure
/// (e.g. `|a: &f32, b: &f32| (a - b).abs() < 1e-4`) for floats so the existing
/// writer thresholds are reproduced exactly.
macro_rules! render_field {
    (
        $(#[$meta:meta])*
        $vis:vis $name:ident : $ty:ty = $default:expr,
        field = $field:ident,
        eq = $eq:expr
    ) => {
        $(#[$meta])*
        $vis mod $name {
            use super::RenderConfig;

            /// Canonical default — the single copy of this field's default.
            pub const DEFAULT: $ty = $default;

            /// Read the configured value, if present.
            pub fn get(cfg: &RenderConfig) -> Option<$ty> {
                cfg.$field.clone()
            }

            /// Store `value`, writing `Some` only when it differs from
            /// [`DEFAULT`] (skip-if-default), `None` otherwise.
            pub fn store(cfg: &mut RenderConfig, value: $ty) {
                let eq: fn(&$ty, &$ty) -> bool = $eq;
                cfg.$field = if eq(&value, &DEFAULT) {
                    None
                } else {
                    Some(value)
                };
            }
        }
    };
}

render_field! {
    /// Number of distance cells across the polar VBAP precompute table
    /// (`render.vbap_distance_res`). Pilot field for the descriptor scheme.
    pub vbap_distance_res: i32 = 8,
    field = vbap_distance_res,
    eq = i32::eq
}

render_field! {
    /// Polar VBAP azimuth cell count across [-180°, +180°]
    /// (`render.vbap_azimuth_resolution`).
    pub vbap_azimuth_resolution: i32 = 360,
    field = vbap_azimuth_resolution,
    eq = i32::eq
}

render_field! {
    /// Polar VBAP elevation cell count (`render.vbap_elevation_resolution`).
    pub vbap_elevation_resolution: i32 = 180,
    field = vbap_elevation_resolution,
    eq = i32::eq
}

render_field! {
    /// Maximum distance covered by the polar VBAP table
    /// (`render.vbap_distance_max`). Float field: the live writer used a 1e-4
    /// tolerance and the CLI writer `f32::EPSILON`; unified here on 1e-4 (the
    /// more robust threshold — only sub-1e-4 float noise is now treated as the
    /// default on the CLI side).
    pub vbap_distance_max: f32 = 2.0,
    field = vbap_distance_max,
    eq = |a: &f32, b: &f32| (a - b).abs() < 1e-4
}

render_field! {
    /// Minimum VBAP spread floor for point sources (`render.vbap_spread_min`).
    pub vbap_spread_min: f32 = 0.0,
    field = vbap_spread_min,
    eq = |a: &f32, b: &f32| *a == *b
}

render_field! {
    /// Maximum VBAP spread cap for fully-diffuse objects
    /// (`render.vbap_spread_max`).
    pub vbap_spread_max: f32 = 1.0,
    field = vbap_spread_max,
    eq = |a: &f32, b: &f32| *a == *b
}

render_field! {
    /// Interpolate between neighbouring VBAP table cells on lookup
    /// (`render.render_evaluation_position_interpolation`). Default is *on*
    /// (true), so only an explicit `false` is persisted. This also fixes a CLI
    /// writer asymmetry where disabling interpolation was previously dropped
    /// (it wrote `Some(true)` / `None`, never `Some(false)`).
    pub render_evaluation_position_interpolation: bool = true,
    field = render_evaluation_position_interpolation,
    eq = bool::eq
}

#[cfg(test)]
mod tests {
    use crate::config::RenderConfig;

    #[test]
    fn store_non_default_sets_some() {
        let mut cfg = RenderConfig::default();
        super::vbap_distance_res::store(&mut cfg, 12);
        assert_eq!(cfg.vbap_distance_res, Some(12));
    }

    #[test]
    fn store_default_clears_to_none() {
        let mut cfg = RenderConfig {
            vbap_distance_res: Some(12),
            ..Default::default()
        };
        super::vbap_distance_res::store(&mut cfg, super::vbap_distance_res::DEFAULT);
        assert_eq!(cfg.vbap_distance_res, None);
    }

    #[test]
    fn get_round_trips_through_store() {
        let mut cfg = RenderConfig::default();
        assert_eq!(super::vbap_distance_res::get(&cfg), None);
        super::vbap_distance_res::store(&mut cfg, 5);
        assert_eq!(super::vbap_distance_res::get(&cfg), Some(5));
    }

    /// CLI and live writers now share `store`, so feeding the same value from
    /// either path must yield an identical config field (parity guard).
    #[test]
    fn cli_and_live_writers_agree() {
        let mut from_cli = RenderConfig::default();
        let mut from_live = RenderConfig::default();
        super::vbap_distance_res::store(&mut from_cli, 12); // CLI: args value
        super::vbap_distance_res::store(&mut from_live, 12); // live: LiveParams value
        assert_eq!(from_cli.vbap_distance_res, from_live.vbap_distance_res);
    }

    #[test]
    fn float_field_uses_tolerance() {
        let mut cfg = RenderConfig::default();
        super::vbap_distance_max::store(&mut cfg, 2.0);
        assert_eq!(cfg.vbap_distance_max, None, "exact default cleared");
        super::vbap_distance_max::store(&mut cfg, 2.000_05);
        assert_eq!(
            cfg.vbap_distance_max, None,
            "within 1e-4 treated as default"
        );
        super::vbap_distance_max::store(&mut cfg, 3.0);
        assert_eq!(cfg.vbap_distance_max, Some(3.0));
    }

    #[test]
    fn interpolation_default_is_on_and_false_persists() {
        let mut cfg = RenderConfig::default();
        // Default (on) is not written.
        super::render_evaluation_position_interpolation::store(&mut cfg, true);
        assert_eq!(cfg.render_evaluation_position_interpolation, None);
        // Disabling it IS persisted (the previous CLI writer dropped this).
        super::render_evaluation_position_interpolation::store(&mut cfg, false);
        assert_eq!(cfg.render_evaluation_position_interpolation, Some(false));
    }
}
