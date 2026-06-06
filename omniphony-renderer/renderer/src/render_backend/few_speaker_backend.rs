use std::f32::consts::FRAC_1_SQRT_2;

use anyhow::Result;

use super::room_transform::room_scaled_position;
use super::{BackendCapabilities, GainModel, GainModelKind, RenderRequest, RenderResponse};
use crate::spatial_vbap::{Gains, spherical_to_adm};
use crate::speaker_layout::SpeakerLayout;

/// VBAP for degenerate geometry: 1 or 2 spatializable speakers, where the panner
/// cannot triangulate. Used for crossover bands (and stereo/mono main layouts)
/// that fall below 3 speakers.
///
/// Same model as VBAP — distance is ignored, only the projection of the speaker
/// and object **directions** onto the unit sphere matters — so an object crossing
/// from a 2-speaker region into a 3-speaker region stays consistent. Like
/// `VbapBackend` it returns pure panning gains; distance attenuation and diffuse
/// blending are applied by the shared decorators.
pub struct FewSpeakerBackend {
    /// Speaker unit direction vectors (ADM), precomputed once from the az/el
    /// positions. Length 1 or 2 in normal use.
    speaker_dirs: Vec<[f32; 3]>,
}

#[inline]
fn dot(a: [f32; 3], b: [f32; 3]) -> f32 {
    a[0] * b[0] + a[1] * b[1] + a[2] * b[2]
}

impl FewSpeakerBackend {
    /// `positions` are speaker `[azimuth, elevation]` in degrees (the same
    /// room-adjusted directions VBAP receives).
    pub fn new(positions: Vec<[f32; 2]>) -> Self {
        let speaker_dirs = positions
            .iter()
            .map(|&[az, el]| {
                let (x, y, z) = spherical_to_adm(az, el, 1.0);
                [x, y, z]
            })
            .collect();
        Self { speaker_dirs }
    }

    fn equal_power(n: usize) -> Gains {
        let mut g = Gains::zeroed(n);
        if n > 0 {
            let v = 1.0 / (n as f32).sqrt();
            for i in 0..n {
                g.set(i, v);
            }
        }
        g
    }

    pub fn speaker_count(&self) -> usize {
        self.speaker_dirs.len()
    }

    pub fn compute_gains(&self, req: &RenderRequest) -> RenderResponse {
        let n = self.speaker_dirs.len();

        // One speaker: it gets all the energy regardless of position.
        if n == 1 {
            let mut g = Gains::zeroed(1);
            g.set(0, 1.0);
            return RenderResponse { gains: g };
        }
        if n != 2 {
            // Never built outside {1, 2}; stay safe rather than panic.
            return RenderResponse {
                gains: Self::equal_power(n),
            };
        }

        // Object direction on the unit sphere (distance dropped via normalisation).
        let scaled = room_scaled_position(
            req.adm_position.map(|v| v as f32),
            req.room_ratio,
            req.room_ratio_rear,
            req.room_ratio_lower,
            req.room_ratio_center_blend,
        );
        let norm_p = dot(scaled, scaled).sqrt();
        if norm_p < 1e-9 {
            // Object at the listener: no direction → split equally.
            return RenderResponse {
                gains: Self::equal_power(2),
            };
        }
        let p = [scaled[0] / norm_p, scaled[1] / norm_p, scaled[2] / norm_p];

        let l0 = self.speaker_dirs[0];
        let l1 = self.speaker_dirs[1];
        let a = dot(l0, p);
        let b = dot(l1, p);

        // Object direction == a speaker direction ⇒ 100% on that speaker
        // (also covers two speakers sharing a direction).
        const COINCIDENT: f32 = 1.0 - 1e-6;
        if a >= COINCIDENT {
            let mut g = Gains::zeroed(2);
            g.set(0, 1.0);
            return RenderResponse { gains: g };
        }
        if b >= COINCIDENT {
            let mut g = Gains::zeroed(2);
            g.set(1, 1.0);
            return RenderResponse { gains: g };
        }

        // Pairwise VBAP: solve g0*l0 + g1*l1 ≈ p (least squares over the speaker
        // pair plane), clamp negatives (out-of-arc → nearest speaker), then
        // constant-power normalise.
        let c = dot(l0, l1);
        let det = 1.0 - c * c;
        let (mut g0, mut g1) = if det < 1e-6 {
            // Coincident or antipodal speakers: no usable pair plane.
            (FRAC_1_SQRT_2, FRAC_1_SQRT_2)
        } else {
            ((a - c * b) / det, (b - c * a) / det)
        };
        g0 = g0.max(0.0);
        g1 = g1.max(0.0);
        let norm = (g0 * g0 + g1 * g1).sqrt();
        if norm > 1e-6 {
            g0 /= norm;
            g1 /= norm;
        } else {
            g0 = FRAC_1_SQRT_2;
            g1 = FRAC_1_SQRT_2;
        }

        let mut g = Gains::zeroed(2);
        g.set(0, g0);
        g.set(1, g1);
        RenderResponse { gains: g }
    }

