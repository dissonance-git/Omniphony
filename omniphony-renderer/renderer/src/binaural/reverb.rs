//! Late-reverberation tail for the binaural stage: a small stereo FDN
//! (feedback delay network).
//!
//! Purpose: the direct-to-reverberant ratio is the dominant distance cue past
//! ~1 m, and six discrete first-order reflections cannot provide the dense
//! decaying tail a real room has. This FDN models the LISTENING room — a
//! small, fairly dry space, constant across content (like the room around a
//! loudspeaker setup) — NOT the acoustics of the scene, which are already in
//! the mix and pass through untouched.
//!
//! Topology: one mono input bus (per-channel sends, summed by the caller) →
//! pre-delay → 8 mutually-prime delay lines with one-pole HF damping in the
//! feedback path, mixed by a Householder matrix (O(N) per sample) → two
//! sign-pattern output taps for decorrelated L/R. In a real room the
//! reverberant field level is roughly independent of source distance. The
//! direct object level is authored (Atmos) and never 1/d-attenuated here, so
//! instead the caller raises the per-source reverb send with distance
//! (near-field roll-in): the DRR falls with distance without ever touching the
//! direct object level.

/// Number of delay lines.
const N: usize = 8;

/// Delay line lengths in samples at 48 kHz (mutually prime, ~21…62 ms),
/// scaled linearly for other rates.
const LENGTHS_48K: [usize; N] = [1031, 1327, 1523, 1801, 2053, 2311, 2617, 2903];

/// Output tap sign patterns (orthogonal rows of a Hadamard-ish matrix) for
/// decorrelated left/right returns.
const SIGNS_L: [f32; N] = [1.0, -1.0, 1.0, 1.0, -1.0, 1.0, -1.0, -1.0];
const SIGNS_R: [f32; N] = [1.0, 1.0, -1.0, 1.0, 1.0, -1.0, -1.0, 1.0];

/// One-pole HF damping coefficient in the feedback path (higher = darker
/// tail; HF decays faster than the broadband RT60, like physical walls).
const DAMPING: f32 = 0.35;

pub struct Fdn {
    lines: Vec<Vec<f32>>,
    pos: [usize; N],
    damp_state: [f32; N],
    /// Per-line feedback gain derived from the current RT60.
    fb_gain: [f32; N],
    predelay: Vec<f32>,
    pre_pos: usize,
    pre_len: usize,
    sample_rate: u32,
    rt60_cached: f32,
}

impl Fdn {
    pub fn new(sample_rate: u32) -> Self {
        let scale = sample_rate as f32 / 48_000.0;
        let lines: Vec<Vec<f32>> = LENGTHS_48K
            .iter()
            .map(|&l| vec![0.0f32; ((l as f32 * scale) as usize).max(16)])
            .collect();
        // 120 ms pre-delay capacity; the active length is set per block.
        let pre_cap = (sample_rate as usize * 120 / 1000).max(16);
        Self {
            lines,
            pos: [0; N],
            damp_state: [0.0; N],
            fb_gain: [0.5; N],
            predelay: vec![0.0; pre_cap],
            pre_pos: 0,
            pre_len: 1,
            sample_rate,
            rt60_cached: 0.0,
        }
    }

    /// Per-block parameter update: RT60 (s) → per-line feedback gains, and the
    /// active pre-delay length.
    pub fn set_params(&mut self, rt60_s: f32, predelay_ms: f32) {
        let rt60 = rt60_s.clamp(0.1, 3.0);
        if (rt60 - self.rt60_cached).abs() > 1e-6 {
            self.rt60_cached = rt60;
            for (i, line) in self.lines.iter().enumerate() {
                // g = 10^(-3 * delay / (rt60 * sr)) → -60 dB after rt60 seconds.
                let exp = -3.0 * line.len() as f32 / (rt60 * self.sample_rate as f32);
                self.fb_gain[i] = 10.0f32.powf(exp);
            }
        }
        let len = (predelay_ms.clamp(0.0, 100.0) * self.sample_rate as f32 / 1000.0) as usize;
        self.pre_len = len.clamp(1, self.predelay.len() - 1);
    }

