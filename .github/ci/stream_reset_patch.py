from pathlib import Path

ROOT = Path("omniphony-renderer")
DELAY = ROOT / "renderer/src/delay_line.rs"
CONV = ROOT / "renderer/src/binaural/convolver.rs"
REFL = ROOT / "renderer/src/binaural/reflections.rs"
REVERB = ROOT / "renderer/src/binaural/reverb.rs"
BINAURAL = ROOT / "renderer/src/binaural/mod.rs"
COMPONENTS = ROOT / "renderer/src/spatial_renderer/components.rs"
SPATIAL = ROOT / "renderer/src/spatial_renderer/mod.rs"
FIXTURE_LIB = ROOT / "dsp_fixtures/src/lib.rs"
RESET_FIXTURE = ROOT / "dsp_fixtures/src/stream_reset.rs"


def replace_exact(path: Path, old: str, new: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one source fragment, found {count}")
    path.write_text(text.replace(old, new, 1), encoding="utf-8")
    print(f"patched {label}")


def insert_before(path: Path, marker: str, addition: str, label: str) -> None:
    text = path.read_text(encoding="utf-8")
    count = text.count(marker)
    if count != 1:
        raise SystemExit(f"{label}: expected exactly one marker, found {count}")
    path.write_text(text.replace(marker, addition + marker, 1), encoding="utf-8")
    print(f"patched {label}")


def main() -> None:
    insert_before(
        DELAY,
        "    /// Process one sample through the delay line.\n",
        """    /// Reset stream-lifetime history in place. Capacity is retained so a
    /// decoder seek/track restart cannot leak old delayed samples into the new
    /// stream and does not allocate or free on the realtime thread.
    pub fn reset_runtime_state(&mut self) {
        self.buf.fill(0.0);
        self.write_pos = 0;
        self.current = 0.0;
        self.target = 0.0;
    }

""",
        "DelayLine in-place reset",
    )

    insert_before(
        CONV,
        "    /// Push one input sample and return the filtered output.\n",
        """    /// Reset stream-lifetime FIR history without reallocating. The next HRIR
    /// installation is treated as the first transfer function of the new stream,
    /// so it installs immediately rather than crossfading from stale geometry.
    pub fn reset_runtime_state(&mut self) {
        self.hist.fill(0.0);
        self.pos = 0;
        self.coeffs.fill(0.0);
        self.prev_coeffs.fill(0.0);
        self.initialized = false;
        self.fade_pos = 0;
        self.fade_len = 0;
    }

""",
        "EarConvolver in-place reset",
    )

    insert_before(
        REFL,
        "    /// Write one input sample and return the summed (left, right) reflection\n",
        """    /// Reset one logical stream while retaining the preallocated reflection ring.
    pub fn reset_runtime_state(&mut self) {
        self.ring.fill(0.0);
        self.write_pos = 0;
        self.taps_l = Default::default();
        self.taps_r = Default::default();
    }

""",
        "ReflectionBank in-place reset",
    )

    insert_before(
        REVERB,
        "    /// Start the next fixed-length modulation segment. This is called by sample\n",
        """    /// Reset stored acoustic history at a discontinuous stream boundary while
    /// retaining all delay-line allocations and the currently configured room.
    pub fn reset_runtime_state(&mut self) {
        for line in &mut self.lines {
            line.fill(0.0);
        }
        self.pos = [0; N];
        self.damp_state = [0.0; N];
        self.cur_delay = self.base_len;
        self.mod_step = [0.0; N];
        self.mod_samples_left = 0;
        self.xover_state = Default::default();
        self.predelay.fill(0.0);
        self.pre_pos = 0;
    }

""",
        "FDN in-place reset",
    )

    insert_before(
        COMPONENTS,
        "    /// Advance the gain slew by one block toward `target`; returns the\n",
        """    pub(super) fn reset_runtime_state(&mut self) {
        self.initialized = false;
        self.gain_db = -128;
        self.slewed_gain = 0.0;
        self.ramp = ChannelRampState::default();
        self.interp_prev_gains.clear();
    }

""",
        "ChannelState in-place reset",
    )

    insert_before(
        BINAURAL,
        "/// Tagged result from the asynchronous HRIR worker.\n",
        """impl ChannelDsp {
    fn reset_runtime_state(&mut self) {
        self.delay_l.reset_runtime_state();
        self.delay_r.reset_runtime_state();
        self.conv_l.reset_runtime_state();
        self.conv_r.reset_runtime_state();
        if let Some(bank) = self.refl.as_mut() {
            bank.reset_runtime_state();
        }
        self.air_state = 0.0;
        self.air_coeff = 0.0;
    }
}

""",
        "ChannelDsp in-place reset",
    )

    insert_before(
        BINAURAL,
        "    /// Identity of the active HRIR grid (tests observe the async swap with it).\n",
        """    /// Reset one discontinuous audio stream while preserving expensive immutable
    /// configuration: the selected HRIR grid and rebuild worker remain alive,
    /// while every sample-history state is cleared in place.
    pub fn reset_runtime_state(&mut self) {
        for channel in &mut self.channels {
            if let Some(dsp) = channel.as_mut() {
                dsp.reset_runtime_state();
            }
        }
        self.channel_gain_boundary.fill(0.0);
        if let Some(fdn) = self.fdn.as_mut() {
            fdn.reset_runtime_state();
        }
        self.reverb_bus.fill(0.0);
    }

""",
        "BinauralRenderer stream reset",
    )

    replace_exact(
        SPATIAL,
        """        if self
            .reset_requested
            .swap(false, std::sync::atomic::Ordering::Acquire)
        {
            self.channel_states.clear();
        }
""",
        """        if self
            .reset_requested
            .swap(false, std::sync::atomic::Ordering::Acquire)
        {
            // A new logical stream owns a new audio history. Preserve allocated
            // storage/configuration, but clear every state that can contain old
            // samples or an old trajectory. This keeps reset bounded and avoids
            // allocator work in the realtime callback.
            for state in &mut self.channel_states {
                state.reset_runtime_state();
            }
            for delay in &mut self.delay_lines {
                delay.reset_runtime_state();
            }
            for slot in &mut self.crossover_filter_states {
                if let Some(states) = slot.as_mut() {
                    for state in states {
                        *state = BiquadState::default();
                    }
                }
            }
            self.binaural.reset_runtime_state();
        }
""",
        "whole-renderer stream reset",
    )

    replace_exact(
        SPATIAL,
        """    pub fn reset_runtime_state(&self) {
        self.reset_requested
            .store(true, std::sync::atomic::Ordering::Release);
        self.first_render
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
""",
        """    pub fn reset_runtime_state(&self) {
        // Dialogue normalization belongs to the old stream unless the new one
        // explicitly publishes its own value. Keep session-level master/auto-gain
        // policy, but return per-stream loudness correction to neutral immediately.
        self.loudness_gain
            .store(1.0f32.to_bits(), std::sync::atomic::Ordering::Relaxed);
        self.reset_requested
            .store(true, std::sync::atomic::Ordering::Release);
        self.first_render
            .store(true, std::sync::atomic::Ordering::Relaxed);
    }
""",
        "per-stream loudness reset",
    )

    lib_text = FIXTURE_LIB.read_text(encoding="utf-8")
    marker = "pub mod scene;\n"
    if lib_text.count(marker) != 1:
        raise SystemExit("fixture module marker changed")
    FIXTURE_LIB.write_text(
        lib_text.replace(marker, marker + "pub mod stream_reset;\n", 1),
        encoding="utf-8",
    )

    if RESET_FIXTURE.exists():
        raise SystemExit("stream_reset fixture already exists")
    RESET_FIXTURE.write_text(
        """//! Always-on stream-boundary fidelity regressions.
//!
//! A decoder seek, track change or explicit stream restart is a discontinuity in
//! the canonical audio timeline. Old FIR/delay/reflection/reverb energy must not
//! leak into the first samples of the next stream, but immutable renderer/HRTF
//! configuration should stay warm.

use crate::scene::{SAMPLE_RATE, build_renderer_binaural, pseudo};
use renderer::live_params::{OutputMode, RampMode};
use renderer::spatial_renderer::SpatialChannelEvent;
use renderer::speaker_layout::SpeakerLayout;

fn object_event() -> SpatialChannelEvent {
    SpatialChannelEvent {
        channel_idx: 0,
        is_bed: false,
        gain_db: Some(0),
        ramp_length: Some(0),
        size: Some([0.0, 0.0, 0.0]),
        position: Some([0.55, 0.82, 0.18]),
        sample_pos: Some(0),
    }
}

#[test]
fn stream_reset_clears_previous_binaural_audio_history() {
    let mut renderer = build_renderer_binaural(
        SpeakerLayout::preset("7.1.4").expect("known preset"),
        true,
        false,
    );
    {
        let control = renderer.renderer_control();
        control.set_requested_ramp_mode(RampMode::Sample);
        let mut live = control.live.write();
        live.ramp_mode = RampMode::Sample;
        live.binaural.output_mode = OutputMode::Binaural;
        live.binaural.reflections.enabled = true;
        live.binaural.reflections.level = 0.7;
        live.binaural.reverb.enabled = true;
        live.binaural.reverb.level = 0.5;
        live.binaural.reverb.rt60_s = 0.6;
        live.binaural.reverb.predelay_ms = 0.0;
    }

    let event = object_event();
    let block = 128usize;
    let total = 2_560usize;
    let mut reuse = Vec::new();
    for start in (0..total).step_by(block) {
        let pcm: Vec<f32> = (0..block)
            .map(|s| pseudo((50_000 + start + s) as u64) * 0.35)
            .collect();
        let events: &[SpatialChannelEvent] = if start == 0 {
            std::slice::from_ref(&event)
        } else {
            &[]
        };
        let frame = renderer
            .render_frame(&pcm, 1, events, reuse, false)
            .expect("fill direct and room history");
        reuse = frame.samples;
        reuse.clear();
    }

    renderer.reset_runtime_state();

    // Keep the new stream's channel logically active while feeding exact silence.
    // Without a whole-pipeline reset, old FIR/ITD/reflection/FDN history can leak
    // into this buffer even though the new PCM contains no energy.
    let silence = vec![0.0f32; 1_024];
    let frame = renderer
        .render_frame(
            &silence,
            1,
            std::slice::from_ref(&event),
            reuse,
            false,
        )
        .expect("render first silent block of new stream");

    let peak = frame
        .samples
        .iter()
        .fold(0.0f32, |m, &x| m.max(x.abs()));
    eprintln!("post-reset stale-history peak={peak:.9}");
    assert!(
        peak <= 1.0e-8,
        "old audio history leaked across stream reset: peak={peak:.9}"
    );
    assert_eq!(SAMPLE_RATE, 48_000);
}
""",
        encoding="utf-8",
    )
    print("created stream reset end-to-end fixture")


if __name__ == "__main__":
    main()
