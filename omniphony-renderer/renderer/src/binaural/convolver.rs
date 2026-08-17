//! Direct-form FIR convolver for one ear of one object.
//!
//! Length is fixed at [`HRIR_LEN`]. The input history persists across
//! coefficient swaps, so updating the HRIR between frames keeps state
//! continuous. Kernel changes can additionally crossfade over a caller-chosen
//! ramp. The steady-state tap loop follows upstream Omniphony v0.5.0's
//! throughput-oriented layout, while retaining this fork's first-kernel and
//! stream-reset guarantees.
//!
//! # Tap-loop layout
//!
//! The history is double-written so the live FIR window is always one
//! contiguous ascending slice. Kernels are stored reversed to match that
//! oldest-to-newest window. Accumulation is split across independent lanes so
//! the hot loop is throughput-bound instead of one serial floating-point chain.

use super::hrir::HRIR_LEN;

const ACC_LANES: usize = 8;

const _: () = assert!(
    HRIR_LEN.is_multiple_of(ACC_LANES),
    "the tap loop consumes the kernel in whole ACC_LANES chunks"
);

#[inline(always)]
fn dot(coeffs: &[f32; HRIR_LEN], win: &[f32]) -> f32 {
    let mut acc = [0.0f32; ACC_LANES];
    for (c, h) in coeffs
        .chunks_exact(ACC_LANES)
        .zip(win.chunks_exact(ACC_LANES))
    {
        for lane in 0..ACC_LANES {
            acc[lane] += c[lane] * h[lane];
        }
    }
    acc.iter().sum()
}

#[inline(always)]
fn dot2(new_c: &[f32; HRIR_LEN], old_c: &[f32; HRIR_LEN], win: &[f32]) -> (f32, f32) {
    let mut acc_new = [0.0f32; ACC_LANES];
    let mut acc_old = [0.0f32; ACC_LANES];
    for ((cn, co), h) in new_c
        .chunks_exact(ACC_LANES)
        .zip(old_c.chunks_exact(ACC_LANES))
        .zip(win.chunks_exact(ACC_LANES))
    {
        for lane in 0..ACC_LANES {
            let hv = h[lane];
            acc_new[lane] += cn[lane] * hv;
            acc_old[lane] += co[lane] * hv;
        }
    }
    (acc_new.iter().sum(), acc_old.iter().sum())
}