    /// Process one block: read `bus` (mono send sum, one sample per frame) and
    /// ADD the stereo return × `level` into `out` (interleaved L/R).
    pub fn process_block(&mut self, bus: &[f32], level: f32, out: &mut [f32]) {
        debug_assert!(out.len() >= bus.len() * 2);
        // Normalise the output taps (N lines, ±1 signs) and fold the level in.
        let out_gain = level / (N as f32).sqrt();
        for (s, &input) in bus.iter().enumerate() {
            // Pre-delay (integer, fixed per block).
            let read = (self.pre_pos + self.predelay.len() - self.pre_len) % self.predelay.len();
            let x = self.predelay[read];
            self.predelay[self.pre_pos] = input;
            self.pre_pos = (self.pre_pos + 1) % self.predelay.len();

            // Read all line outputs.
            let mut o = [0.0f32; N];
            let mut sum = 0.0f32;
            for i in 0..N {
                o[i] = self.lines[i][self.pos[i]];
                sum += o[i];
            }

            // Output taps.
            let mut l = 0.0f32;
            let mut r = 0.0f32;
            for i in 0..N {
                l += SIGNS_L[i] * o[i];
                r += SIGNS_R[i] * o[i];
            }
            let oidx = s * 2;
            out[oidx] += l * out_gain;
            out[oidx + 1] += r * out_gain;

            // Householder feedback: H·o = o − (2/N)·Σo, then damping + gain,
            // plus the (sign-alternated) input injection.
            let k = 2.0 / N as f32 * sum;
            for i in 0..N {
                let fb = o[i] - k;
                // One-pole low-pass in the loop: HF dies faster than RT60.
                self.damp_state[i] += (fb - self.damp_state[i]) * (1.0 - DAMPING);
                let inject = if i % 2 == 0 { x } else { -x };
                self.lines[i][self.pos[i]] = self.damp_state[i] * self.fb_gain[i] + inject;
                self.pos[i] = (self.pos[i] + 1) % self.lines[i].len();
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn tail_energy(out: &[f32], from: usize, to: usize) -> f32 {
        out[from * 2..to * 2].iter().map(|x| x * x).sum()
    }

    #[test]
    fn impulse_produces_a_decaying_tail() {
        let mut fdn = Fdn::new(48_000);
        fdn.set_params(0.4, 10.0);
        let n = 48_000; // 1 s
        let mut bus = vec![0.0f32; n];
        bus[0] = 1.0;
        let mut out = vec![0.0f32; n * 2];
        fdn.process_block(&bus, 1.0, &mut out);

        // Dense energy well past the early-reflection window…
        let mid = tail_energy(&out, 5_000, 15_000); // ~104…312 ms
        assert!(mid > 1e-6, "no late tail: {mid}");
        // …and decaying: the second half of the second must be much quieter.
        let late = tail_energy(&out, 30_000, 40_000);
        assert!(late < mid * 0.5, "tail not decaying: mid={mid} late={late}");
    }

    #[test]
    fn rt60_controls_decay_speed() {
        let render = |rt60: f32| -> (f32, f32) {
            let mut fdn = Fdn::new(48_000);
            fdn.set_params(rt60, 5.0);
            let n = 48_000;
            let mut bus = vec![0.0f32; n];
            bus[0] = 1.0;
            let mut out = vec![0.0f32; n * 2];
            fdn.process_block(&bus, 1.0, &mut out);
            (
                tail_energy(&out, 4_000, 8_000),
                tail_energy(&out, 24_000, 28_000),
            )
        };
        let (short_early, short_late) = render(0.2);
        let (long_early, long_late) = render(1.5);
        // Both ring early; the long RT60 must retain much more late energy
        // relative to its early energy than the short one.
        let short_ratio = short_late / short_early.max(1e-12);
        let long_ratio = long_late / long_early.max(1e-12);
        assert!(
            long_ratio > short_ratio * 10.0,
            "rt60 had no effect: short={short_ratio} long={long_ratio}"
        );
    }

    #[test]
    fn left_and_right_returns_are_decorrelated() {
        let mut fdn = Fdn::new(48_000);
        fdn.set_params(0.5, 5.0);
        let n = 24_000;
        let mut bus = vec![0.0f32; n];
        bus[0] = 1.0;
        let mut out = vec![0.0f32; n * 2];
        fdn.process_block(&bus, 1.0, &mut out);
        let l: Vec<f32> = out.iter().step_by(2).copied().collect();
        let r: Vec<f32> = out.iter().skip(1).step_by(2).copied().collect();
        let dot: f32 = l.iter().zip(&r).map(|(a, b)| a * b).sum();
        let el: f32 = l.iter().map(|x| x * x).sum();
        let er: f32 = r.iter().map(|x| x * x).sum();
        let corr = dot / (el.sqrt() * er.sqrt()).max(1e-12);
        assert!(
            corr.abs() < 0.35,
            "L/R reverb returns too correlated: {corr}"
        );
    }
}
