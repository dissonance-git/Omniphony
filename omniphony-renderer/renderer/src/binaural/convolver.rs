//! Direct-form FIR convolver for one ear of one object.
//!
//! Length is fixed at [`HRIR_LEN`]. The input-history ring buffer persists
//! across coefficient swaps, so updating the HRIR between frames (as the object
//! or head moves) keeps the output continuous. Direct-form FIR is the simplest
//! steady-state cost for short kernels; a partitioned-FFT path can replace this
//! in M2/M3 if profiling on the object count demands it.

use super::hrir::HRIR_LEN;

pub struct EarConvolver {
    /// Past inputs; `pos` marks the slot just written (most recent sample).
    hist: [f32; HRIR_LEN],
    pos: usize,
    coeffs: [f32; HRIR_LEN],
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
        }
    }

    /// Replace the FIR kernel. The history is untouched, so the response remains
    /// continuous through the swap.
    #[inline]
    pub fn set_coeffs(&mut self, coeffs: &[f32; HRIR_LEN]) {
        self.coeffs.copy_from_slice(coeffs);
    }

    /// Push one input sample and return the filtered output.
    #[inline]
    pub fn process(&mut self, x: f32) -> f32 {
        self.hist[self.pos] = x;
        let mut acc = 0.0f32;
        let mut idx = self.pos;
        for &c in self.coeffs.iter() {
            acc += c * self.hist[idx];
            idx = if idx == 0 { HRIR_LEN - 1 } else { idx - 1 };
        }
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
}
