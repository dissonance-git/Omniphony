//! Source-aware presentation policy for already-separated causal sources.
//!
//! Retro VGM Compiler owns source truth. This module chooses only an Omniphony
//! presentation. Native routing constrains presentation but never becomes fake
//! authored 3-D. The source-aware sphere is intentionally a modern immersive
//! remix: when opened, stable recovered sources may occupy a larger 3-D field
//! even when the historical artifact never authored rear, height, or distance.

use crate::spatial_vbap::spherical_to_adm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLaneKind {
    DrySource,
    SharedWetReturn,
    ReferenceMix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePositionAuthority {
    Authored,
    InferredPresentation,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct NativeStereoRoute {
    pub left_gain: f32,
    pub right_gain: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourceSceneEvidence {
    pub lane_kind: SourceLaneKind,
    pub source_id: u64,
    pub persistent_part_id: Option<u64>,
    pub native_stereo_route: Option<NativeStereoRoute>,
    pub authored_position: Option<[f64; 3]>,
    pub foundation: f32,
    pub foreground: f32,
    pub diffuse: f32,
    pub width: f32,
    pub vertical_affinity: f32,
    pub confidence: f32,
}

impl Default for SourceSceneEvidence {
    fn default() -> Self {
        Self {
            lane_kind: SourceLaneKind::DrySource,
            source_id: 0,
            persistent_part_id: None,
            native_stereo_route: None,
            authored_position: None,
            foundation: 0.0,
            foreground: 0.0,
            diffuse: 0.0,
            width: 0.0,
            vertical_affinity: 0.0,
            confidence: 0.0,
        }
    }
}

/// Presentation controls for a source-native shared effect field such as the
/// SNES S-DSP echo return. This is deliberately separate from dry-object policy:
/// a historical wet field is neither another instrument nor Omniphony's own
/// listening-room reflections, and it deserves independent scale and geometry.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SharedWetPresentationPolicy {
    /// Additional wet-layer opening inside the parent source sphere.
    pub strength: f32,
    /// Rearward target angle before native L/R laterality is applied.
    pub rear_azimuth_deg: f32,
    /// Upper-field target for the environmental layer.
    pub elevation_deg: f32,
    /// Radial target in the same normalized units as dry-source distance.
    pub distance: f32,
    /// Horizontal/depth/vertical apparent extent target.
    pub extent: [f32; 3],
}

impl Default for SharedWetPresentationPolicy {
    fn default() -> Self {
        Self {
            strength: 1.0,
            rear_azimuth_deg: 138.0,
            elevation_deg: 38.0,
            distance: 1.60,
            extent: [1.0, 0.95, 0.85],
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourcePresentationPolicy {
    /// Explicit immersive-remix amount for source-aware material. Zero keeps
    /// unknown material at native laterality; one opens the full designed field.
    /// This is a presentation control, never a confidence that the historical
    /// source authored the resulting 3-D geometry.
    pub sphere_strength: f32,
    pub max_rear_azimuth_deg: f32,
    pub max_elevation_deg: f32,
    pub max_distance: f32,
    /// Shared historical wet fields get their own production layer rather than
    /// inheriting the dry-object geometry by accident.
    pub shared_wet: SharedWetPresentationPolicy,
}

impl Default for SourcePresentationPolicy {
    fn default() -> Self {
        Self {
            sphere_strength: 1.0,
            max_rear_azimuth_deg: 145.0,
            max_elevation_deg: 55.0,
            max_distance: 1.65,
            shared_wet: SharedWetPresentationPolicy::default(),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourcePresentation {
    pub render_as_object: bool,
    pub authority: SourcePositionAuthority,
    pub position: [f64; 3],
    pub size: [f32; 3],
    pub azimuth_deg: f32,
    pub elevation_deg: f32,
    pub distance: f32,
    pub route_pan: f32,
    pub rear_weight: f32,
}

fn clamp01(value: f32) -> f32 {
    if value.is_finite() { value.clamp(0.0, 1.0) } else { 0.0 }
}

fn clamp_signed(value: f32) -> f32 {
    if value.is_finite() { value.clamp(-1.0, 1.0) } else { 0.0 }
}

/// -1 = left, +1 = right. Polarity does not swap sides.
pub fn route_pan(route: Option<NativeStereoRoute>) -> f32 {
    let Some(route) = route else { return 0.0; };
    let left = route.left_gain.abs();
    let right = route.right_gain.abs();
    let sum = left + right;
    if !sum.is_finite() || sum <= 1.0e-9 {
        0.0
    } else {
        ((right - left) / sum).clamp(-1.0, 1.0)
    }
}

/// Matrix-surround-compatible phase-opposition cue derived from authored native
/// stereo routing. 0 means there is no usable opposite-polarity evidence; 1
/// means equal-magnitude L/R routes with opposite signs.
///
/// This is deliberately not called an authored 3-D position. A Dolby-style
/// matrix decoder can use phase/opposition to steer rear/surround energy, but a
/// signed device route alone does not prove a discrete source coordinate or
/// elevation. The magnitude-balance term keeps strongly one-sided inverted
/// routes from being promoted into a rear cue merely because one sign differs.
pub fn matrix_surround_phase_cue(route: Option<NativeStereoRoute>) -> f32 {
    let Some(route) = route else { return 0.0; };
    if !route.left_gain.is_finite() || !route.right_gain.is_finite() {
        return 0.0;
    }

    let left = route.left_gain.abs();
    let right = route.right_gain.abs();
    let sum = left + right;
    if left <= 1.0e-9 || right <= 1.0e-9 || sum <= 1.0e-9 {
        return 0.0;
    }
    if route.left_gain.is_sign_negative() == route.right_gain.is_sign_negative() {
        return 0.0;
    }

    (1.0 - (left - right).abs() / sum).clamp(0.0, 1.0)
}

fn presentation_identity(source: &SourceSceneEvidence) -> u64 {
    source.persistent_part_id.unwrap_or(source.source_id)
}

/// Stable deterministic presentation coordinate. A persistent musical part wins
/// over a temporary physical/source id so voice stealing does not make a part
/// jump. Different salts provide independent but repeatable spatial dimensions.
fn identity_dimension(source: &SourceSceneEvidence, salt: u64) -> f32 {
    let mut z = presentation_identity(source)
        .wrapping_add(0x9E3779B97F4A7C15)
        .wrapping_add(salt);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^= z >> 31;
    let unit = (z as u32) as f32 / u32::MAX as f32;
    unit * 2.0 - 1.0
}

fn identity_bias(source: &SourceSceneEvidence) -> f32 {
    identity_dimension(source, 0)
}

fn to_cartesian(azimuth: f32, elevation: f32, distance: f32) -> [f64; 3] {
    let (x, y, z) = spherical_to_adm(azimuth, elevation, distance);
    [x as f64, y as f64, z as f64]
}

fn present_shared_wet(
    source: SourceSceneEvidence,
    policy: SourcePresentationPolicy,
    pan: f32,
    identity: f32,
) -> SourcePresentation {
    let sphere = clamp01(policy.sphere_strength);
    let confidence = clamp01(source.confidence);
    let diffuse = clamp01(source.diffuse);
    let width = clamp01(source.width);
    let wet = policy.shared_wet;
    let wet_strength = clamp01(wet.strength);

    // Source-side presentation evidence can now trim the historical wet layer
    // without an ABI revision. SPC supplies diffuse=width=1 by default; the
    // causal soundtrack governor may lower them after observing a dense or
    // already-wet scene. The floor keeps a real shared return recognizably a
    // field rather than collapsing it into a point merely because confidence or
    // an adaptive control momentarily falls.
    let evidence_strength = (0.65 * diffuse + 0.35 * width).clamp(0.0, 1.0);
    let strength = sphere
        * wet_strength
        * (0.35 + 0.65 * evidence_strength)
        * (0.55 + 0.45 * confidence);
    let side = if pan.abs() > 0.05 {
        pan.signum()
    } else if identity.abs() > 1.0e-6 {
        identity.signum()
    } else {
        1.0
    };

    // Native L/R remains the field's historical side authority. Everything
    // beyond that is an explicit modern wet-layer production choice.
    let native_azimuth = pan * 70.0;
    let rear_limit = policy.max_rear_azimuth_deg.clamp(90.0, 179.0);
    let rear_target = side * wet.rear_azimuth_deg.clamp(90.0, rear_limit);
    let azimuth = native_azimuth + (rear_target - native_azimuth) * strength;
    let elevation_target = if wet.elevation_deg.is_finite() {
        wet.elevation_deg.clamp(-policy.max_elevation_deg.max(0.0), policy.max_elevation_deg.max(0.0))
    } else {
        0.0
    };
    let elevation = elevation_target * strength;
    let distance_target = if wet.distance.is_finite() {
        wet.distance.clamp(1.0, policy.max_distance.max(1.0))
    } else {
        1.0
    };
    let distance = 1.0 + (distance_target - 1.0) * strength;
    let extent = wet.extent.map(clamp01);
    let extent_evidence = [width, diffuse, diffuse];

    SourcePresentation {
        render_as_object: true,
        authority: SourcePositionAuthority::InferredPresentation,
        position: to_cartesian(azimuth, elevation, distance),
        size: [
            extent[0] * extent_evidence[0] * strength,
            extent[1] * extent_evidence[1] * strength,
            extent[2] * extent_evidence[2] * strength,
        ],
        azimuth_deg: azimuth,
        elevation_deg: elevation,
        distance,
        route_pan: pan,
        rear_weight: strength,
    }
}

fn inferred_presentation(
    source: SourceSceneEvidence,
    policy: SourcePresentationPolicy,
) -> SourcePresentation {
    let pan = route_pan(source.native_stereo_route);
    let matrix_surround = matrix_surround_phase_cue(source.native_stereo_route);
    let identity = identity_bias(&source);
    let depth_identity = identity_dimension(&source, 0xD1B54A32D192ED03);
    let height_identity = identity_dimension(&source, 0x94D049BB133111EB);

    if source.lane_kind == SourceLaneKind::ReferenceMix {
        return SourcePresentation {
            render_as_object: false,
            authority: SourcePositionAuthority::InferredPresentation,
            position: [0.0, 1.0, 0.0],
            size: [0.0; 3],
            azimuth_deg: 0.0,
            elevation_deg: 0.0,
            distance: 1.0,
            route_pan: pan,
            rear_weight: 0.0,
        };
    }

    if source.lane_kind == SourceLaneKind::SharedWetReturn {
        return present_shared_wet(source, policy, pan, identity);
    }

    let sphere = clamp01(policy.sphere_strength);
    let confidence = clamp01(source.confidence);
    let foundation = clamp01(source.foundation);
    let foreground = clamp01(source.foreground);
    let diffuse = clamp01(source.diffuse);
    let width = clamp01(source.width);
    let vertical = clamp_signed(source.vertical_affinity);

    // Foundation remains the anchor of the mix. Everything else is allowed to
    // inhabit a larger stable scene when the source-aware sphere is opened.
    let foundation_lock = (1.0 - foundation).powi(2);
    let evidence_movable = sphere * confidence * foundation_lock;
    let remix_movable = sphere * foundation_lock * (0.45 + 0.35 * confidence);
    let support = diffuse * (1.0 - foundation) * (1.0 - foreground);

    // Native authored left/right routing is the first constraint. When routing
    // leaves room, stable musical/source identity supplies a repeatable creative
    // spread rather than keeping every historically centered voice piled at 0°.
    let route_azimuth = pan * 70.0;
    let identity_azimuth = identity * 52.0 * remix_movable * (1.0 - pan.abs());
    let frontal_azimuth = (route_azimuth + identity_azimuth).clamp(-92.0, 92.0);

    let evidence_rear_weight = (evidence_movable
        * (0.72 * diffuse + 0.42 * support)
        * (1.0 - 0.85 * foreground))
        .clamp(0.0, 1.0);

    // A source-aware surround mix is intentionally allowed to use rear space
    // even when the old hardware never authored a rear coordinate. This remains
    // DERIVED presentation. Stable identity makes that choice repeatable rather
    // than callback-random, while foreground/foundation evidence resists it.
    let remix_rear_weight = (remix_movable
        * 0.42
        * ((depth_identity + 1.0) * 0.5)
        * (1.0 - 0.82 * foreground))
        .clamp(0.0, 1.0);

    // Opposite-polarity, magnitude-balanced native routing is authored phase
    // evidence that historically fed matrix-surround decoders. It remains a
    // presentation prior rather than a fabricated authored point.
    let matrix_rear_weight = (sphere * matrix_surround * 0.92).clamp(0.0, 1.0);
    let rear_weight = evidence_rear_weight
        .max(remix_rear_weight)
        .max(matrix_rear_weight);

    let side = if pan.abs() > 0.05 {
        pan.signum()
    } else if identity.abs() > 1.0e-6 {
        identity.signum()
    } else {
        1.0
    };
    let rear_target = side
        * (110.0 + 35.0 * identity.abs())
            .min(policy.max_rear_azimuth_deg.clamp(90.0, 179.0));
    let azimuth = frontal_azimuth + (rear_target - frontal_azimuth) * rear_weight;

    // Explicit musical vertical evidence gets first say. The immersive-remix
    // layer may also give otherwise neutral sources a bounded stable elevation,
    // creating genuine vertical scale without relabeling it as historical data.
    let vertical_context = 0.72 + 0.28 * diffuse.max(support);
    let evidence_elevation = vertical
        * policy.max_elevation_deg.clamp(0.0, 80.0)
        * evidence_movable
        * vertical_context;
    let remix_elevation = height_identity
        * policy.max_elevation_deg.clamp(0.0, 80.0)
        * 0.32
        * remix_movable
        * (1.0 - 0.72 * foreground);
    let elevation = (evidence_elevation + remix_elevation)
        .clamp(-policy.max_elevation_deg.clamp(0.0, 80.0), policy.max_elevation_deg.clamp(0.0, 80.0));

    let evidence_depth_weight = evidence_movable
        * (0.55 * support + 0.70 * diffuse)
        * (1.0 - 0.65 * foreground);
    let remix_depth_weight = remix_movable
        * 0.34
        * ((depth_identity + 1.0) * 0.5)
        * (1.0 - 0.55 * foreground);
    let depth_weight = evidence_depth_weight
        .max(remix_depth_weight)
        .max(0.30 * sphere * matrix_surround);
    let distance = 1.0
        + (policy.max_distance.max(1.0) - 1.0) * depth_weight.clamp(0.0, 1.0);

    // Width and source extent are production dimensions too. The creative base
    // gives isolated chip voices body in the immersive field; stronger source
    // evidence can enlarge or diffuse them further.
    let horizontal_size = (0.12 + 0.28 * remix_movable + 0.62 * width + 0.40 * diffuse)
        .max(0.82 * matrix_surround)
        .clamp(0.0, 1.0);
    let depth_size = (0.06 + 0.18 * remix_movable + 0.50 * diffuse + 0.24 * support)
        .max(0.55 * matrix_surround)
        .clamp(0.0, 1.0);
    let height_size = (0.04
        + 0.16 * remix_movable
        + 0.45 * diffuse
        + 0.24 * vertical.abs())
        .clamp(0.0, 1.0);
    let size_scale = (0.30 + 0.70 * remix_movable)
        .max(0.75 * matrix_surround)
        .clamp(0.0, 1.0);

    SourcePresentation {
        render_as_object: true,
        authority: SourcePositionAuthority::InferredPresentation,
        position: to_cartesian(azimuth, elevation, distance),
        size: [
            horizontal_size * size_scale,
            depth_size * size_scale,
            height_size * size_scale,
        ],
        azimuth_deg: azimuth,
        elevation_deg: elevation,
        distance,
        route_pan: pan,
        rear_weight,
    }
}

/// Source-authored geometry passes through untouched. Everything else remains
/// explicitly an Omniphony presentation decision.
pub fn present_source(
    source: SourceSceneEvidence,
    policy: SourcePresentationPolicy,
) -> SourcePresentation {
    if let Some(position) = source.authored_position {
        let x = position[0] as f32;
        let y = position[1] as f32;
        let z = position[2] as f32;
        let horizontal = (x * x + y * y).sqrt();
        let distance = (horizontal * horizontal + z * z).sqrt();
        let azimuth = x.atan2(y).to_degrees();
        let elevation = if horizontal <= 1.0e-6 {
            if z > 0.0 { 90.0 } else if z < 0.0 { -90.0 } else { 0.0 }
        } else {
            z.atan2(horizontal).to_degrees()
        };
        return SourcePresentation {
            render_as_object: source.lane_kind != SourceLaneKind::ReferenceMix,
            authority: SourcePositionAuthority::Authored,
            position,
            size: [
                clamp01(source.width),
                clamp01(source.diffuse),
                clamp01(source.diffuse),
            ],
            azimuth_deg: azimuth,
            elevation_deg: elevation,
            distance,
            route_pan: route_pan(source.native_stereo_route),
            rear_weight: 0.0,
        };
    }
    inferred_presentation(source, policy)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dry(id: u64) -> SourceSceneEvidence {
        SourceSceneEvidence {
            source_id: id,
            confidence: 1.0,
            ..SourceSceneEvidence::default()
        }
    }

    #[test]
    fn route_pan_uses_magnitude_not_polarity() {
        let positive = route_pan(Some(NativeStereoRoute { left_gain: 1.0, right_gain: 0.25 }));
        let inverted = route_pan(Some(NativeStereoRoute { left_gain: -1.0, right_gain: 0.25 }));
        assert!(positive < 0.0);
        assert_eq!(positive, inverted);
    }

    #[test]
    fn matrix_surround_phase_cue_requires_opposite_polarity_and_balance() {
        assert_eq!(
            matrix_surround_phase_cue(Some(NativeStereoRoute { left_gain: 1.0, right_gain: 1.0 })),
            0.0
        );
        assert_eq!(
            matrix_surround_phase_cue(Some(NativeStereoRoute { left_gain: -1.0, right_gain: -1.0 })),
            0.0
        );
        assert_eq!(
            matrix_surround_phase_cue(Some(NativeStereoRoute { left_gain: -1.0, right_gain: 1.0 })),
            1.0
        );
        let weak = matrix_surround_phase_cue(Some(NativeStereoRoute { left_gain: -1.0, right_gain: 0.1 }));
        assert!(weak > 0.0 && weak < 0.25);
    }

    #[test]
    fn protected_reference_mix_is_not_an_extra_object() {
        let out = present_source(
            SourceSceneEvidence { lane_kind: SourceLaneKind::ReferenceMix, ..dry(1) },
            SourcePresentationPolicy::default(),
        );
        assert!(!out.render_as_object);
    }

    #[test]
    fn foundation_remains_frontal_even_at_full_sphere() {
        let out = present_source(
            SourceSceneEvidence {
                foundation: 1.0,
                diffuse: 1.0,
                vertical_affinity: 1.0,
                native_stereo_route: Some(NativeStereoRoute { left_gain: 1.0, right_gain: 1.0 }),
                ..dry(2)
            },
            SourcePresentationPolicy::default(),
        );
        assert!(out.azimuth_deg.abs() < 1.0e-5);
        assert!(out.elevation_deg.abs() < 1.0e-5);
        assert!(out.distance <= 1.0001);
        assert!(out.rear_weight <= 1.0e-6);
    }

    #[test]
    fn authored_phase_opposition_outranks_inferred_foundation() {
        let out = present_source(
            SourceSceneEvidence {
                foundation: 1.0,
                native_stereo_route: Some(NativeStereoRoute { left_gain: -1.0, right_gain: 1.0 }),
                ..dry(20)
            },
            SourcePresentationPolicy::default(),
        );
        assert!(out.rear_weight > 0.85);
        assert!(out.azimuth_deg.abs() > 100.0);
        assert_eq!(out.elevation_deg, 0.0);
        assert!(out.size[0] > 0.5);
    }

    #[test]
    fn native_routes_bias_expected_sides() {
        let left = present_source(
            SourceSceneEvidence {
                native_stereo_route: Some(NativeStereoRoute { left_gain: 1.0, right_gain: 0.0 }),
                foreground: 1.0,
                ..dry(3)
            },
            SourcePresentationPolicy::default(),
        );
        let right = present_source(
            SourceSceneEvidence {
                native_stereo_route: Some(NativeStereoRoute { left_gain: 0.0, right_gain: 1.0 }),
                foreground: 1.0,
                ..dry(4)
            },
            SourcePresentationPolicy::default(),
        );
        assert!(left.azimuth_deg < -45.0);
        assert!(right.azimuth_deg > 45.0);
    }

    #[test]
    fn unknown_role_gets_stable_derived_immersive_space() {
        let open = present_source(dry(10), SourcePresentationPolicy::default());
        let closed = present_source(
            dry(10),
            SourcePresentationPolicy {
                sphere_strength: 0.0,
                ..SourcePresentationPolicy::default()
            },
        );
        assert_eq!(open.authority, SourcePositionAuthority::InferredPresentation);
        assert_eq!(closed.azimuth_deg, 0.0);
        assert_eq!(closed.elevation_deg, 0.0);
        assert_eq!(closed.distance, 1.0);
        assert!(open.distance > closed.distance);
        assert!(open.size[0] > closed.size[0]);
    }

    #[test]
    fn explicit_vertical_evidence_does_not_require_diffuse_support() {
        let up = present_source(
            SourceSceneEvidence {
                vertical_affinity: 0.8,
                ..dry(21)
            },
            SourcePresentationPolicy::default(),
        );
        let down = present_source(
            SourceSceneEvidence {
                vertical_affinity: -0.8,
                ..dry(22)
            },
            SourcePresentationPolicy::default(),
        );
        assert!(up.elevation_deg > 20.0);
        assert!(down.elevation_deg < -20.0);
    }

    #[test]
    fn explicit_vertical_evidence_still_obeys_sphere_and_foundation_locks() {
        let closed = present_source(
            SourceSceneEvidence {
                vertical_affinity: 1.0,
                ..dry(23)
            },
            SourcePresentationPolicy {
                sphere_strength: 0.0,
                ..SourcePresentationPolicy::default()
            },
        );
        let foundation = present_source(
            SourceSceneEvidence {
                foundation: 1.0,
                vertical_affinity: 1.0,
                ..dry(24)
            },
            SourcePresentationPolicy::default(),
        );
        assert_eq!(closed.elevation_deg, 0.0);
        assert_eq!(foundation.elevation_deg, 0.0);
    }

    #[test]
    fn affirmative_diffuse_evidence_can_use_rear_height_and_depth() {
        let out = present_source(
            SourceSceneEvidence {
                diffuse: 0.9,
                vertical_affinity: 0.8,
                width: 0.8,
                ..dry(11)
            },
            SourcePresentationPolicy::default(),
        );
        assert!(out.rear_weight > 0.0);
        assert!(out.elevation_deg > 0.0);
        assert!(out.distance > 1.0);
        assert!(out.size[0] > 0.2);
    }

    #[test]
    fn shared_wet_is_a_separately_tunable_environment_layer() {
        let source = SourceSceneEvidence {
            lane_kind: SourceLaneKind::SharedWetReturn,
            confidence: 1.0,
            diffuse: 1.0,
            width: 1.0,
            source_id: 12,
            ..SourceSceneEvidence::default()
        };
        let native = present_source(
            source,
            SourcePresentationPolicy {
                sphere_strength: 0.0,
                max_elevation_deg: 0.0,
                max_distance: 1.0,
                ..SourcePresentationPolicy::default()
            },
        );
        let full = present_source(source, SourcePresentationPolicy::default());
        let restrained = present_source(
            SourceSceneEvidence {
                diffuse: 0.45,
                width: 0.55,
                ..source
            },
            SourcePresentationPolicy {
                shared_wet: SharedWetPresentationPolicy {
                    strength: 0.4,
                    rear_azimuth_deg: 112.0,
                    elevation_deg: 18.0,
                    distance: 1.25,
                    extent: [0.55, 0.45, 0.30],
                },
                ..SourcePresentationPolicy::default()
            },
        );
        assert_eq!(native.elevation_deg, 0.0);
        assert_eq!(native.distance, 1.0);
        assert_eq!(native.size, [0.0, 0.0, 0.0]);
        assert!(full.azimuth_deg.abs() > 90.0);
        assert!(full.elevation_deg > 20.0);
        assert!(full.distance > 1.0);
        assert!(full.size[0] > full.size[2]);
        assert!(restrained.rear_weight < full.rear_weight);
        assert!(restrained.elevation_deg.abs() < full.elevation_deg.abs());
        assert!(restrained.distance < full.distance);
        assert!(restrained.size[0] < full.size[0]);
    }

    #[test]
    fn persistent_part_identity_survives_source_reassignment() {
        let a = present_source(
            SourceSceneEvidence {
                source_id: 6,
                persistent_part_id: Some(9001),
                diffuse: 0.7,
                vertical_affinity: 0.6,
                confidence: 1.0,
                ..SourceSceneEvidence::default()
            },
            SourcePresentationPolicy::default(),
        );
        let b = present_source(
            SourceSceneEvidence {
                source_id: 123456,
                persistent_part_id: Some(9001),
                diffuse: 0.7,
                vertical_affinity: 0.6,
                confidence: 1.0,
                ..SourceSceneEvidence::default()
            },
            SourcePresentationPolicy::default(),
        );
        assert_eq!(a.position, b.position);
        assert_eq!(a.size, b.size);
    }

    #[test]
    fn authored_position_passes_through_untouched() {
        let position = [-0.4, -0.7, 0.3];
        let out = present_source(
            SourceSceneEvidence {
                authored_position: Some(position),
                foundation: 1.0,
                diffuse: 1.0,
                vertical_affinity: -1.0,
                ..dry(14)
            },
            SourcePresentationPolicy::default(),
        );
        assert_eq!(out.authority, SourcePositionAuthority::Authored);
        assert_eq!(out.position, position);
    }
}
