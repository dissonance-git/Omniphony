//! Pure-Rust VBAP backend — drop-in replacement for `saf_backend.rs`.
//!
//! Used when the `saf_vbap` feature is disabled (no C FFI, no external library).

use super::Gains;
use super::gain_source::VbapGainSource;
use crate::spatial_vbap::vbap_native::{
    find_ls_triplets, invert_ls_mtx_3d, prepare_effective_speaker_dirs, vbap3d,
};

/// Maximum spread in degrees accepted by `vbap3d`.
/// Matches `SpartaVbapLayout::NORMALIZED_SPREAD_MAX_DEG` for parity.
const NORMALIZED_SPREAD_MAX_DEG: f32 = 180.0;

#[inline]
fn normalized_spread_to_degrees(spread: f32) -> f32 {
    spread.clamp(0.0, 1.0) * NORMALIZED_SPREAD_MAX_DEG
}

/// Pure-Rust equivalent of `SpartaVbapLayout`.
///
/// Owns the triangulation and inverse speaker matrices produced by
/// `find_ls_triplets` + `invert_ls_mtx_3d`. Implements [`VbapGainSource`]
/// so it can be used interchangeably with the SAF FFI backend.
pub(crate) struct NativeVbapLayout {
    /// Number of *real* (non-dummy) speakers — the size of the returned `Gains`.
    pub(crate) n_speakers: usize,
    pub(crate) n_faces: usize,
    /// Total speaker count used for triangulation (real + dummy virtual speakers).
    n_eff: usize,
    #[allow(dead_code)]
    u_spkr: Vec<[f32; 3]>,
    ls_groups: Vec<[usize; 3]>,
    layout_inv_mtx: Vec<[f32; 9]>,
    /// Per-effective-speaker flag: `true` for the virtual ±90° pole(s) injected
    /// for triangulation. `vbap3d` uses this to fold dummy gain back into real
    /// speakers of the matched triangle instead of letting it be silently dropped.
    is_dummy: Vec<bool>,
}

impl NativeVbapLayout {
    /// Build a layout from speaker directions (azimuth, elevation in degrees).
    ///
    /// The real layout is triangulated first. If that fails, virtual speakers at
    /// ±90° elevation are injected as a fallback so the 3D convex hull can be
    /// built. Dummy gains are stripped before returning.
    pub fn from_speaker_dirs(speaker_dirs_deg: &[[f32; 2]]) -> Result<Self, String> {
        let n_real = speaker_dirs_deg.len();
        let (effective_dirs, is_dummy) =
            prepare_effective_speaker_dirs(speaker_dirs_deg, true, true)
                .ok_or_else(|| "find_ls_triplets failed".to_string())?;

        let n_eff = effective_dirs.len();
        debug_assert_eq!(is_dummy.len(), n_eff);

        let (u_spkr, ls_groups) = find_ls_triplets(&effective_dirs, true)
            .ok_or_else(|| "find_ls_triplets failed".to_string())?;

        if ls_groups.is_empty() {
            return Err("No valid loudspeaker triangles found".to_string());
        }

        let layout_inv_mtx = invert_ls_mtx_3d(&u_spkr, &ls_groups);
        let n_faces = ls_groups.len();

        Ok(Self {
            n_speakers: n_real,
            n_faces,
            n_eff,
            u_spkr,
            ls_groups,
            layout_inv_mtx,
            is_dummy,
        })
    }

    /// Compute VBAP gains for a single source direction and spread.
    /// Returns gains for real speakers only (dummy columns are stripped).
    pub fn vbap_gains(
        &self,
        azimuth_deg: f32,
        elevation_deg: f32,
        spread: f32,
    ) -> Result<Gains, String> {
        let spread_deg = normalized_spread_to_degrees(spread);
        let src_dirs = [[azimuth_deg, elevation_deg]];

        let gain_vec = vbap3d(
            &src_dirs,
            self.n_eff,
            &self.ls_groups,
            spread_deg,
            &self.layout_inv_mtx,
            &self.is_dummy,
        );

        // Strip dummy speaker columns — keep only the first n_speakers entries.
        Ok(Gains::from_slice(&gain_vec[..self.n_speakers]))
    }
}