    pub fn save_to_file(
        &self,
        path: &std::path::Path,
        speaker_layout: &SpeakerLayout,
    ) -> Result<()> {
        let _ = (path, speaker_layout);
        // Geometry-only, like VbapBackend: precomputed tables are exported from the
        // evaluation layer, not from the backend.
        anyhow::bail!(
            "few-speaker backend is geometry-only; export the precomputed table via the evaluator"
        )
    }
}

impl GainModel for FewSpeakerBackend {
    fn kind(&self) -> GainModelKind {
        // Degenerate VBAP — reuse the VBAP identity (no separate UI backend).
        GainModelKind::Vbap
    }

    fn backend_id(&self) -> &'static str {
        "vbap"
    }

    fn backend_label(&self) -> &'static str {
        "VBAP"
    }

    fn capabilities(&self) -> BackendCapabilities {
        BackendCapabilities {
            supports_realtime: true,
            supports_precomputed_polar: true,
            supports_precomputed_cartesian: true,
            supports_position_interpolation: true,
            supports_distance_model: true,
            supports_spread: false,
            supports_spread_from_distance: false,
            supports_event_size: false,
            supports_distance_diffuse: true,
            supports_table_export: true,
        }
    }

    fn speaker_count(&self) -> usize {
        FewSpeakerBackend::speaker_count(self)
    }

    fn compute_gains(&self, req: &RenderRequest) -> RenderResponse {
        FewSpeakerBackend::compute_gains(self, req)
    }

    fn save_to_file(&self, path: &std::path::Path, speaker_layout: &SpeakerLayout) -> Result<()> {
        FewSpeakerBackend::save_to_file(self, path, speaker_layout)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Neutral request: identity room ratios, so the scaled position equals the
    /// ADM position (the backend then only depends on direction).
    fn req(pos: [f64; 3]) -> RenderRequest {
        RenderRequest {
            adm_position: pos,
            event_size: [0.0; 3],
            size_to_spread_mode: Default::default(),
            spread_min: 0.0,
            spread_max: 0.0,
            spread_from_distance: false,
            spread_distance_range: 1.0,
            spread_distance_curve: 1.0,
            room_ratio: [1.0, 1.0, 1.0],
            room_ratio_rear: 1.0,
            room_ratio_lower: 1.0,
            room_ratio_center_blend: 0.5,
            use_distance_diffuse: false,
            distance_diffuse_threshold: 1.0,
            distance_diffuse_curve: 1.0,
            distance_model: crate::spatial_vbap::DistanceModel::default(),
            experimental_distance_distance_floor: 0.0,
            experimental_distance_min_active_speakers: 1,
            experimental_distance_max_active_speakers: 1,
            experimental_distance_position_error_floor: 0.0,
            experimental_distance_position_error_nearest_scale: 0.0,
            experimental_distance_position_error_span_scale: 0.0,
        }
    }

    fn adm(az: f32, el: f32, dist: f32) -> [f64; 3] {
        let (x, y, z) = spherical_to_adm(az, el, dist);
        [x as f64, y as f64, z as f64]
    }

    #[test]
    fn single_speaker_is_unity_everywhere() {
        let b = FewSpeakerBackend::new(vec![[30.0, 0.0]]);
        for pos in [adm(0.0, 0.0, 1.0), adm(120.0, 45.0, 3.0), [0.0, 0.0, 0.0]] {
            let g = b.compute_gains(&req(pos)).gains;
            assert_eq!(g.len(), 1);
            assert!((g[0] - 1.0).abs() < 1e-6, "got {:?}", &g[..]);
        }
    }

    #[test]
    fn object_on_speaker_direction_is_full_on_that_speaker() {
        let b = FewSpeakerBackend::new(vec![[-30.0, 0.0], [30.0, 0.0]]);
        let g0 = b.compute_gains(&req(adm(-30.0, 0.0, 1.0))).gains;
        assert!(
            (g0[0] - 1.0).abs() < 1e-6 && g0[1].abs() < 1e-6,
            "{:?}",
            &g0[..]
        );
        let g1 = b.compute_gains(&req(adm(30.0, 0.0, 1.0))).gains;
        assert!(
            g1[0].abs() < 1e-6 && (g1[1] - 1.0).abs() < 1e-6,
            "{:?}",
            &g1[..]
        );
    }

    #[test]
    fn midpoint_is_balanced_constant_power() {
        let b = FewSpeakerBackend::new(vec![[-30.0, 0.0], [30.0, 0.0]]);
        let g = b.compute_gains(&req(adm(0.0, 0.0, 1.0))).gains;
        assert!((g[0] - g[1]).abs() < 1e-5, "{:?}", &g[..]);
        assert!((g[0] - FRAC_1_SQRT_2).abs() < 1e-4, "{:?}", &g[..]);
    }

    #[test]
    fn constant_power_across_the_arc() {
        let b = FewSpeakerBackend::new(vec![[-30.0, 0.0], [30.0, 0.0]]);
        for az in [-30.0, -15.0, -5.0, 0.0, 7.0, 20.0, 30.0] {
            let g = b.compute_gains(&req(adm(az, 0.0, 1.0))).gains;
            let power = g[0] * g[0] + g[1] * g[1];
            assert!((power - 1.0).abs() < 1e-4, "az={az} power={power}");
        }
    }

    #[test]
    fn out_of_arc_collapses_to_nearest() {
        let b = FewSpeakerBackend::new(vec![[-30.0, 0.0], [30.0, 0.0]]);
        // Far to the left, well outside [-30, 30]: only the left speaker survives.
        let g = b.compute_gains(&req(adm(-90.0, 0.0, 1.0))).gains;
        assert!(
            (g[0] - 1.0).abs() < 1e-6 && g[1].abs() < 1e-6,
            "{:?}",
            &g[..]
        );
    }

    #[test]
    fn distance_is_ignored() {
        let b = FewSpeakerBackend::new(vec![[-30.0, 0.0], [30.0, 0.0]]);
        // Keep both probes inside the unit box (the room depth warp clamps |y|≤1,
        // like VBAP): within it the gains depend only on direction, not distance.
        let near = b.compute_gains(&req(adm(12.0, 10.0, 0.4))).gains;
        let far = b.compute_gains(&req(adm(12.0, 10.0, 0.8))).gains;
        assert!((near[0] - far[0]).abs() < 1e-4 && (near[1] - far[1]).abs() < 1e-4);
    }

    #[test]
    fn antipodal_speakers_do_not_nan() {
        let b = FewSpeakerBackend::new(vec![[0.0, 0.0], [180.0, 0.0]]);
        let g = b.compute_gains(&req(adm(90.0, 0.0, 1.0))).gains;
        assert!(g[0].is_finite() && g[1].is_finite(), "{:?}", &g[..]);
        let power = g[0] * g[0] + g[1] * g[1];
        assert!((power - 1.0).abs() < 1e-4, "power={power}");
    }
}
