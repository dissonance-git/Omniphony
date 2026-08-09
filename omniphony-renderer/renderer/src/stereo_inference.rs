//! Lightweight stereo evidence extraction for ordinary music sources.
//!
//! This module is intentionally narrower than a full auditory-scene model. It
//! provides inspectable signal-derived evidence that downstream Omniphony code
//! can use when deciding whether stereo energy behaves more like a coherent
//! object, a hard-localized object, or a diffuse/field-like component.
//!
//! The directness formulation is a clean Rust reimplementation of an idea that
//! proved useful in the earlier `dissonance-git/spatial-dsp` experiment: phase
//! alignment is useful for shared L/R material, but a hard-panned dry source
//! must not be called diffuse merely because the near-silent opposite channel
//! has numerically unstable phase. Channel asymmetry therefore also contributes
//! evidence for source-like directness.
//!
//! This is evidence, not semantic truth. It does not name instruments, create
//! stems, or decide final binaural positions by itself.

use std::f32::consts::PI;

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StereoBinEvidence {
    /// Magnitude of the left-channel complex bin.
    pub left_magnitude: f32,
    /// Magnitude of the right-channel complex bin.
    pub right_magnitude: f32,
    /// Left-channel phase in radians.
    pub left_phase: f32,
    /// Right-channel phase in radians.
    pub right_phase: f32,
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StereoBinEstimate {
    /// L/R position evidence in [-1, 1]. -1 is left, +1 is right.
    pub pan: f32,
    /// Cosine phase coherence in [-1, 1].
    pub phase_coherence: f32,
    /// Magnitude asymmetry in [0, 1]. 1 means effectively one-sided.
    pub pan_intensity: f32,
    /// Source-like/direct evidence in [0, 1].
    pub directness: f32,
    /// Complementary field-like evidence in [0, 1].
    pub diffuseness: f32,
    /// Magnitude of the true complex mid signal, |(L + R) / 2|.
    pub mid_magnitude: f32,
    /// Magnitude of the true complex side signal, |(L - R) / 2|.
    pub side_magnitude: f32,
    /// Quadrature magnitude of the stereo pair.
    pub total_magnitude: f32,
}

impl Default for StereoBinEstimate {
    fn default() -> Self {
        Self {
            pan: 0.0,
            phase_coherence: 1.0,
            pan_intensity: 0.0,
            directness: 0.0,
            diffuseness: 1.0,
            mid_magnitude: 0.0,
            side_magnitude: 0.0,
            total_magnitude: 0.0,
        }
    }
}

/// Parameters that reshape raw stereo evidence without turning it into a
/// renderer policy.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StereoInferenceParams {
    /// Positive values make source-like evidence more decisive. Negative values
    /// make classification more conservative. Range is clamped to [-1, 1].
    pub focus: f32,
    /// Additional separation pressure in [0, 1]. This exaggerates the gap
    /// between source-like and field-like evidence while retaining continuity.
    pub object_separation: f32,
}

impl Default for StereoInferenceParams {
    fn default() -> Self {
        Self {
            focus: 0.0,
            object_separation: 0.0,
        }
    }
}

/// Wrap an absolute phase difference to [0, π].
pub fn wrapped_phase_delta(left_phase: f32, right_phase: f32) -> f32 {
    let mut delta = (left_phase - right_phase).abs() % (2.0 * PI);
    if delta > PI {
        delta = 2.0 * PI - delta;
    }
    delta
}

#[inline]
fn polar_to_cartesian(magnitude: f32, phase: f32) -> (f32, f32) {
    (magnitude * phase.cos(), magnitude * phase.sin())
}

