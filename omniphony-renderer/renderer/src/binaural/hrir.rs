//! Head-related impulse responses (HRIR) and their interpolation grid.
//!
//! The renderer convolves each ear with a per-direction FIR. The *source* of
//! those FIRs is abstracted by [`HrirProvider`] so a measured set (KEMAR / SOFA,
//! M3) can replace the built-in [`SyntheticHrir`] without touching the render
//! path. The interaural *delay* is intentionally **not** baked into these FIRs
//! (it is applied separately as a per-ear delay line, see [`super::itd`]), which
//! keeps all grid FIRs time-aligned so they can be linearly interpolated without
//! comb-filtering.

/// FIR length per ear. 64 taps @ ≥44.1 kHz comfortably captures the synthetic
/// head-shadow response (and is a reasonable budget for a measured set).
pub const HRIR_LEN: usize = 64;

/// A left/right pair of (minimum-delay) impulse responses.
#[derive(Clone)]
pub struct HrirPair {
    pub left: [f32; HRIR_LEN],
    pub right: [f32; HRIR_LEN],
}

impl HrirPair {
    fn zeroed() -> Self {
        Self {
            left: [0.0; HRIR_LEN],
            right: [0.0; HRIR_LEN],
        }
    }
}

/// Produces an HRIR pair for a given direction. Implementors must return
/// time-aligned FIRs (no bulk interaural delay) for safe interpolation.
pub trait HrirProvider {
    /// `az_deg`: 0 = front, positive = right. `el_deg`: 0 = horizontal, +90 = up.
    fn render(&self, az_deg: f32, el_deg: f32, sample_rate: u32) -> HrirPair;
}

/// Built-in analytic head model: per-ear broadband gain (ILD) plus a one-pole
/// low-pass whose cutoff drops as the source moves contralateral (head shadow).
/// Self-contained (no measured data); a stand-in until a measured set is loaded.
pub struct SyntheticHrir;

impl SyntheticHrir {
    /// Minimum per-ear gain at full shadow (≈ −14 dB).
    const GAIN_MIN: f32 = 0.2;
    /// Low-pass cutoff at full shadow (Hz).
    const FC_SHADOW: f32 = 1200.0;

    fn one_pole_lowpass_ir(gain: f32, cutoff_hz: f32, sample_rate: u32, out: &mut [f32; HRIR_LEN]) {
        let fc = cutoff_hz.clamp(200.0, sample_rate as f32 * 0.49);
        let a = (-2.0 * std::f32::consts::PI * fc / sample_rate as f32).exp();
        // h[n] = gain*(1-a)*a^n — DC gain == `gain`.
        let mut p = gain * (1.0 - a);
        for slot in out.iter_mut() {
            *slot = p;
            p *= a;
        }
    }
}

impl HrirProvider for SyntheticHrir {
    fn render(&self, az_deg: f32, el_deg: f32, sample_rate: u32) -> HrirPair {
        let az = az_deg.to_radians();
        let el = el_deg.to_radians();
        // Lateral position: +1 fully right, −1 fully left, 0 median plane.
        let lateral = (az.sin() * el.cos()).clamp(-1.0, 1.0);
        let exposure_r = 0.5 * (1.0 + lateral);
        let exposure_l = 0.5 * (1.0 - lateral);

        let open = sample_rate as f32 * 0.45;
        let gain = |e: f32| Self::GAIN_MIN + (1.0 - Self::GAIN_MIN) * e;
        let cutoff = |e: f32| Self::FC_SHADOW + (open - Self::FC_SHADOW) * e;

        let mut pair = HrirPair::zeroed();
        Self::one_pole_lowpass_ir(
            gain(exposure_l),
            cutoff(exposure_l),
            sample_rate,
            &mut pair.left,
        );
        Self::one_pole_lowpass_ir(
            gain(exposure_r),
            cutoff(exposure_r),
            sample_rate,
            &mut pair.right,
        );
        pair
    }
}

/// A direction-indexed grid of HRIR pairs with bilinear (az × el) interpolation.
///
/// Built once from an [`HrirProvider`]; queried per object per frame on the
/// audio thread (cheap: 4 lookups + a sample-wise lerp).
pub struct HrirSet {
    az_count: usize,
    el_count: usize,
    el_min_deg: f32,
    el_max_deg: f32,
    /// Row-major `[el_idx * az_count + az_idx]`. Azimuth wraps; elevation clamps.
    grid: Vec<HrirPair>,
}

impl HrirSet {
    const AZ_STEP_DEG: f32 = 10.0;
    const EL_STEP_DEG: f32 = 10.0;
    const EL_MIN_DEG: f32 = -40.0;
    const EL_MAX_DEG: f32 = 90.0;

