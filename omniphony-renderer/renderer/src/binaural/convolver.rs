//! Direct-form FIR convolver for one ear of one object.
//!
//! Length is fixed at [`HRIR_LEN`]. The input-history ring buffer persists
//! across coefficient swaps, so updating the HRIR between frames (as the object
//! or head moves) keeps the *state* continuous — and a kernel change is
//! additionally crossfaded over a caller-chosen ramp
//! ([`set_coeffs_smooth`](EarConvolver::set_coeffs_smooth)): the old and new
//! kernels run over the same history and blend linearly, which is exactly
//! equivalent to interpolating the coefficients per sample, so the transfer
//! function moves without a block-boundary discontinuity (issue #155). The
//! doubled dot product is paid only during fade samples of blocks whose kernel
//! actually changed. Direct-form FIR is the simplest steady-state cost for
//! short kernels; a partitioned-FFT path can replace this in M2/M3 if
//! profiling on the object count demands it.

use super::hrir::HRIR_LEN;

pub struct EarConvolver {
    /// Past inputs; `pos` marks the slot just written (most recent sample).
    hist: [f32; HRIR_LEN],
    pos: usize,
    coeffs: [f32; HRIR_LEN],
    /// Whether a real transfer kernel has ever been installed. The first kernel
    /// has no audible predecessor to crossfade from, so it is installed
    /// immediately. Only subsequent changes require transfer-function continuity.
    initialized: bool,
    /// Fade-out kernel of a running crossfade (valid while `fade_pos < fade_len`).
    prev_coeffs: [f32; HRIR_LEN],
    fade_pos: u32,
    fade_len: u32,
}

impl Default for EarConvolver {
    fn default() -> Self {
        Self::new()
    }
}

impl EarConvolver {
    pub fn new() -> Self {
        Self {
            hist: [0.0; HRIR_LEN],
            pos: 0,
            coeffs: [0.0; HRIR_LEN],
            initialized: false,
            prev_coeffs: [0.0; HRIR_LEN],
            fade_pos: 0,
            fade_len: 0,
        }
    }

    /// Replace the FIR kernel immediately (no crossfade). The history is
    /// untouched, so the *state* remains continuous through the swap, but the
    /// transfer function jumps — prefer [`set_coeffs_smooth`](Self::set_coeffs_smooth)
    /// on live update paths.
    #[inline]
    pub fn set_coeffs(&mut self, coeffs: &[f32; HRIR_LEN]) {
        self.coeffs.copy_from_slice(coeffs);
        self.initialized = true;
        self.fade_pos = 0;
        self.fade_len = 0;
    }

    /// Replace the FIR kernel, crossfading from the current one over the next
    /// `fade_len` processed samples. The first kernel is installed immediately:
    /// before it there is no audible transfer function whose continuity needs
    /// preserving, and fading from the all-zero construction state would make
    /// channel activation depend on the caller's block size. A later no-op when
    /// the kernel is unchanged (a static object under a static head — the common
    /// case) costs one array compare and keeps the single dot product. Restarting
    /// mid-fade departs from the currently *effective* (blended) kernel, so
    /// back-to-back changes stay click-free too.
    pub fn set_coeffs_smooth(&mut self, coeffs: &[f32; HRIR_LEN], fade_len: usize) {
        if !self.initialized {
            self.set_coeffs(coeffs);
            return;
        }
        if *coeffs == self.coeffs {
            return;
        }
        if fade_len == 0 {
            self.set_coeffs(coeffs);
            return;
        }
        if self.fade_pos < self.fade_len {
            // Freeze the running blend as the new fade-out kernel.
            let w = self.fade_pos as f32 / self.fade_len as f32;
            for i in 0..HRIR_LEN {
                self.prev_coeffs[i] += (self.coeffs[i] - self.prev_coeffs[i]) * w;
            }
        } else {
            self.prev_coeffs.copy_from_slice(&self.coeffs);
        }
        self.coeffs.copy_from_slice(coeffs);
        self.fade_pos = 0;
        self.fade_len = fade_len as u32;
    }

    /// Push one input sample and return the filtered output.
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        self.hist[self.pos] = x;
        let mut idx = self.pos;
        let acc = if self.fade_pos < self.fade_len {
            // Crossfade: both kernels over the shared history, linear blend.
            self.fade_pos += 1;
            let w = self.fade_pos as f32 / self.fade_len as f32;
            let mut acc_new = 0.0f32;
            let mut acc_old = 0.0f32;
            for i in 0..HRIR_LEN {
                let h = self.hist[idx];
                acc_new += self.coeffs[i] * h;
                acc_old += self.prev_coeffs[i] * h;
                idx = if idx == 0 { HRIR_LEN - 1 } else { idx - 1 };
            }
            acc_old + (acc_new - acc_old) * w
        } else {
            let mut acc = 0.0f32;
            for &c in self.coeffs.iter() {
                acc += c * self.hist[idx];
                idx = if idx == 0 { HRIR_LEN - 1 } else { idx - 1 };
            }
            acc
        };
        self.pos = if self.pos + 1 == HRIR_LEN {
            0
        } else {
            self.pos + 1
        };
        acc
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unit_impulse_kernel_is_passthrough() {
        let mut c = EarConvolver::new();
        let mut k = [0.0; HRIR_LEN];
        k[0] = 1.0;
        c.set_coeffs(&k);
        assert_eq!(c.process(0.5), 0.5);
        assert_eq!(c.process(-0.25), -0.25);
    }