pub struct EarConvolver {
    /// Each input is stored twice so the last `HRIR_LEN` samples always form a
    /// contiguous ascending slice. `pos` is the primary slot of the newest
    /// sample.
    hist: [f32; 2 * HRIR_LEN],
    pos: usize,
    /// Current kernel in reverse order, aligned with the ascending history.
    rcoeffs: [f32; HRIR_LEN],
    /// The first real kernel has no audible predecessor and therefore installs
    /// immediately rather than fading from construction-time silence.
    initialized: bool,
    /// Fade-out kernel, also reversed.
    prev_rcoeffs: [f32; HRIR_LEN],
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
            hist: [0.0; 2 * HRIR_LEN],
            pos: 0,
            rcoeffs: [0.0; HRIR_LEN],
            initialized: false,
            prev_rcoeffs: [0.0; HRIR_LEN],
            fade_pos: 0,
            fade_len: 0,
        }
    }

    /// Replace the FIR kernel immediately without clearing history.
    #[inline]
    pub fn set_coeffs(&mut self, coeffs: &[f32; HRIR_LEN]) {
        for (dst, &c) in self.rcoeffs.iter_mut().zip(coeffs.iter().rev()) {
            *dst = c;
        }
        self.initialized = true;
        self.fade_pos = 0;
        self.fade_len = 0;
    }

    #[inline]
    fn kernel_is(&self, coeffs: &[f32; HRIR_LEN]) -> bool {
        self.rcoeffs.iter().eq(coeffs.iter().rev())
    }

    /// Crossfade from the current kernel to `coeffs`. The first kernel installs
    /// immediately so startup remains independent of the host callback size.
    pub fn set_coeffs_smooth(&mut self, coeffs: &[f32; HRIR_LEN], fade_len: usize) {
        if !self.initialized {
            self.set_coeffs(coeffs);
            return;
        }
        if self.kernel_is(coeffs) {
            return;
        }
        if fade_len == 0 {
            self.set_coeffs(coeffs);
            return;
        }
        if self.fade_pos < self.fade_len {
            let w = self.fade_pos as f32 / self.fade_len as f32;
            for i in 0..HRIR_LEN {
                self.prev_rcoeffs[i] += (self.rcoeffs[i] - self.prev_rcoeffs[i]) * w;
            }
        } else {
            self.prev_rcoeffs.copy_from_slice(&self.rcoeffs);
        }
        for (dst, &c) in self.rcoeffs.iter_mut().zip(coeffs.iter().rev()) {
            *dst = c;
        }
        self.fade_pos = 0;
        self.fade_len = fade_len as u32;
    }

    /// Reset stream-lifetime FIR history without allocating. The next HRIR is
    /// again treated as the first transfer function of the new stream.
    pub fn reset_runtime_state(&mut self) {
        self.hist.fill(0.0);
        self.pos = 0;
        self.rcoeffs.fill(0.0);
        self.prev_rcoeffs.fill(0.0);
        self.initialized = false;
        self.fade_pos = 0;
        self.fade_len = 0;
    }

    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        self.pos = if self.pos + 1 == HRIR_LEN {
            0
        } else {
            self.pos + 1
        };
        self.hist[self.pos] = x;
        self.hist[self.pos + HRIR_LEN] = x;
        let win = &self.hist[self.pos + 1..self.pos + 1 + HRIR_LEN];

        if self.fade_pos < self.fade_len {
            self.fade_pos += 1;
            let w = self.fade_pos as f32 / self.fade_len as f32;
            let (acc_new, acc_old) = dot2(&self.rcoeffs, &self.prev_rcoeffs, win);
            acc_old + (acc_new - acc_old) * w
        } else {
            dot(&self.rcoeffs, win)
        }
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
        k[3] = 1.0;
        c.set_coeffs(&k);
        let xs = [1.0, 0.0, 0.0, 0.0, 0.0];
        let ys: Vec<f32> = xs.iter().map(|&x| c.process(x)).collect();
        assert_eq!(ys, vec![0.0, 0.0, 0.0, 1.0, 0.0]);
    }

    #[test]
    fn last_tap_survives_history_wrap() {
        let mut c = EarConvolver::new();
        let mut k = [0.0; HRIR_LEN];
        k[HRIR_LEN - 1] = 1.0;
        c.set_coeffs(&k);
        let mut hits = Vec::new();
        for t in 0..(3 * HRIR_LEN) {
            let y = c.process(if t == 0 { 1.0 } else { 0.0 });
            if y != 0.0 {
                hits.push((t, y));
            }
        }
        assert_eq!(hits, vec![(HRIR_LEN - 1, 1.0)]);
    }

    #[test]
    fn representative_taps_map_to_their_delay() {
        for tap in [0, 1, 2, 7, 8, 63, HRIR_LEN - 2, HRIR_LEN - 1] {
            let mut c = EarConvolver::new();
            let mut k = [0.0; HRIR_LEN];
            k[tap] = 1.0;
            c.set_coeffs(&k);
            let mut hits = Vec::new();
            for t in 0..(2 * HRIR_LEN) {
                let y = c.process(if t == 0 { 1.0 } else { 0.0 });
                if y != 0.0 {
                    hits.push(t);
                }
            }
            assert_eq!(hits, vec![tap], "tap {tap} landed at the wrong delay");
        }
    }

    #[test]
    fn dc_gain_equals_coefficient_sum() {
        let mut c = EarConvolver::new();
        let mut k = [0.0; HRIR_LEN];
        k[0] = 0.5;
        k[1] = 0.25;
        c.set_coeffs(&k);
        let mut y = 0.0;
        for _ in 0..HRIR_LEN {
            y = c.process(1.0);
        }
        assert!((y - 0.75).abs() < 1e-6);
    }

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

    #[test]
    fn reset_makes_the_next_smooth_kernel_a_first_install_again() {
        let mut c = EarConvolver::new();
        let mut a = [0.0; HRIR_LEN];
        a[0] = 1.0;
        c.set_coeffs(&a);
        for _ in 0..32 {
            c.process(0.5);
        }
        c.reset_runtime_state();
        c.set_coeffs_smooth(&a, HRIR_LEN);
        assert_eq!(c.process(1.0), 1.0);
    }

    #[test]
    fn kernel_swap_crossfades_linearly() {
        let mut c = EarConvolver::new();
        let mut a = [0.0; HRIR_LEN];
        a[0] = 1.0;
        c.set_coeffs(&a);
        for _ in 0..HRIR_LEN {
            c.process(1.0);
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
            assert!(y < prev);
            prev = y;
        }
        assert!((c.process(1.0) - 0.5).abs() < 1e-6);
    }

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
        assert_eq!(c.process(1.0), 1.0);
    }

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
        c.set_coeffs_smooth(&b, 16);
        let mut y = 1.0;
        for _ in 0..8 {
            y = c.process(1.0);
        }
        assert!((y - 0.5).abs() < 1e-6);
        c.set_coeffs_smooth(&a, 16);
        let first = c.process(1.0);
        assert!((first - 0.5).abs() < 0.1, "restart jumped to {first}");
        for _ in 0..16 {
            y = c.process(1.0);
        }
        assert!((y - 1.0).abs() < 1e-6);
    }
}
