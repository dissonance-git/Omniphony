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
    /// True when the host has already applied the source's sample-accurate
    /// native gain trajectory to the causal PCM. The route remains authoritative
    /// pose/polarity evidence but must not attenuate the PCM a second time.
    pub route_gain_preapplied: bool,
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
            route_gain_preapplied: false,
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
    let elevation = 40.0_f32
        .min(policy.max_elevation_deg.max(0.0))
        * strength;
    let distance = 1.0
        + (policy.max_distance.max(1.0) - 1.0) * strength;

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
    let identity = identity_bias(&source);

    if source.lane_kind == SourceLaneKind::ReferenceMix {
        return SourcePresentation {
            render_as_object: false,
            authority: SourcePositionAuthority::InferredPresentation,
            position: [0.0, 0.0, 1.0],
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

    // Native routing remains a side constraint. Identity only provides a small,
    // stable separation for sources whose native route is balanced/unknown.
    let native_azimuth = pan * 70.0;
    let stable_offset = identity * 18.0 * (1.0 - pan.abs());
    let frontal_anchor = (0.75 * foundation + 0.35 * foreground).clamp(0.0, 1.0);

    // Rear placement is earned by positive diffuse/support evidence. Missing
    // foreground/foundation evidence is not itself support evidence.
    let support = (0.70 * diffuse + 0.30 * width).clamp(0.0, 1.0);
    let rear_weight = sphere
        * confidence
        * support
        * (1.0 - 0.85 * frontal_anchor)
        * (0.55 + 0.45 * (1.0 - pan.abs()));

    let side_sign = if pan.abs() > 0.05 {
        pan.signum()
    } else if identity.abs() > 1.0e-6 {
        identity.signum()
    } else {
        1.0
    };
    let rear_target = side_sign
        * policy
            .max_rear_azimuth_deg
            .clamp(90.0, 179.0)
        * (0.55 + 0.45 * support);
    let front_target = (native_azimuth + stable_offset).clamp(-80.0, 80.0);
    let azimuth = front_target + (rear_target - front_target) * rear_weight;

    // Foundation sinks slightly and remains frontal. Positive upper/register
    // evidence can earn height; diffuse support can also lift modestly.
    let upward = (vertical.max(0.0) * 0.75 + diffuse * 0.25)
        * (1.0 - 0.75 * foundation);
    let downward = foundation * (0.18 + 0.12 * (-vertical).max(0.0));
    let elevation = ((upward - downward)
        * sphere
        * confidence
        * policy.max_elevation_deg.max(0.0))
        .clamp(-18.0, policy.max_elevation_deg.max(0.0));

    let distance_push = (0.55 * diffuse + 0.25 * width + 0.20 * rear_weight)
        * (1.0 - 0.65 * foundation);
    let distance = 1.0
        + (policy.max_distance.max(1.0) - 1.0)
            * sphere
            * confidence
            * distance_push;

    SourcePresentation {
        render_as_object: true,
        authority: SourcePositionAuthority::InferredPresentation,
        position: to_cartesian(azimuth, elevation, distance),
        size: [width, diffuse, diffuse],
        azimuth_deg: azimuth,
        elevation_deg: elevation,
        distance,
        route_pan: pan,
        rear_weight,
    }
}

pub fn present_source(
    source: SourceSceneEvidence,
    policy: SourcePresentationPolicy,
) -> SourcePresentation {
    if let Some(position) = source.authored_position {
        return SourcePresentation {
            render_as_object: source.lane_kind != SourceLaneKind::ReferenceMix,
            authority: SourcePositionAuthority::Authored,
            position,
            size: [clamp01(source.width); 3],
            azimuth_deg: 0.0,
            elevation_deg: 0.0,
            distance: 1.0,
            route_pan: route_pan(source.native_stereo_route),
            rear_weight: 0.0,
        };
    }
    inferred_presentation(source, policy)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn route_pan_uses_magnitude_not_polarity() {
        assert_eq!(route_pan(Some(NativeStereoRoute { left_gain: -1.0, right_gain: 0.0 })), -1.0);
        assert_eq!(route_pan(Some(NativeStereoRoute { left_gain: 0.0, right_gain: -1.0 })), 1.0);
    }

    #[test]
    fn reference_mix_is_never_promoted_to_object() {
        let source = SourceSceneEvidence {
            lane_kind: SourceLaneKind::ReferenceMix,
            ..SourceSceneEvidence::default()
        };
        assert!(!present_source(source, SourcePresentationPolicy::default()).render_as_object);
    }

    #[test]
    fn foundation_stays_frontal_and_low() {
        let source = SourceSceneEvidence {
            source_id: 10,
            native_stereo_route: Some(NativeStereoRoute { left_gain: 1.0, right_gain: 1.0 }),
            foundation: 1.0,
            confidence: 1.0,
            ..SourceSceneEvidence::default()
        };
        let p = present_source(source, SourcePresentationPolicy::default());
        assert!(p.azimuth_deg.abs() < 10.0);
        assert!(p.elevation_deg <= 0.0);
        assert!(p.distance <= 1.1);
    }

    #[test]
    fn positive_diffuse_evidence_can_earn_rear_and_height() {
        let source = SourceSceneEvidence {
            source_id: 21,
            diffuse: 1.0,
            width: 1.0,
            vertical_affinity: 0.7,
            confidence: 1.0,
            ..SourceSceneEvidence::default()
        };
        let p = present_source(source, SourcePresentationPolicy::default());
        assert!(p.azimuth_deg.abs() > 80.0);
        assert!(p.elevation_deg > 0.0);
        assert!(p.distance > 1.0);
    }

    #[test]
    fn missing_role_evidence_does_not_become_support_evidence() {
        let source = SourceSceneEvidence {
            source_id: 22,
            confidence: 1.0,
            ..SourceSceneEvidence::default()
        };
        let p = present_source(source, SourcePresentationPolicy::default());
        assert!(p.azimuth_deg.abs() <= 18.0);
        assert_eq!(p.rear_weight, 0.0);
        assert_eq!(p.distance, 1.0);
    }

    #[test]
    fn persistent_part_controls_stable_identity_position() {
        let a = SourceSceneEvidence {
            source_id: 1,
            persistent_part_id: Some(777),
            confidence: 1.0,
            ..SourceSceneEvidence::default()
        };
        let b = SourceSceneEvidence {
            source_id: 999,
            persistent_part_id: Some(777),
            confidence: 1.0,
            ..SourceSceneEvidence::default()
        };
        assert_eq!(present_source(a, SourcePresentationPolicy::default()).position,
                   present_source(b, SourcePresentationPolicy::default()).position);
    }

    #[test]
    fn authored_position_passes_through_unchanged() {
        let source = SourceSceneEvidence {
            source_id: 1,
            authored_position: Some([0.25, -0.5, 1.0]),
            ..SourceSceneEvidence::default()
        };
        let p = present_source(source, SourcePresentationPolicy::default());
        assert_eq!(p.authority, SourcePositionAuthority::Authored);
        assert_eq!(p.position, [0.25, -0.5, 1.0]);
    }
}
