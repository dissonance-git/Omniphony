//! Source-aware presentation policy for causal game-music/object lanes.
//!
//! This module begins **after** a decoder/interpreter has already separated
//! causal sources. It therefore does not run stereo source-separation or claim
//! that renderer-chosen positions were authored by the source format.
//!
//! Design boundary:
//!
//! ```text
//! source truth / persistent identity / native routing
//!                 ↓
//!        source_scene (presentation)
//!                 ↓
//!       SpatialChannelEvent / VBAP
//! ```
//!
//! Native signed stereo gains constrain left/right by their magnitudes. Their
//! signs remain phase/polarity evidence and never flip the perceived side by
//! themselves. A persistent musical part owns a stable presentation identity
//! across physical-voice reassignment. Shared wet returns are environmental
//! fields, not fake per-instrument reverb stems.

use crate::spatial_vbap::spherical_to_adm;

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourceLaneKind {
    /// Causal dry/source contribution where independent audio exists.
    DrySource,
    /// One shared feedback/reverb/room return preserved by the source system.
    SharedWetReturn,
    /// Protected historical/reference mix. It is a control, not another object.
    ReferenceMix,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SourcePositionAuthority {
    /// Position came from source metadata and must pass through unchanged.
    Authored,
    /// Position was selected by Omniphony presentation policy.
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
    /// Runtime source identity. Physical resource reuse may change this.
    pub source_id: u64,
    /// Stable musical-part identity, when GMI has earned one.
    pub persistent_part_id: Option<u64>,
    /// Native source/device routing. Signed gains are preserved as evidence;
    /// presentation laterality uses magnitudes only.
    pub native_stereo_route: Option<NativeStereoRoute>,
    /// Source-authored Cartesian ADM position, if the source truly carries one.
    pub authored_position: Option<[f64; 3]>,
    /// Continuous role/evidence weights in [0,1]. These are presentation inputs,
    /// not claims that a format stored a semantic label.
    pub foundation: f32,
    pub foreground: f32,
    pub diffuse: f32,
    pub width: f32,
    /// Signed vertical evidence/policy affinity in [-1,1]. Zero means no earned
    /// preference. Positive is above, negative below.
    pub vertical_affinity: f32,
    /// Confidence that the persistent/role evidence is safe enough to use.
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
    /// 0 = conservative frontal scene, 1 = use the full earned sphere.
    pub sphere_strength: f32,
    /// Maximum inferred rear azimuth magnitude in degrees.
    pub max_rear_azimuth_deg: f32,
    /// Maximum inferred absolute elevation in degrees for dry sources.
    pub max_elevation_deg: f32,
    /// Maximum distance used for diffuse/support material.
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
    /// False for the protected reference mix. It remains available as a control
    /// but is not rendered as an additional object on top of its own sources.
    pub render_as_object: bool,
    pub authority: SourcePositionAuthority,
    pub position: [f64; 3],
    /// Object width/depth/height in the renderer's normalised extent language.
    pub size: [f32; 3],
    /// Diagnostic values used by fixtures and future scene smoothing.
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

/// Laterality encoded by native stereo gain magnitudes.
///
/// -1 = fully left, +1 = fully right, 0 = balanced/unknown. Polarity inversion
/// does not swap sides.
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

/// Small deterministic identity coordinate. Persistent musical identity wins
/// over runtime source/physical identity so a melody does not teleport when the
/// driver reallocates it to another voice.
fn identity_bias(source: &SourceSceneEvidence) -> f32 {
    let id = source.persistent_part_id.unwrap_or(source.source_id);
    // SplitMix64 finaliser. We only need a stable, cheap distribution here, not
    // cryptographic randomness.
    let mut z = id.wrapping_add(0x9E3779B97F4A7C15);
    z = (z ^ (z >> 30)).wrapping_mul(0xBF58476D1CE4E5B9);
    z = (z ^ (z >> 27)).wrapping_mul(0x94D049BB133111EB);
    z ^= z >> 31;
    let unit = (z as u32) as f32 / u32::MAX as f32;
    unit * 2.0 - 1.0
}

fn inferred_presentation(
    source: SourceSceneEvidence,
    policy: SourcePresentationPolicy,
) -> SourcePresentation {
    let sphere = clamp01(policy.sphere_strength);
    let confidence = clamp01(source.confidence);
    let foundation = clamp01(source.foundation);
    let foreground = clamp01(source.foreground);
    let diffuse = clamp01(source.diffuse);
    let width = clamp01(source.width);
    let vertical = clamp_signed(source.vertical_affinity);
    let pan = route_pan(source.native_stereo_route);
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

    // Shared wet state represents one environmental system. It is intentionally
    // broad and may occupy the rear/upper field, but does not inherit the point
    // position of any individual source that feeds it.
    if source.lane_kind == SourceLaneKind::SharedWetReturn {
        let side = if pan.abs() > 0.05 {
            pan.signum()
        } else if identity == 0.0 {
            1.0
        } else {
            identity.signum()
        };
        let strength = sphere * (0.55 + 0.45 * confidence);
        let azimuth = side * (95.0 + 40.0 * strength);
        let elevation = (18.0 + 22.0 * strength).min(policy.max_elevation_deg.max(0.0));
        let distance = 1.15 + (policy.max_distance.max(1.15) - 1.15) * strength;
        let (x, y, z) = spherical_to_adm(azimuth, elevation, distance);
        return SourcePresentation {
            render_as_object: true,
            authority: SourcePositionAuthority::InferredPresentation,
            position: [x as f64, y as f64, z as f64],
            size: [1.0, 1.0, 1.0],
            azimuth_deg: azimuth,
            elevation_deg: elevation,
            distance,
            route_pan: pan,
            rear_weight: strength,
        };
    }

    // Foundation protection is deliberately stronger than generic confidence.
    // A source cannot earn spectacular movement merely because its classifier is
    // certain that it is the bass/foundation.
    let movable = sphere * confidence * (1.0 - foundation).powi(2);
    let support = (1.0 - foundation.max(foreground)).clamp(0.0, 1.0);

    // Native route dominates the side decision. Stable identity contributes
    // only when the native route is balanced/absent, avoiding a stack of truly
    // centered sources occupying exactly one point while remaining deterministic.
    let route_azimuth = pan * 70.0;
    let identity_azimuth = identity * 28.0 * movable * (1.0 - pan.abs());
    let frontal_azimuth = (route_azimuth + identity_azimuth).clamp(-78.0, 78.0);

    // Rear placement is a presentation decision earned by non-foundational,
    // non-foreground support/diffuse evidence. Foreground stays in the front
    // hemisphere even when the overall sphere is at maximum.
    let rear_weight = (movable
        * (0.72 * diffuse + 0.42 * support)
        * (1.0 - 0.85 * foreground))
        .clamp(0.0, 1.0);
    let side = if pan.abs() > 0.05 {
        pan.signum()
    } else if identity == 0.0 {
        1.0
    } else {
        identity.signum()
    };
    let rear_target = side
        * (110.0 + 35.0 * identity.abs())
            .min(policy.max_rear_azimuth_deg.clamp(90.0, 179.0));
    let azimuth = frontal_azimuth + (rear_target - frontal_azimuth) * rear_weight;

    // Height/lower-sphere placement requires an explicit signed affinity. The
    // policy can be full-strength without manufacturing vertical evidence from
    // physical voice number or spectral brightness alone.
    let elevation = vertical
        * policy.max_elevation_deg.clamp(0.0, 80.0)
        * movable
        * (0.25 + 0.75 * (diffuse.max(support)));

    // Foreground remains intimate; support/diffuse sources can recede. This is
    // depth presentation, not historical distance reconstruction.
    let depth_weight = movable * (0.55 * support + 0.70 * diffuse) * (1.0 - 0.65 * foreground);
    let distance = 1.0 + (policy.max_distance.max(1.0) - 1.0) * depth_weight.clamp(0.0, 1.0);

    let horizontal_size = (0.08 + 0.72 * width + 0.45 * diffuse).clamp(0.0, 1.0);
    let depth_size = (0.05 + 0.55 * diffuse + 0.25 * support).clamp(0.0, 1.0);
    let height_size = (0.03 + 0.50 * diffuse + 0.25 * vertical.abs()).clamp(0.0, 1.0);
    let size_scale = (0.20 + 0.80 * movable).clamp(0.0, 1.0);
    let size = [
        horizontal_size * size_scale,
        depth_size * size_scale,
        height_size * size_scale,
    ];

    let (x, y, z) = spherical_to_adm(azimuth, elevation, distance);
    SourcePresentation {
        render_as_object: true,
        authority: SourcePositionAuthority::InferredPresentation,
        position: [x as f64, y as f64, z as f64],
        size,
        azimuth_deg: azimuth,
        elevation_deg: elevation,
        distance,
        route_pan: pan,
        rear_weight,
    }
}

/// Convert causal source evidence into a renderer presentation decision.
/// Source-authored geometry always bypasses the inferred placement law.
pub fn present_source(
    source: SourceSceneEvidence,
    policy: SourcePresentationPolicy,
) -> SourcePresentation {
    if let Some(position) = source.authored_position {
        let x = position[0] as f32;
        let y = position[1] as f32;
        let z = position[2] as f32;
        let distance = (x * x + y * y + z * z).sqrt();
        let horizontal = (x * x + y * y).sqrt();
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
            size: [clamp01(source.width), clamp01(source.diffuse), clamp01(source.diffuse)],
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
    fn protected_reference_mix_is_not_an_extra_object() {
        let source = SourceSceneEvidence {
            lane_kind: SourceLaneKind::ReferenceMix,
            ..dry(1)
        };
        let out = present_source(source, SourcePresentationPolicy::default());
        assert!(!out.render_as_object);
    }

    #[test]
    fn foundation_remains_frontal_even_at_full_sphere() {
        let source = SourceSceneEvidence {
            foundation: 1.0,
            diffuse: 1.0,
            vertical_affinity: 1.0,
            native_stereo_route: Some(NativeStereoRoute {
                left_gain: 1.0,
                right_gain: 1.0,
            }),
            ..dry(2)
        };
        let out = present_source(source, SourcePresentationPolicy::default());
        assert!(out.azimuth_deg.abs() < 1.0e-5);
        assert!(out.elevation_deg.abs() < 1.0e-5);
        assert!(out.distance <= 1.0001);
        assert!(out.rear_weight <= 1.0e-6);
    }

    #[test]
    fn native_left_and_right_routes_bias_the_expected_sides() {
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
    fn shared_wet_return_becomes_a_broad_rear_height_field() {
        let out = present_source(
            SourceSceneEvidence {
                lane_kind: SourceLaneKind::SharedWetReturn,
                diffuse: 1.0,
                confidence: 1.0,
                source_id: 5,
                ..SourceSceneEvidence::default()
            },
            SourcePresentationPolicy::default(),
        );
        assert!(out.render_as_object);
        assert!(out.azimuth_deg.abs() > 90.0);
        assert!(out.elevation_deg > 20.0);
        assert!(out.distance > 1.1);
        assert_eq!(out.size, [1.0, 1.0, 1.0]);
    }

    #[test]
    fn persistent_part_identity_survives_physical_source_reassignment() {
        let a = present_source(
            SourceSceneEvidence {
                persistent_part_id: Some(9001),
                diffuse: 0.7,
                vertical_affinity: 0.6,
                ..dry(6)
            },
            SourcePresentationPolicy::default(),
        );
        let b = present_source(
            SourceSceneEvidence {
                source_id: 123456,
                persistent_part_id: Some(9001),
                confidence: 1.0,
                diffuse: 0.7,
                vertical_affinity: 0.6,
                ..SourceSceneEvidence::default()
            },
            SourcePresentationPolicy::default(),
        );
        assert_eq!(a.position, b.position);
        assert_eq!(a.size, b.size);
    }

    #[test]
    fn physical_source_identity_can_separate_unidentified_balanced_sources() {
        let a = present_source(dry(10), SourcePresentationPolicy::default());
        let b = present_source(dry(11), SourcePresentationPolicy::default());
        assert_ne!(a.azimuth_deg, b.azimuth_deg);
        assert!(a.azimuth_deg.abs() <= 28.0);
        assert!(b.azimuth_deg.abs() <= 28.0);
    }

    #[test]
    fn stronger_sphere_expands_support_but_not_foundation() {
        let support = SourceSceneEvidence {
            diffuse: 0.8,
            vertical_affinity: 0.8,
            width: 0.7,
            ..dry(12)
        };
        let conservative = present_source(
            support,
            SourcePresentationPolicy {
                sphere_strength: 0.2,
                ..SourcePresentationPolicy::default()
            },
        );
        let full = present_source(support, SourcePresentationPolicy::default());
        assert!(full.rear_weight > conservative.rear_weight);
        assert!(full.elevation_deg.abs() > conservative.elevation_deg.abs());
        assert!(full.distance > conservative.distance);

        let foundation = SourceSceneEvidence {
            foundation: 1.0,
            ..dry(13)
        };
        let low = present_source(
            foundation,
            SourcePresentationPolicy {
                sphere_strength: 0.2,
                ..SourcePresentationPolicy::default()
            },
        );
        let high = present_source(foundation, SourcePresentationPolicy::default());
        assert_eq!(low.position, high.position);
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
