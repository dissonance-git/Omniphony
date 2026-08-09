//! Conservative scene-evidence synthesis for ordinary stereo music.
//!
//! This module sits between low-level [`crate::stereo_inference`] measurements
//! and future libaural-aware object/field tracking. It deliberately does **not**
//! claim that stereo acoustics can reveal whether a source was physically in
//! front of or behind the listener. A stereo master normally contains no such
//! ground truth.
//!
//! Instead it answers a safer product question:
//!
//! > Which parts of the mixture look like stable frontal anchors, persistent
//! > lateral object candidates, broad sources, or diffuse fields, and how safe
//! > would it be for a later policy to spatially reassign them?
//!
//! `reassignment_safety` is therefore **not rear evidence**. Rear placement must
//! remain a separate renderer/music-policy decision, ideally informed by
//! libaural object identity, masking, musical role, section context and human
//! validation.

use crate::stereo_inference::{
    StereoBinEstimate, TrackedStereoEvidence, stable_lateral_object_score,
};

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SceneEvidenceKind {
    /// Coherent center or source-like low-frequency foundation that should remain authoritative.
    FrontalAnchor,
    /// Persistent, source-like lateral evidence that may support a discrete object.
    LateralObjectCandidate,
    /// Energy with some width/extent but insufficient evidence for a point object.
    BroadSource,
    /// Field-like / decorrelated energy better represented as ambience or room.
    DiffuseField,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SpatialSpecificity {
    /// Preserve the existing stereo presentation; evidence is weak or foundational.
    PreserveStereo,
    /// Wider/extended placement may be justified, but avoid a hard point source.
    Broad,
    /// Evidence is stable enough that a later policy may choose a specific position.
    Specific,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneEvidenceInput {
    pub frequency_hz: f32,
    pub estimate: StereoBinEstimate,
    pub tracked: TrackedStereoEvidence,
    pub magnitude: f32,
    pub reference_magnitude: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct SceneCandidateEvidence {
    pub kind: SceneEvidenceKind,
    pub specificity: SpatialSpecificity,
    pub pan: f32,
    pub lateral_strength: f32,
    pub side_fraction: f32,
    pub object_support: f32,
    pub field_support: f32,
    /// Pure frequency prior used to discourage aggressive low-frequency movement.
    pub bass_anchor: f32,
    /// Evidence that low-frequency energy behaves like a coherent musical foundation.
    pub foundation_support: f32,
    /// Product-policy evidence that spatial reassignment may be safe.
    /// This is explicitly not evidence that the source belongs behind the listener.
    pub reassignment_safety: f32,
}

/// Smooth low-frequency protection law.
///
/// Below ~80 Hz the groove/fundamental floor is strongly protected from
/// aggressive spatial reassignment. Between 80–220 Hz the protection fades
/// continuously. Above 220 Hz this particular prior contributes nothing.
///
/// This value is **not an object classifier**. Diffuse low-frequency room or
/// texture energy can have a high bass-protection value without becoming a
/// `FrontalAnchor`.
pub fn bass_anchor_weight(frequency_hz: f32) -> f32 {
    if !frequency_hz.is_finite() || frequency_hz <= 0.0 {
        return 1.0;
    }
    if frequency_hz <= 80.0 {
        return 1.0;
    }
    if frequency_hz >= 220.0 {
        return 0.0;
    }

    let t = ((frequency_hz - 80.0) / 140.0).clamp(0.0, 1.0);
    // Smoothstep, inverted: no abrupt spectral routing boundary.
    1.0 - t * t * (3.0 - 2.0 * t)
}

pub fn infer_scene_evidence(input: SceneEvidenceInput) -> SceneCandidateEvidence {
    let estimate = input.estimate;
    let tracked = input.tracked;

    let lateral_strength = ((tracked.pan.abs() - 0.12) / 0.88).clamp(0.0, 1.0);
    let ms_sum = estimate.mid_magnitude + estimate.side_magnitude;
    let side_fraction = if ms_sum > 1.0e-12 {
        (estimate.side_magnitude / ms_sum).clamp(0.0, 1.0)
    } else {
        0.0
    };

    let object_support = stable_lateral_object_score(
        tracked,
        input.magnitude,
        input.reference_magnitude,
    );

    // Diffuse evidence should become stronger when both the directness model and
    // true complex M/S structure say the energy is field-like.
    let field_support = (estimate.diffuseness * (0.35 + 0.65 * side_fraction))
        .clamp(0.0, 1.0);

    let bass_anchor = bass_anchor_weight(input.frequency_hz);
    let coherent_center = (1.0 - tracked.pan.abs()).clamp(0.0, 1.0)
        * tracked.directness
        * tracked.persistence;

    // Frequency alone cannot establish musical role. Low-frequency energy only
    // becomes foundation evidence when it is also persistent, source-like and
    // not better explained as a diffuse field.
    let foundation_support = (bass_anchor
        * tracked.directness
        * tracked.persistence
        * (1.0 - field_support))
        .clamp(0.0, 1.0);

    let kind = if coherent_center > 0.62 || foundation_support > 0.55 {
        SceneEvidenceKind::FrontalAnchor
    } else if field_support > 0.55 && field_support > object_support * 1.15 {
        SceneEvidenceKind::DiffuseField
    } else if object_support > 0.28 && tracked.persistence > 0.45 {
        SceneEvidenceKind::LateralObjectCandidate
    } else {
        SceneEvidenceKind::BroadSource
    };

    // Spatial reassignment must pay a penalty for low-frequency/foundation
    // content and for field-like evidence. It rewards persistent, lateral,
    // direct evidence. The bass penalty applies even when low-frequency energy
    // is diffuse: "do not smear the low end" is a presentation safeguard, not
    // a claim that every bass component is one frontal object.
    let reassignment_safety = (object_support
        * lateral_strength
        * (1.0 - bass_anchor)
        * (1.0 - 0.65 * field_support))
        .clamp(0.0, 1.0);

    // The upstream object-support score intentionally maps a reference-level
    // component to roughly half-scale before lateral/persistence penalties. A
    // >0.5 gate would therefore make Specific unreachable for ordinary
    // reference-level material. 0.30 remains conservative while allowing a
    // mature, strongly lateral, source-like component to become eligible.
    let specificity = if reassignment_safety > 0.30
        && matches!(kind, SceneEvidenceKind::LateralObjectCandidate)
    {
        SpatialSpecificity::Specific
    } else if matches!(kind, SceneEvidenceKind::BroadSource | SceneEvidenceKind::DiffuseField)
        || side_fraction > 0.30
    {
        SpatialSpecificity::Broad
    } else {
        SpatialSpecificity::PreserveStereo
    };

    SceneCandidateEvidence {
        kind,
        specificity,
        pan: tracked.pan,
        lateral_strength,
        side_fraction,
        object_support,
        field_support,
        bass_anchor,
        foundation_support,
        reassignment_safety,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::stereo_inference::{
        StereoBinEvidence, StereoEvidenceTracker, StereoInferenceParams, estimate_bin,
    };
    use std::f32::consts::PI;

    fn estimate(l: f32, r: f32, lp: f32, rp: f32) -> StereoBinEstimate {
        estimate_bin(
            StereoBinEvidence {
                left_magnitude: l,
                right_magnitude: r,
                left_phase: lp,
                right_phase: rp,
            },
            StereoInferenceParams::default(),
        )
    }

    fn mature(e: StereoBinEstimate) -> TrackedStereoEvidence {
        let mut tracker = StereoEvidenceTracker::default();
        let mut out = tracker.update(e, 10.0, 120.0);
        for _ in 1..40 {
            out = tracker.update(e, 10.0, 120.0);
        }
        out
    }

    #[test]
    fn centered_coherent_material_remains_a_frontal_anchor() {
        let e = estimate(1.0, 1.0, 0.0, 0.0);
        let out = infer_scene_evidence(SceneEvidenceInput {
            frequency_hz: 1_000.0,
            estimate: e,
            tracked: mature(e),
            magnitude: 1.0,
            reference_magnitude: 1.0,
        });
        assert_eq!(out.kind, SceneEvidenceKind::FrontalAnchor);
        assert_eq!(out.specificity, SpatialSpecificity::PreserveStereo);
    }

    #[test]
    fn source_like_low_bass_stays_a_foundation_even_when_lateral() {
        let e = estimate(1.0, 0.0, 0.0, 2.0);
        let out = infer_scene_evidence(SceneEvidenceInput {
            frequency_hz: 55.0,
            estimate: e,
            tracked: mature(e),
            magnitude: 1.0,
            reference_magnitude: 1.0,
        });
        assert_eq!(out.kind, SceneEvidenceKind::FrontalAnchor);
        assert!(out.bass_anchor > 0.99);
        assert!(out.foundation_support > 0.90);
        assert!(out.reassignment_safety < 0.01);
    }

    #[test]
    fn diffuse_low_frequency_energy_is_not_mislabeled_as_a_foundation() {
        let e = estimate(1.0, 1.0, 0.0, PI);
        let out = infer_scene_evidence(SceneEvidenceInput {
            frequency_hz: 55.0,
            estimate: e,
            tracked: mature(e),
            magnitude: 1.0,
            reference_magnitude: 1.0,
        });
        assert_eq!(out.kind, SceneEvidenceKind::DiffuseField);
        assert!(out.bass_anchor > 0.99);
        assert!(out.foundation_support < 0.01);
        assert!(out.reassignment_safety < 0.01);
    }

    #[test]
    fn mature_high_frequency_lateral_material_can_be_specific() {
        let e = estimate(1.0, 0.0, 0.0, 2.0);
        let out = infer_scene_evidence(SceneEvidenceInput {
            frequency_hz: 2_000.0,
            estimate: e,
            tracked: mature(e),
            magnitude: 1.0,
            reference_magnitude: 1.0,
        });
        assert_eq!(out.kind, SceneEvidenceKind::LateralObjectCandidate);
        assert_eq!(out.specificity, SpatialSpecificity::Specific);
        assert!(out.reassignment_safety > 0.30);
    }

    #[test]
    fn balanced_antiphase_material_is_a_diffuse_field() {
        let e = estimate(1.0, 1.0, 0.0, PI);
        let out = infer_scene_evidence(SceneEvidenceInput {
            frequency_hz: 2_000.0,
            estimate: e,
            tracked: mature(e),
            magnitude: 1.0,
            reference_magnitude: 1.0,
        });
        assert_eq!(out.kind, SceneEvidenceKind::DiffuseField);
        assert_eq!(out.specificity, SpatialSpecificity::Broad);
        assert!(out.field_support > 0.9);
    }

    #[test]
    fn first_frame_lateral_event_is_not_promoted_to_an_object() {
        let e = estimate(1.0, 0.0, 0.0, 2.0);
        let mut tracker = StereoEvidenceTracker::default();
        let tracked = tracker.update(e, 10.0, 200.0);
        let out = infer_scene_evidence(SceneEvidenceInput {
            frequency_hz: 2_000.0,
            estimate: e,
            tracked,
            magnitude: 1.0,
            reference_magnitude: 1.0,
        });
        assert_ne!(out.kind, SceneEvidenceKind::LateralObjectCandidate);
        assert_ne!(out.specificity, SpatialSpecificity::Specific);
    }

    #[test]
    fn bass_anchor_transition_is_continuous_and_bounded() {
        assert_eq!(bass_anchor_weight(40.0), 1.0);
        assert_eq!(bass_anchor_weight(300.0), 0.0);
        let a = bass_anchor_weight(120.0);
        let b = bass_anchor_weight(180.0);
        assert!(a > b);
        assert!((0.0..=1.0).contains(&a));
        assert!((0.0..=1.0).contains(&b));
    }
}
