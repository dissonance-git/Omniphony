//! Source-aware presentation policy for already-separated causal sources.
//!
//! GMI owns source truth. This module chooses only an Omniphony presentation.
//! Native routing constrains presentation but never becomes fake authored 3-D.

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

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SourcePresentationPolicy {
    pub sphere_strength: f32,
    pub max_rear_azimuth_deg: f32,
    pub max_elevation_deg: f32,
    pub max_distance: f32,
}

impl Default for SourcePresentationPolicy {
    fn default() -> Self {
        Self {
            sphere_strength: 1.0,
            max_rear_azimuth_deg: 145.0,
            max_elevation_deg: 55.0,
            max_distance: 1.65,
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
    if value.is_finite() {
        value.clamp(0.0, 1.0)
    } else {
        0.0
    }
}

fn clamp_signed(value: f32) -> f32 {
    if value.is_finite() {
        value.clamp(-1.0, 1.0)
    } else {
        0.0
    }
}

/// -1 = left, +1 = right. Polarity does not swap sides.
pub fn route_pan(route: Option<NativeStereoRoute>) -> f32 {
    let Some(route) = route else {
        return 0.0;
    };
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
    let Some(route) = route else {
        return 0.0;
    };
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

/// Stable deterministic coordinate. A persistent musical part wins over a
/// temporary physical/source id so voice stealing does not make a part jump.
fn identity_bias(source: &SourceSceneEvidence) -> f32 {
    let id = source.persistent_part_id.unwrap_or(source.source_id);
    let mut z = id.wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^= z >> 31;
    let unit = (z as u32) as f32 / u32::MAX as f32;
    unit * 2.0 - 1.0
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
    let strength = sphere * (0.55 + 0.45 * confidence);
    let side = if pan.abs() > 0.05 {
        pan.signum()
    } else if identity.abs() > 1.0e-6 {
        identity.signum()
    } else {
        1.0
    };

    // With sphere_strength=0, preserve only native laterality. Rear, height and
    // extra distance appear only as the sphere is deliberately opened.
    let native_azimuth = pan * 70.0;
    let rear_target = side * 135.0_f32.min(policy.max_rear_azimuth_deg.clamp(90.0, 179.0));
    let azimuth = native_azimuth + (rear_target - native_azimuth) * strength;
    let elevation = 40.0_f32.min(policy.max_elevation_deg.max(0.0)) * strength;
    let distance = 1.0 + (policy.max_distance.max(1.0) - 1.0) * strength;

    SourcePresentation {
        render_as_object: true,
        authority: SourcePositionAuthority::InferredPresentation,
        position: to_cartesian(azimuth, elevation, distance),
        size: [1.0, 1.0, 1.0],
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

    // Foundation is intentionally difficult to dislodge. Confidence that a
    // source is foundational must not become confidence that it should move.
    let movable = sphere * confidence * (1.0 - foundation).powi(2);

    // Crucial evidence law: absence of foreground/foundation labels is NOT
    // positive support evidence. Rear/depth placement requires an affirmative
    // diffuse/support cue. This keeps unknown centered sources conservative.
    let support = diffuse * (1.0 - foundation) * (1.0 - foreground);

    let route_azimuth = pan * 70.0;
    let identity_azimuth = identity * 28.0 * movable * (1.0 - pan.abs());
    let frontal_azimuth = (route_azimuth + identity_azimuth).clamp(-78.0, 78.0);

    let inferred_rear_weight =
        (movable * (0.72 * diffuse + 0.42 * support) * (1.0 - 0.85 * foreground)).clamp(0.0, 1.0);

    // Opposite-polarity, magnitude-balanced native routing is authored phase
    // evidence that historically fed matrix-surround decoders. It outranks our
    // inferred role classifier, including an inferred "foundation" label, but
    // remains a presentation prior rather than a fabricated authored point.
    let matrix_rear_weight = (sphere * matrix_surround * 0.92).clamp(0.0, 1.0);
    let rear_weight = inferred_rear_weight.max(matrix_rear_weight);

    let side = if pan.abs() > 0.05 {
        pan.signum()
    } else if identity.abs() > 1.0e-6 {
        identity.signum()
    } else {
        1.0
    };
    let rear_target =
        side * (110.0 + 35.0 * identity.abs()).min(policy.max_rear_azimuth_deg.clamp(90.0, 179.0));
    let azimuth = frontal_azimuth + (rear_target - frontal_azimuth) * rear_weight;

    // Matrix-surround phase evidence still carries no vertical coordinate.
    // Explicit signed vertical_affinity, however, IS positive presentation
    // evidence and must not require a second diffuse/support classifier to earn
    // most of its height. Diffuse/support can strengthen the elevation, while
    // sphere strength, confidence and the foundation lock remain authoritative.
    let vertical_context = 0.72 + 0.28 * diffuse.max(support);
    let elevation =
        vertical * policy.max_elevation_deg.clamp(0.0, 80.0) * movable * vertical_context;

    let inferred_depth_weight =
        movable * (0.55 * support + 0.70 * diffuse) * (1.0 - 0.65 * foreground);
    let depth_weight = inferred_depth_weight.max(0.30 * sphere * matrix_surround);
    let distance = 1.0 + (policy.max_distance.max(1.0) - 1.0) * depth_weight.clamp(0.0, 1.0);

    let horizontal_size = (0.08 + 0.72 * width + 0.45 * diffuse)
        .max(0.82 * matrix_surround)
        .clamp(0.0, 1.0);
    let depth_size = (0.05 + 0.55 * diffuse + 0.25 * support)
        .max(0.55 * matrix_surround)
        .clamp(0.0, 1.0);
    let height_size = (0.03 + 0.50 * diffuse + 0.25 * vertical.abs()).clamp(0.0, 1.0);
    // An authored matrix cue must not disappear simply because inferred role
    // evidence says the source is immovable.
    let size_scale = (0.20 + 0.80 * movable)
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
            if z > 0.0 {
                90.0
            } else if z < 0.0 {
                -90.0
            } else {
                0.0
            }
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
        let positive = route_pan(Some(NativeStereoRoute {
            left_gain: 1.0,
            right_gain: 0.25,
        }));
        let inverted = route_pan(Some(NativeStereoRoute {
            left_gain: -1.0,
            right_gain: 0.25,
        }));
        assert!(positive < 0.0);
        assert_eq!(positive, inverted);
    }

    #[test]
    fn matrix_surround_phase_cue_requires_opposite_polarity_and_balance() {
        assert_eq!(
            matrix_surround_phase_cue(Some(NativeStereoRoute {
                left_gain: 1.0,
                right_gain: 1.0
            })),
            0.0
        );
        assert_eq!(
            matrix_surround_phase_cue(Some(NativeStereoRoute {
                left_gain: -1.0,
                right_gain: -1.0
            })),
            0.0
        );
        assert_eq!(
            matrix_surround_phase_cue(Some(NativeStereoRoute {
                left_gain: -1.0,
                right_gain: 1.0
            })),
            1.0
        );
        let weak = matrix_surround_phase_cue(Some(NativeStereoRoute {
            left_gain: -1.0,
            right_gain: 0.1,
        }));
        assert!(weak > 0.0 && weak < 0.25);
    }

    #[test]
    fn protected_reference_mix_is_not_an_extra_object() {
        let out = present_source(
            SourceSceneEvidence {
                lane_kind: SourceLaneKind::ReferenceMix,
                ..dry(1)
            },
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
                native_stereo_route: Some(NativeStereoRoute {
                    left_gain: 1.0,
                    right_gain: 1.0,
                }),
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
                native_stereo_route: Some(NativeStereoRoute {
                    left_gain: -1.0,
                    right_gain: 1.0,
                }),
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
                native_stereo_route: Some(NativeStereoRoute {
                    left_gain: 1.0,
                    right_gain: 0.0,
                }),
                foreground: 1.0,
                ..dry(3)
            },
            SourcePresentationPolicy::default(),
        );
        let right = present_source(
            SourceSceneEvidence {
                native_stereo_route: Some(NativeStereoRoute {
                    left_gain: 0.0,
                    right_gain: 1.0,
                }),
                foreground: 1.0,
                ..dry(4)
            },
            SourcePresentationPolicy::default(),
        );
        assert!(left.azimuth_deg < -45.0);
        assert!(right.azimuth_deg > 45.0);
    }

    #[test]
    fn unknown_role_does_not_become_rear_support_by_absence() {
        let out = present_source(dry(10), SourcePresentationPolicy::default());
        assert_eq!(out.rear_weight, 0.0);
        assert_eq!(out.distance, 1.0);
        assert_eq!(out.elevation_deg, 0.0);
        assert!(out.azimuth_deg.abs() <= 28.0);
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
        assert!(up.elevation_deg > 30.0);
        assert!(down.elevation_deg < -30.0);
        assert_eq!(up.rear_weight, 0.0);
        assert_eq!(up.distance, 1.0);
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
    fn shared_wet_is_broad_environment_only_when_sphere_opens() {
        let source = SourceSceneEvidence {
            lane_kind: SourceLaneKind::SharedWetReturn,
            confidence: 1.0,
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
        assert_eq!(native.elevation_deg, 0.0);
        assert_eq!(native.distance, 1.0);
        assert!(full.azimuth_deg.abs() > 90.0);
        assert!(full.elevation_deg > 20.0);
        assert!(full.distance > 1.0);
        assert_eq!(full.size, [1.0, 1.0, 1.0]);
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