/// Estimate source-like versus diffuse evidence for one stereo frequency bin.
///
/// The key property is that both of these can support directness:
///
/// - phase-aligned energy present in both channels;
/// - strongly asymmetric energy that is cleanly localized to one channel.
///
/// This avoids treating a hard-panned dry source as diffuse simply because the
/// almost-silent channel has an arbitrary numerical phase.
pub fn estimate_bin(
    evidence: StereoBinEvidence,
    params: StereoInferenceParams,
) -> StereoBinEstimate {
    let left = evidence.left_magnitude.max(0.0);
    let right = evidence.right_magnitude.max(0.0);
    let sum = left + right;

    if sum <= 1.0e-12 {
        return StereoBinEstimate::default();
    }

    let delta = wrapped_phase_delta(evidence.left_phase, evidence.right_phase);
    let phase_coherence = delta.cos().clamp(-1.0, 1.0);
    let phase_alignment = phase_coherence.max(0.0);

    let pan = ((right - left) / sum).clamp(-1.0, 1.0);
    let pan_intensity = ((left - right).abs() / sum).clamp(0.0, 1.0);

    // Shared, phase-aligned energy is source-like. Strongly one-sided energy is
    // also source-like. In-between cases smoothly interpolate between the two.
    let mut directness =
        (pan_intensity + phase_alignment * (1.0 - pan_intensity)).clamp(0.0, 1.0);

    let focus = params.focus.clamp(-1.0, 1.0);
    if focus > 0.0 {
        directness = 1.0 - (1.0 - directness).powf(1.0 + focus * 3.0);
    } else if focus < 0.0 {
        directness = directness.powf(1.0 + (-focus) * 3.0);
    }

    let separation = params.object_separation.clamp(0.0, 1.0);
    if separation > 0.0 {
        directness = 1.0 - (1.0 - directness).powf(1.0 + separation * 4.0);
    }

    // M/S must be formed from the complex bins, not from scalar magnitudes.
    // Otherwise equal-amplitude antiphase material incorrectly looks all-mid.
    let (left_re, left_im) = polar_to_cartesian(left, evidence.left_phase);
    let (right_re, right_im) = polar_to_cartesian(right, evidence.right_phase);
    let mid_magnitude =
        0.5 * (left_re + right_re).hypot(left_im + right_im);
    let side_magnitude =
        0.5 * (left_re - right_re).hypot(left_im - right_im);

    StereoBinEstimate {
        pan,
        phase_coherence,
        pan_intensity,
        directness,
        diffuseness: 1.0 - directness,
        mid_magnitude,
        side_magnitude,
        total_magnitude: left.hypot(right),
    }
}

/// Time-aware state for one analysis bin or one aggregated band.
///
/// Unlike the earlier block-fixed EMA experiment, this tracker derives its
/// update coefficient from elapsed time and a time constant. Its behaviour is
/// therefore stable when FFT size, hop size, or sample rate changes.
///
/// A newly observed component is intentionally *not* considered stable merely
/// because its first observation agrees with itself. `observed_ms` supplies a
/// separate persistence/maturity term, so one-frame excursions cannot receive
/// full object-like support on arrival.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StereoEvidenceTracker {
    directness: f32,
    pan: f32,
    observed_ms: f32,
    initialized: bool,
}

impl Default for StereoEvidenceTracker {
    fn default() -> Self {
        Self {
            directness: 0.0,
            pan: 0.0,
            observed_ms: 0.0,
            initialized: false,
        }
    }
}

impl StereoEvidenceTracker {
    pub fn reset(&mut self) {
        *self = Self::default();
    }

    pub fn directness(&self) -> f32 {
        self.directness
    }

    pub fn pan(&self) -> f32 {
        self.pan
    }

    pub fn observed_ms(&self) -> f32 {
        self.observed_ms
    }