impl VbapGainSource for NativeVbapLayout {
    fn compute_gains(
        &self,
        azimuth_deg: f32,
        elevation_deg: f32,
        spread: f32,
    ) -> Result<Gains, String> {
        self.vbap_gains(azimuth_deg, elevation_deg, spread)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Standard horizontal layout (no top/bottom speakers) — exercises both
    /// dummy injections (±90°) so the redistribution path is hit at both poles.
    fn horizontal_7_layout() -> [[f32; 2]; 7] {
        [
            [0.0, 0.0],    // C
            [-30.0, 0.0],  // L
            [30.0, 0.0],   // R
            [-110.0, 0.0], // LS
            [110.0, 0.0],  // RS
            [-150.0, 0.0], // LRS
            [150.0, 0.0],  // RRS
        ]
    }

    fn rms(g: &Gains) -> f32 {
        (0..g.len()).map(|i| g[i] * g[i]).sum::<f32>().sqrt()
    }

    #[test]
    fn test_real_height_speakers_do_not_force_dummy_poles() {
        let dirs = [
            [-30.0_f32, 0.0],
            [30.0, 0.0],
            [-110.0, 0.0],
            [110.0, 0.0],
            [-45.0, 36.0],
            [45.0, 36.0],
        ];

        let layout = NativeVbapLayout::from_speaker_dirs(&dirs).unwrap();

        assert_eq!(layout.n_speakers, dirs.len());
        assert_eq!(layout.n_eff, dirs.len());
        assert!(layout.is_dummy.iter().all(|flag| !flag));
    }

    #[test]
    fn test_coplanar_layout_still_falls_back_to_dummy_poles() {
        let dirs = horizontal_7_layout();
        let layout = NativeVbapLayout::from_speaker_dirs(&dirs).unwrap();

        assert_eq!(layout.n_speakers, dirs.len());
        assert_eq!(layout.n_eff, dirs.len() + 2);
        assert_eq!(layout.is_dummy.iter().filter(|flag| **flag).count(), 2);
    }

    #[test]
    fn test_vertical_z_axis_no_silence() {
        let dirs = horizontal_7_layout();
        let layout = NativeVbapLayout::from_speaker_dirs(&dirs).unwrap();

        // (X=0, Y=0, Z>0) → elevation = +90° (zenith)
        let zenith = layout.vbap_gains(0.0, 90.0, 0.0).unwrap();
        assert!(
            rms(&zenith) > 0.5,
            "zenith should not be silent, got rms={}",
            rms(&zenith)
        );

        // (X=0, Y=0, Z<0) → elevation = -90° (nadir)
        let nadir = layout.vbap_gains(0.0, -90.0, 0.0).unwrap();
        assert!(
            rms(&nadir) > 0.5,
            "nadir should not be silent, got rms={}",
            rms(&nadir)
        );
    }

    #[test]
    fn test_z_sweep_continuity() {
        let dirs = horizontal_7_layout();
        let layout = NativeVbapLayout::from_speaker_dirs(&dirs).unwrap();

        // Sweep elevation from -90° to +90°, azimuth pinned (matches the
        // X=0, Y=0, Z varying trajectory after `adm_to_spherical`).
        for el_i in -9..=9 {
            let el = el_i as f32 * 10.0;
            let g = layout.vbap_gains(0.0, el, 0.0).unwrap();
            assert!(
                rms(&g) > 0.5,
                "energy dropout at elevation {el}°: rms={}",
                rms(&g)
            );
        }
    }

    #[test]
    fn test_energy_conservation_at_pole() {
        let dirs = horizontal_7_layout();
        let layout = NativeVbapLayout::from_speaker_dirs(&dirs).unwrap();

        // After redistribution, the RMS over real speakers should be ≈ 1.0
        // (energy that used to leak onto the dummy is now folded back).
        for el in [-90.0_f32, -75.0, 0.0, 75.0, 90.0] {
            let g = layout.vbap_gains(0.0, el, 0.0).unwrap();
            let r = rms(&g);
            assert!(
                (r - 1.0).abs() < 0.05,
                "energy not conserved at elevation {el}°: rms={r}"
            );
        }
    }

    #[test]
    fn test_coplanar_speakers_position_aware() {
        // 4 speakers all at el=0 — previously would fail triangulation.
        let dirs = [
            [-90.0_f32, 0.0], // Left
            [90.0, 0.0],      // Right
            [0.0, 0.0],       // Front
            [180.0, 0.0],     // Rear
        ];
        let layout =
            NativeVbapLayout::from_speaker_dirs(&dirs).expect("should succeed with dummy speakers");

        assert_eq!(layout.n_speakers, 4);

        // Source at left (az=-90) → left speaker should dominate
        let gains_left = layout.vbap_gains(-90.0, 0.0, 0.0).unwrap();
        let gains_right = layout.vbap_gains(90.0, 0.0, 0.0).unwrap();

        assert_eq!(gains_left.len(), 4);
        // Left speaker (index 0) should have highest gain when source is on the left
        let left_at_left: f32 = gains_left[0];
        let right_at_left: f32 = gains_left[1];
        assert!(
            left_at_left > right_at_left,
            "left speaker gain {left_at_left} should exceed right {right_at_left} for left source"
        );
        // Right speaker (index 1) should have highest gain when source is on the right
        let left_at_right: f32 = gains_right[0];
        let right_at_right: f32 = gains_right[1];
        assert!(
            right_at_right > left_at_right,
            "right speaker gain {right_at_right} should exceed left {left_at_right} for right source"
        );
    }
}