    #[test]
    fn delayed_kernel_delays_signal() {
        let mut c = EarConvolver::new();
        let mut k = [0.0; HRIR_LEN];
        k[3] = 1.0; // 3-sample delay
        c.set_coeffs(&k);
        let xs = [1.0, 0.0, 0.0, 0.0, 0.0];
        let ys: Vec<f32> = xs.iter().map(|&x| c.process(x)).collect();
        assert_eq!(ys, vec![0.0, 0.0, 0.0, 1.0, 0.0]);
    }

    #[test]
    fn dc_gain_equals_coefficient_sum() {
        let mut c = EarConvolver::new();
        let mut k = [0.0; HRIR_LEN];
        k[0] = 0.5;
        k[1] = 0.25;
        c.set_coeffs(&k);
        // Drive DC; after the kernel fills, output settles at sum of coeffs.
        let mut y = 0.0;
        for _ in 0..HRIR_LEN {
            y = c.process(1.0);
        }
        assert!((y - 0.75).abs() < 1e-6);
    }

    /// Construction has no previous audible transfer function. Installing the
    /// first HRIR through the smooth API must therefore be immediate and must
    /// not inherit a host callback-dependent fade length.
    #[test]
    fn first_smooth_kernel_install_is_fade_length_invariant() {
        let mut short = EarConvolver::new();
        let mut long = EarConvolver::new();
        let mut k = [0.0; HRIR_LEN];
        k[0] = 1.0;

        short.set_coeffs_smooth(&k, 40);
        long.set_coeffs_smooth(&k, HRIR_LEN);

        assert_eq!(short.process(1.0), 1.0);
        assert_eq!(long.process(1.0), 1.0);
    }

    /// A kernel change must ramp the output linearly over the fade instead of
    /// jumping at the swap (issue #155): DC through gain 1.0 → gain 0.5 with a
    /// 16-sample fade steps down by exactly 1/32 per sample.
    #[test]
    fn kernel_swap_crossfades_linearly() {
        let mut c = EarConvolver::new();
        let mut a = [0.0; HRIR_LEN];
        a[0] = 1.0;
        c.set_coeffs(&a);
        for _ in 0..HRIR_LEN {
            c.process(1.0); // settle at 1.0
        }
        let mut b = [0.0; HRIR_LEN];
        b[0] = 0.5;
        const FADE: usize = 16;
        c.set_coeffs_smooth(&b, FADE);
        let mut prev = 1.0f32;
        for i in 0..FADE {
            let y = c.process(1.0);
            let expected = 1.0 - 0.5 * (i + 1) as f32 / FADE as f32;
            assert!(
                (y - expected).abs() < 1e-6,
                "sample {i}: got {y}, expected {expected}"
            );
            assert!(y < prev, "fade must be monotonic");
            prev = y;
        }
        // Steady state on the new kernel afterwards.
        assert!((c.process(1.0) - 0.5).abs() < 1e-6);
    }

    /// Re-setting the same kernel must not restart a fade or change output.
    #[test]
    fn unchanged_kernel_is_a_no_op() {
        let mut c = EarConvolver::new();
        let mut k = [0.0; HRIR_LEN];
        k[0] = 1.0;
        c.set_coeffs(&k);
        for _ in 0..4 {
            c.process(1.0);
        }
        c.set_coeffs_smooth(&k, 16);
        assert_eq!(c.process(1.0), 1.0, "same kernel must stay transparent");
    }

    /// A second change mid-fade must depart from the blended kernel — the
    /// output stays inside the envelope of the kernels involved, no jump back.
    #[test]
    fn midfade_restart_stays_continuous() {
        let mut c = EarConvolver::new();
        let mut a = [0.0; HRIR_LEN];
        a[0] = 1.0;
        c.set_coeffs(&a);
        for _ in 0..HRIR_LEN {
            c.process(1.0);
        }
        let mut b = [0.0; HRIR_LEN];
        b[0] = 0.0; // fade toward silence
        c.set_coeffs_smooth(&b, 16);
        let mut y = 1.0;
        for _ in 0..8 {
            y = c.process(1.0); // half-way: ~0.5
        }
        assert!((y - 0.5).abs() < 1e-6);
        // Change again mid-fade, back to gain 1.0: must ramp 0.5 → 1.0.
        c.set_coeffs_smooth(&a, 16);
        let first = c.process(1.0);
        assert!(
            (first - 0.5).abs() < 0.1,
            "restart must depart from the blended kernel, got {first}"
        );
        for _ in 0..16 {
            y = c.process(1.0);
        }
        assert!((y - 1.0).abs() < 1e-6, "must settle on the new kernel");
    }
}