    /// Precompute the grid from `provider` at `sample_rate`.
    pub fn new(provider: &dyn HrirProvider, sample_rate: u32) -> Self {
        let az_count = (360.0 / Self::AZ_STEP_DEG).round() as usize; // wrap: no duplicate at 360
        let el_count =
            ((Self::EL_MAX_DEG - Self::EL_MIN_DEG) / Self::EL_STEP_DEG).round() as usize + 1;
        let mut grid = Vec::with_capacity(az_count * el_count);
        for ei in 0..el_count {
            let el = Self::EL_MIN_DEG + ei as f32 * Self::EL_STEP_DEG;
            for ai in 0..az_count {
                let az = ai as f32 * Self::AZ_STEP_DEG;
                grid.push(provider.render(az, el, sample_rate));
            }
        }
        Self {
            az_count,
            el_count,
            el_min_deg: Self::EL_MIN_DEG,
            el_max_deg: Self::EL_MAX_DEG,
            grid,
        }
    }

    /// Convenience constructor for the built-in synthetic model.
    pub fn synthetic(sample_rate: u32) -> Self {
        Self::new(&SyntheticHrir, sample_rate)
    }

    #[inline]
    fn pair(&self, az_idx: usize, el_idx: usize) -> &HrirPair {
        &self.grid[el_idx * self.az_count + az_idx]
    }

    /// Bilinearly-interpolated HRIR pair for an arbitrary direction.
    /// `az_deg`: 0 = front, positive = right. `el_deg`: 0 = horizontal.
    pub fn at(&self, az_deg: f32, el_deg: f32, out: &mut HrirPair) {
        // Azimuth: wrap into [0, az_count) cells.
        let az_norm = az_deg.rem_euclid(360.0) / Self::AZ_STEP_DEG;
        let a0 = az_norm.floor() as usize % self.az_count;
        let a1 = (a0 + 1) % self.az_count;
        let fa = az_norm - az_norm.floor();

        // Elevation: clamp into the grid, then interpolate between rows.
        let el_clamped = el_deg.clamp(self.el_min_deg, self.el_max_deg);
        let el_norm = (el_clamped - self.el_min_deg) / Self::EL_STEP_DEG;
        let e0 = (el_norm.floor() as usize).min(self.el_count - 1);
        let e1 = (e0 + 1).min(self.el_count - 1);
        let fe = el_norm - el_norm.floor();

        let p00 = self.pair(a0, e0);
        let p10 = self.pair(a1, e0);
        let p01 = self.pair(a0, e1);
        let p11 = self.pair(a1, e1);

        let w00 = (1.0 - fa) * (1.0 - fe);
        let w10 = fa * (1.0 - fe);
        let w01 = (1.0 - fa) * fe;
        let w11 = fa * fe;

        for n in 0..HRIR_LEN {
            out.left[n] =
                w00 * p00.left[n] + w10 * p10.left[n] + w01 * p01.left[n] + w11 * p11.left[n];
            out.right[n] =
                w00 * p00.right[n] + w10 * p10.right[n] + w01 * p01.right[n] + w11 * p11.right[n];
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn energy(h: &[f32; HRIR_LEN]) -> f32 {
        h.iter().map(|&x| x * x).sum()
    }

    #[test]
    fn right_source_is_louder_in_right_ear() {
        let set = HrirSet::synthetic(48_000);
        let mut p = HrirPair::zeroed();
        set.at(90.0, 0.0, &mut p);
        assert!(energy(&p.right) > energy(&p.left) * 2.0);
    }

    #[test]
    fn front_source_is_symmetric() {
        let set = HrirSet::synthetic(48_000);
        let mut p = HrirPair::zeroed();
        set.at(0.0, 0.0, &mut p);
        assert!((energy(&p.left) - energy(&p.right)).abs() < 1e-4);
    }

    #[test]
    fn interpolation_is_continuous_across_cells() {
        // A 0.1° step either side of a grid node must not jump the response.
        let set = HrirSet::synthetic(48_000);
        let mut a = HrirPair::zeroed();
        let mut b = HrirPair::zeroed();
        set.at(29.99, 5.0, &mut a);
        set.at(30.01, 5.0, &mut b);
        let max_diff = a
            .left
            .iter()
            .zip(b.left.iter())
            .map(|(x, y)| (x - y).abs())
            .fold(0.0f32, f32::max);
        assert!(max_diff < 1e-3, "discontinuity {max_diff}");
    }

    #[test]
    fn azimuth_wraps_at_360() {
        let set = HrirSet::synthetic(48_000);
        let mut a = HrirPair::zeroed();
        let mut b = HrirPair::zeroed();
        set.at(1.0, 0.0, &mut a);
        set.at(361.0, 0.0, &mut b);
        for n in 0..HRIR_LEN {
            assert!((a.left[n] - b.left[n]).abs() < 1e-6);
        }
    }
}
