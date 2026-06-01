//! Cross-crate shared handle for the post-rendering output pacer.
//!
//! The output side (this crate) owns the FIFOs and atomics. The input side
//! (`audio_input` crate, PipeWire input process callback) reads this handle
//! to drain pacer samples into the ring buffer at a cadence that exactly
//! matches the rate at which IEC958 chunks arrive — i.e. the source clock.
//! This breaks the decoder-batching burst pattern out of the ring buffer
//! signal that the PI servo consumes, without filtering anything downstream.
//!
//! Why a struct rather than passing the Arcs individually: the handle is
//! threaded through `InputControl::install_output_pacer`, which is called
//! by the decode lifecycle wiring once both the input PwStream and the
//! output PipewireWriter exist. Passing one struct keeps the wiring stable
//! as fields evolve.

use crossbeam::queue::ArrayQueue;
use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

/// Shared state allowing the PipeWire input thread to drain rendered samples
/// from the output-side pacer FIFO into the ring buffer in lockstep with
/// the IEC958 input chunk arrival cadence.
#[derive(Clone)]
pub struct PacerHandle {
    /// Producer: renderer thread (via `PipewireWriter::write_samples`).
    /// Consumer: PipeWire input thread (drain step).
    pub pacer_fifo: Arc<ArrayQueue<f32>>,
    /// The audio ring the DAC callback consumes. Drain target.
    pub ring: Arc<ArrayQueue<f32>>,
    /// `false` until `pacer_fifo` has accumulated at least
    /// `pre_roll_threshold_samples` — until then the input-thread drain
    /// pushes silence into the ring instead of popping from the FIFO,
    /// guaranteeing the FIFO is never drained below the minimum safety
    /// margin once "real" draining begins.
    pub pre_roll_complete: Arc<AtomicBool>,
    /// Samples (across all channels) that must accumulate in `pacer_fifo`
    /// before the drain switches from silence to real audio. Sized to
    /// exceed one full decoded AU (~32 ms at 48 kHz × 8 ch ≈ 12 288 samples)
    /// with comfortable margin.
    pub pre_roll_threshold_samples: usize,
    /// Output sample rate (Hz) used to compute the per-chunk drain quantum.
    pub out_sample_rate: u32,
    /// Output channel count used to compute the per-chunk drain quantum.
    pub out_channels: u32,
    /// Live mirror of `AdaptiveResamplingConfig::use_output_pacing`. When
    /// `false`, the input-thread drain is a no-op and the writer pushes
    /// directly to `ring` — i.e. legacy behaviour. Toggling this atomic is
    /// the hot-swap mechanism for the feature flag.
    pub enabled: Arc<AtomicBool>,
    /// Diagnostic: cumulative number of samples drawn from the FIFO (real
    /// audio + zero fills). f64-encoded so the diag plot can read it like
    /// any other atomic metric.
    pub diag_drain_total: Arc<AtomicU64>,
    /// Diagnostic: cumulative number of zero-fill samples emitted because
    /// the FIFO underran. f64-encoded. If non-zero in steady state,
    /// `pre_roll_threshold_samples` is too low.
    pub diag_underrun_total: Arc<AtomicU64>,
    /// Diagnostic: instantaneous FIFO occupancy (in samples, f64-encoded).
    /// Should oscillate around `pre_roll_threshold_samples` in steady state
    /// — bottoms out near zero just before each decoder AU lands.
    pub diag_fifo_level: Arc<AtomicU64>,
}

impl PacerHandle {
    /// Flush the FIFO and re-arm the pre-roll. Called on
    /// `recovery_reacquire_pending` consumption or codec switch from the
    /// DAC callback (output side); the input callback observes the
    /// `pre_roll_complete` atomic and adapts on the next chunk.
    pub fn flush_and_rearm(&self) {
        while self.pacer_fifo.pop().is_some() {}
        self.pre_roll_complete
            .store(false, std::sync::atomic::Ordering::Relaxed);
    }

    /// Move `drain_samples` (across all channels) from the pacer FIFO into the
    /// ring. Honours pre-roll (pushes silence until the FIFO is primed) and
    /// zero-fills on underrun. `drain_samples` should be a whole number of
    /// frames (i.e. a multiple of the output channel count) so the ring's
    /// channel interleaving stays aligned.
    ///
    /// Callers gate on [`PacerHandle::enabled`] before invoking this. Both the
    /// PipeWire input RT callback (Live / PipewireBridge modes) and the pure
    /// pipe-bridge drain thread share this single drain implementation; the
    /// difference is only how each computes `drain_samples` and what clock
    /// drives the call.
    pub fn drain(&self, drain_samples: usize) {
        let mut underruns = 0u64;
        let priming = !self.pre_roll_complete.load(Ordering::Relaxed);
        if priming {
            if self.pacer_fifo.len() >= self.pre_roll_threshold_samples {
                self.pre_roll_complete.store(true, Ordering::Relaxed);
            } else {
                for _ in 0..drain_samples {
                    let _ = self.ring.push(0.0);
                }
                underruns += drain_samples as u64;
            }
        }
        if !priming || self.pre_roll_complete.load(Ordering::Relaxed) {
            for _ in 0..drain_samples {
                let value = self.pacer_fifo.pop().unwrap_or_else(|| {
                    underruns += 1;
                    0.0
                });
                let _ = self.ring.push(value);
            }
        }
        let prev_drain = f64::from_bits(self.diag_drain_total.load(Ordering::Relaxed));
        self.diag_drain_total.store(
            (prev_drain + drain_samples as f64).to_bits(),
            Ordering::Relaxed,
        );
        if underruns > 0 {
            let prev_under = f64::from_bits(self.diag_underrun_total.load(Ordering::Relaxed));
            self.diag_underrun_total
                .store((prev_under + underruns as f64).to_bits(), Ordering::Relaxed);
        }
        self.diag_fifo_level
            .store((self.pacer_fifo.len() as f64).to_bits(), Ordering::Relaxed);
    }
}