    /// Update the persistent state.
    ///
    /// `elapsed_ms` is the time since the previous estimate for this bin/band.
    /// `time_constant_ms` describes both the smoothing horizon and, for this
    /// lightweight pre-scene tracker, the maturity horizon for object evidence.
    /// A later scene layer may split those into independently learned values.
    pub fn update(
        &mut self,
        estimate: StereoBinEstimate,
        elapsed_ms: f32,
        time_constant_ms: f32,
    ) -> TrackedStereoEvidence {
        let elapsed_ms = elapsed_ms.max(0.0);

        if !self.initialized {
            self.directness = estimate.directness;
            self.pan = estimate.pan;
            self.initialized = true;
        } else {
            let alpha = ema_alpha(elapsed_ms, time_constant_ms);
            self.directness += alpha * (estimate.directness - self.directness);
            self.pan += alpha * (estimate.pan - self.pan);
        }

        self.observed_ms += elapsed_ms;

        let pan_deviation = (estimate.pan - self.pan).abs();
        let agreement = (1.0 - 4.0 * pan_deviation).clamp(0.0, 1.0);
        let persistence = persistence_weight(self.observed_ms, time_constant_ms);
        let stability = agreement * persistence;

        TrackedStereoEvidence {
            directness: self.directness.clamp(0.0, 1.0),
            pan: self.pan.clamp(-1.0, 1.0),
            instantaneous_pan: estimate.pan,
            pan_deviation,
            agreement,
            persistence,
            stability,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TrackedStereoEvidence {
    pub directness: f32,
    pub pan: f32,
    pub instantaneous_pan: f32,
    pub pan_deviation: f32,
    /// Instantaneous agreement with the tracked trajectory, independent of age.
    pub agreement: f32,
    /// Evidence that the candidate has existed long enough to be trusted.
    pub persistence: f32,
    /// Conservative product of trajectory agreement and persistence.
    pub stability: f32,
}

pub fn ema_alpha(elapsed_ms: f32, time_constant_ms: f32) -> f32 {
    if elapsed_ms <= 0.0 {
        return 0.0;
    }
    if time_constant_ms <= 0.0 {
        return 1.0;
    }
    (1.0 - (-elapsed_ms / time_constant_ms).exp()).clamp(0.0, 1.0)
}

pub fn persistence_weight(observed_ms: f32, time_constant_ms: f32) -> f32 {
    if observed_ms <= 0.0 {
        return 0.0;
    }
    if time_constant_ms <= 0.0 {
        return 1.0;
    }
    (1.0 - (-observed_ms / time_constant_ms).exp()).clamp(0.0, 1.0)
}

/// Conservative score for evidence that a lateral component behaves like a
/// persistent object rather than a one-frame transient.
///
/// This deliberately remains a score instead of a routing command. A future
/// libaural/Omniphony scene layer can combine it with onset, timbre, masking,
/// semantic and spatial evidence before deciding whether an object exists.
pub fn stable_lateral_object_score(
    tracked: TrackedStereoEvidence,
    magnitude: f32,
    reference_magnitude: f32,
) -> f32 {
    if magnitude <= 0.0 {
        return 0.0;
    }

    let lateral = ((tracked.pan.abs() - 0.35) / 0.65).clamp(0.0, 1.0);
    let energy = if reference_magnitude > 1.0e-12 {
        (magnitude / reference_magnitude).clamp(0.0, 2.0) * 0.5
    } else {
        1.0
    };

    (lateral * tracked.stability * tracked.directness * energy).clamp(0.0, 1.0)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn e(l: f32, r: f32, lp: f32, rp: f32) -> StereoBinEvidence {
        StereoBinEvidence {
            left_magnitude: l,
            right_magnitude: r,
            left_phase: lp,
            right_phase: rp,
        }
    }

    #[test]
    fn centered_in_phase_material_is_direct_and_mid_dominant() {
        let v = estimate_bin(e(1.0, 1.0, 0.2, 0.2), StereoInferenceParams::default());
        assert!(v.pan.abs() < 1.0e-6);
        assert!(v.phase_coherence > 0.999);
        assert!(v.directness > 0.999);
        assert!((v.mid_magnitude - 1.0).abs() < 1.0e-5);
        assert!(v.side_magnitude < 1.0e-5);
    }

    #[test]
    fn hard_panned_material_is_not_misclassified_by_missing_channel_phase() {
        let v = estimate_bin(e(1.0, 0.0, 0.0, 2.4), StereoInferenceParams::default());
        assert!(v.pan < -0.999);
        assert!(v.pan_intensity > 0.999);
        assert!(v.directness > 0.999);
    }

    #[test]
    fn balanced_antiphase_material_is_diffuse_and_side_dominant() {
        let v = estimate_bin(e(1.0, 1.0, 0.0, PI), StereoInferenceParams::default());
        assert!(v.phase_coherence < -0.999);
        assert!(v.directness < 0.001);
        assert!(v.diffuseness > 0.999);
        assert!(v.mid_magnitude < 1.0e-5);
        assert!((v.side_magnitude - 1.0).abs() < 1.0e-5);
    }

    #[test]
    fn time_constant_tracker_is_independent_of_block_shape() {
        let estimate = StereoBinEstimate {
            pan: 1.0,
            directness: 1.0,
            ..StereoBinEstimate::default()
        };

        let mut a = StereoEvidenceTracker::default();
        let mut b = StereoEvidenceTracker::default();

        // Seed both from the same state.
        let seed = StereoBinEstimate {
            pan: 0.0,
            directness: 0.0,
            ..StereoBinEstimate::default()
        };
        a.update(seed, 1.0, 200.0);
        b.update(seed, 1.0, 200.0);

        // One 100 ms update should closely match ten 10 ms updates.
        let one = a.update(estimate, 100.0, 200.0);
        let mut many = b.update(estimate, 10.0, 200.0);
        for _ in 1..10 {
            many = b.update(estimate, 10.0, 200.0);
        }

        assert!((one.pan - many.pan).abs() < 1.0e-5);
        assert!((one.directness - many.directness).abs() < 1.0e-5);
        assert!((one.persistence - many.persistence).abs() < 1.0e-5);
    }

    #[test]
    fn new_lateral_candidate_is_not_immediately_stable() {
        let estimate = StereoBinEstimate {
            pan: 0.9,
            directness: 1.0,
            ..StereoBinEstimate::default()
        };
        let mut tracker = StereoEvidenceTracker::default();
        let first = tracker.update(estimate, 10.0, 200.0);

        assert!(first.agreement > 0.999);
        assert!(first.persistence < 0.06);
        assert!(first.stability < 0.06);
        assert!(stable_lateral_object_score(first, 1.0, 1.0) < 0.05);
    }

    #[test]
    fn sustained_lateral_candidate_accumulates_object_support() {
        let estimate = StereoBinEstimate {
            pan: 0.9,
            directness: 1.0,
            ..StereoBinEstimate::default()
        };
        let mut tracker = StereoEvidenceTracker::default();
        let mut tracked = tracker.update(estimate, 10.0, 200.0);
        for _ in 1..40 {
            tracked = tracker.update(estimate, 10.0, 200.0);
        }

        assert!(tracked.persistence > 0.86);
        assert!(tracked.stability > 0.86);
        assert!(stable_lateral_object_score(tracked, 1.0, 1.0) > 0.35);
    }

    #[test]
    fn pan_excursion_breaks_trajectory_agreement() {
        let stable = StereoBinEstimate {
            pan: 0.85,
            directness: 1.0,
            ..StereoBinEstimate::default()
        };
        let excursion = StereoBinEstimate {
            pan: -0.85,
            directness: 1.0,
            ..StereoBinEstimate::default()
        };
        let mut tracker = StereoEvidenceTracker::default();
        for _ in 0..40 {
            tracker.update(stable, 10.0, 200.0);
        }
        let tracked = tracker.update(excursion, 10.0, 200.0);

        assert!(tracked.pan_deviation > 1.0);
        assert_eq!(tracked.agreement, 0.0);
        assert_eq!(tracked.stability, 0.0);
        assert_eq!(stable_lateral_object_score(tracked, 1.0, 1.0), 0.0);
    }
}
