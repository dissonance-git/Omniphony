from pathlib import Path
import re

root = Path("omniphony-renderer")


def replace_once(path: str, old: str, new: str) -> None:
    p = root / path
    s = p.read_text()
    count = s.count(old)
    if count != 1:
        raise SystemExit(f"{path}: expected exactly one replacement, found {count}")
    p.write_text(s.replace(old, new, 1))


# 1) Zero-delay delay-line fast path must still retain history so a later
# live delay change has the same past samples as process().
replace_once(
    "renderer/src/delay_line.rs",
    """    pub fn is_bypass(&self) -> bool {
        self.target == 0.0 && self.current == 0.0
    }

    /// Reset stream-lifetime history in place.""",
    """    pub fn is_bypass(&self) -> bool {
        self.target == 0.0 && self.current == 0.0
    }

    /// Keep the ring warm without doing the fractional read.
    ///
    /// While [`is_bypass`](Self::is_bypass) holds, `process` is the identity.
    /// The write still matters because a later non-zero delay must be able to
    /// read audio that passed while the line was bypassed.
    #[inline]
    pub fn push_history(&mut self, input: f32) {
        debug_assert!(
            self.is_bypass(),
            "push_history is only the identity while bypassed",
        );
        self.buf[self.write_pos] = input;
        self.write_pos = (self.write_pos + 1) % self.buf.len();
    }

    /// Reset stream-lifetime history in place.""",
)
replace_once(
    "renderer/src/delay_line.rs",
    """    #[test]
    fn fractional_delay_just_above_write_pos_does_not_panic() {""",
    """    #[test]
    fn bypass_history_matches_full_processing_when_delay_turns_on() {
        let mut fast = DelayLine::new(32);
        let mut reference = DelayLine::new(32);
        for i in 0..24 {
            let x = ((i * 7 % 17) as f32 - 8.0) / 8.0;
            fast.push_history(x);
            assert_eq!(reference.process(x).to_bits(), x.to_bits());
        }
        fast.set_target_ms(4.0 / 48.0, 48_000);
        reference.set_target_ms(4.0 / 48.0, 48_000);
        for i in 0..32 {
            let x = ((i * 11 % 19) as f32 - 9.0) / 9.0;
            assert_eq!(fast.process(x).to_bits(), reference.process(x).to_bits());
        }
    }

    #[test]
    fn fractional_delay_just_above_write_pos_does_not_panic() {""",
)

# 2) Speaker-major finalisation keeps one delay ring cache-hot and avoids the
# fractional-read path entirely for the zero-delay common case.
p = root / "renderer/src/spatial_renderer/speaker_stage.rs"
s = p.read_text()
pat = re.compile(
    r"""    pub\(super\) fn finalize_output\(
        &mut self,
        speaker_params: &\[crate::live_params::SpeakerLiveParams\],
        total_gain: f32,
        output: &mut \[f32\],
    \) -> \(f32, usize\) \{.*?
    \}

    /// Push the live read-time interpolation flag""",
    re.S,
)
repl = """    pub(super) fn finalize_output(
        &mut self,
        speaker_params: &[crate::live_params::SpeakerLiveParams],
        total_gain: f32,
        output: &mut [f32],
    ) -> (f32, usize) {
        self.speaker_gains_buf
            .iter_mut()
            .enumerate()
            .for_each(|(idx, g)| {
                let sp = speaker_params.get(idx);
                *g = if sp.is_some_and(|s| s.muted) {
                    0.0
                } else {
                    total_gain * sp.map_or(1.0, |s| s.gain)
                };
            });
        for (idx, dl) in self.delay_lines.iter_mut().enumerate() {
            dl.set_target_ms(
                speaker_params.get(idx).map_or(0.0, |s| s.delay_ms),
                self.sample_rate,
            );
        }
        let speaker_total_gains = &self.speaker_gains_buf;

        // Speaker-major rather than sample-major: each delay ring stays hot in
        // cache, and the zero-delay common case bypasses the fractional read
        // while still preserving history for a later live delay change.
        let num_speakers = self.num_speakers;
        let sample_length = output.len() / num_speakers.max(1);
        let mut peak_sample: f32 = 0.0;
        let mut peak_speaker_idx: usize = 0;
        for (speaker_idx, delay_line) in self
            .delay_lines
            .iter_mut()
            .enumerate()
            .take(num_speakers)
        {
            let gain = speaker_total_gains[speaker_idx];
            let mut peak: f32 = 0.0;
            if delay_line.is_bypass() {
                for sample_idx in 0..sample_length {
                    let sample = &mut output[sample_idx * num_speakers + speaker_idx];
                    *sample *= gain;
                    delay_line.push_history(*sample);
                    peak = peak.max(sample.abs());
                }
            } else {
                for sample_idx in 0..sample_length {
                    let sample = &mut output[sample_idx * num_speakers + speaker_idx];
                    *sample = delay_line.process(*sample * gain);
                    peak = peak.max(sample.abs());
                }
            }
            if peak > peak_sample {
                peak_sample = peak;
                peak_speaker_idx = speaker_idx;
            }
        }
        (peak_sample, peak_speaker_idx)
    }

    /// Push the live read-time interpolation flag"""
s2, n = pat.subn(repl, s, count=1)
if n != 1:
    raise SystemExit(f"speaker_stage.rs: finalize_output replacement count {n}")
p.write_text(s2)

# 3) Track which asynchronous HRIR request is actually live. This lets
# validation wait on state rather than a wall-clock sleep.
replace_once(
    "renderer/src/binaural/mod.rs",
    """    /// HRIR source last *requested* (the active grid may briefly lag it while
    /// the worker builds — see [`Self::ensure_source`]).
    source: HrirSource,
    /// Finished, source-tagged grids from the rebuild worker, awaiting the""",
    """    /// HRIR source last *requested* (the active grid may briefly lag it while
    /// the worker builds — see [`Self::ensure_source`]).
    source: HrirSource,
    /// Source identity of the grid currently convolving audio. Kept separate
    /// from `source` so validation can observe the asynchronous handoff.
    active_source: HrirSource,
    /// Finished, source-tagged grids from the rebuild worker, awaiting the""",
)
replace_once(
    "renderer/src/binaural/mod.rs",
    """            hrir: std::sync::Arc::new(Self::build_hrir(&source, sample_rate)),
            source,
            incoming,""",
    """            hrir: std::sync::Arc::new(Self::build_hrir(&source, sample_rate)),
            active_source: source.clone(),
            source,
            incoming,""",
)
replace_once(
    "renderer/src/binaural/mod.rs",
    """            if built.source == self.source {
                self.hrir = std::sync::Arc::clone(&built.set);
                // Same direction on a different HRTF set is a different kernel.""",
    """            if built.source == self.source {
                self.active_source = built.source.clone();
                self.hrir = std::sync::Arc::clone(&built.set);
                // Same direction on a different HRTF set is a different kernel.""",
)
replace_once(
    "renderer/src/binaural/mod.rs",
    """    /// Render one frame to interleaved stereo.""",
    """    /// Whether the requested HRIR source is still being built or waiting
    /// for the audio-thread swap. Steady state is one enum comparison.
    pub fn rebuild_pending(&self) -> bool {
        self.source != self.active_source
    }

    /// Render one frame to interleaved stereo.""",
)

# Expose the state through SpatialRenderer for the dev-only fixture.
replace_once(
    "renderer/src/spatial_renderer/mod.rs",
    """    pub fn num_speakers(&self) -> usize {
        self.num_speakers
    }""",
    """    /// True while a requested binaural HRIR grid has not yet become active.
    /// Primarily used by deterministic validation; the production swap remains
    /// asynchronous and never blocks the audio thread.
    pub fn binaural_rebuild_pending(&self) -> bool {
        self.binaural.rebuild_pending()
    }

    pub fn num_speakers(&self) -> usize {
        self.num_speakers
    }""",
)

# 4) Replace the scheduler-dependent 100 ms sleep with a render-driven state
# barrier, then prime from fixed seeds after the right grid lands.
p = root / "dsp_fixtures/src/scene.rs"
s = p.read_text()
pat = re.compile(
    r"""pub fn render_single_object_binaural\(
    azimuth_deg: f32,
    blocks: usize,
    hrir_source: HrirSource,
\) -> \(Vec<f32>, Vec<f32>\) \{.*?
\}

#\[cfg\(test\)\]""",
    re.S,
)
repl = """pub fn render_single_object_binaural(
    azimuth_deg: f32,
    blocks: usize,
    hrir_source: HrirSource,
) -> (Vec<f32>, Vec<f32>) {
    const PRIME_BLOCKS: usize = 64;

    let theta = (azimuth_deg as f64).to_radians();
    let position = [theta.sin(), theta.cos(), 0.0];

    let mut r = build_renderer_binaural(
        SpeakerLayout::preset("7.1.4").expect("known preset"),
        true,
        false,
    );
    {
        let ctrl = r.renderer_control();
        ctrl.set_requested_ramp_mode(RampMode::Frame);
        let mut live = ctrl.live.write();
        live.ramp_mode = RampMode::Frame;
        live.binaural.output_mode = OutputMode::Binaural;
        live.binaural.hrir_source = hrir_source.clone();
    }

    let event = vec![SpatialChannelEvent {
        channel_idx: 0,
        is_bed: false,
        gain_db: Some(0),
        ramp_length: Some(BLOCK_SAMPLES as u32),
        size: Some([0.0, 0.0, 0.0]),
        position: Some(position),
        sample_pos: Some(0),
    }];

    let mut buf = Vec::new();
    let render_one = |r: &mut SpatialRenderer, buf: Vec<f32>, seed: usize| {
        let frame = r
            .render_frame(&make_pcm_block(1, seed), 1, &event, buf, false)
            .expect("binaural ITD render");
        let mut samples = frame.samples;
        samples.clear();
        samples
    };

    // Stage 1: drive the audio thread until the specifically requested HRIR
    // grid is active. The first render issues the async request, and later
    // renders are what consume the worker result.
    const SETTLE_SEED_BASE: usize = 1 << 20;
    let deadline = std::time::Instant::now() + std::time::Duration::from_secs(30);
    let mut settled = 0usize;
    buf = render_one(&mut r, buf, SETTLE_SEED_BASE);
    while r.binaural_rebuild_pending() {
        assert!(
            std::time::Instant::now() < deadline,
            "binaural HRIR rebuild for {hrir_source:?} never landed"
        );
        std::thread::yield_now();
        settled += 1;
        buf = render_one(&mut r, buf, SETTLE_SEED_BASE + settled);
    }

    // Stage 2: now prime from a fixed excitation sequence. The measured state
    // is therefore independent of how many scheduler-dependent settle frames
    // were needed above.
    for block in 0..PRIME_BLOCKS {
        buf = render_one(&mut r, buf, block);
    }

    let mut left = Vec::with_capacity(blocks * BLOCK_SAMPLES);
    let mut right = Vec::with_capacity(blocks * BLOCK_SAMPLES);
    for block in 0..blocks {
        let frame = r
            .render_frame(
                &make_pcm_block(1, PRIME_BLOCKS + block),
                1,
                &event,
                buf,
                false,
            )
            .expect("binaural ITD render");
        for pair in frame.samples.chunks_exact(2) {
            left.push(pair[0]);
            right.push(pair[1]);
        }
        buf = frame.samples;
        buf.clear();
    }
    (left, right)
}

#[cfg(test)]"""
s2, n = pat.subn(repl, s, count=1)
if n != 1:
    raise SystemExit(f"scene.rs: render_single_object_binaural replacement count {n}")
p.write_text(s2)

# 5) The existing Interp regression must now respect the intentional output-
# mode fade rather than assume live request == current frame.
p = root / "renderer/src/spatial_renderer/tests.rs"
s = p.read_text()
pat = re.compile(
    r"""#\[test\]
fn interp_survives_speaker_cascade_width_switch\(\) \{.*?
\}

// TODO: Add integration test""",
    re.S,
)
repl = """#[test]
fn interp_survives_speaker_cascade_width_switch() {
    let mut r = build_cascade_test_renderer(LiveEvaluationMode::PrecomputedCartesian, false);
    {
        let ctrl = r.control.clone();
        ctrl.set_requested_ramp_mode(crate::live_params::RampMode::Interp);
        let mut live = ctrl.live.write();
        live.ramp_mode = crate::live_params::RampMode::Interp;
        live.binaural.mode = crate::live_params::BinauralMode::Cascaded;
    }
    let pcm: Vec<f32> = (0..40).map(|i| (i * 7 % 13) as f32 / 13.0 - 0.5).collect();
    let event = vec![SpatialChannelEvent {
        channel_idx: 0,
        is_bed: false,
        gain_db: Some(0),
        ramp_length: Some(40),
        size: Some([0.0, 0.0, 0.0]),
        position: Some([0.4, 0.6, 0.2]),
        sample_pos: Some(0),
    }];

    let set_mode = |r: &mut SpatialRenderer, mode: crate::live_params::OutputMode| {
        r.control.live.write().binaural.output_mode = mode;
    };

    // Seed Interp state on the 12-wide speaker path.
    let out = r.render_frame(&pcm, 1, &event, Vec::new(), false).unwrap();
    assert_eq!(out.n_channels, 12);
    assert_eq!(out.samples.len(), 40 * 12);

    // A mode request first fades the active 12-wide frame to silence. Only
    // after that completed frame does the renderer adopt the 13-wide cascade
    // and emit stereo. Exercise enough blocks to cross the 5 ms boundary.
    set_mode(&mut r, crate::live_params::OutputMode::Binaural);
    let mut reached_binaural = false;
    for _ in 0..16 {
        let out = r.render_frame(&pcm, 1, &[], Vec::new(), false).unwrap();
        assert!(out.n_channels == 12 || out.n_channels == 2);
        assert_eq!(out.samples.len(), 40 * out.n_channels);
        if out.n_channels == 2 {
            reached_binaural = true;
            break;
        }
    }
    assert!(reached_binaural, "cascade mode never became the active frame");

    set_mode(&mut r, crate::live_params::OutputMode::SpeakerArray);
    let mut speaker_out = None;
    for _ in 0..16 {
        let out = r.render_frame(&pcm, 1, &[], Vec::new(), false).unwrap();
        assert!(out.n_channels == 2 || out.n_channels == 12);
        assert_eq!(out.samples.len(), 40 * out.n_channels);
        if out.n_channels == 12 {
            speaker_out = Some(out);
            break;
        }
    }
    let out = speaker_out.expect("speaker mode never became the active frame");
    assert!(
        out.samples.iter().any(|sample| sample.abs() > 1e-6),
        "speaker output must survive the round-trip"
    );
}

// TODO: Add integration test"""
s2, n = pat.subn(repl, s, count=1)
if n != 1:
    raise SystemExit(f"tests.rs: Interp regression replacement count {n}")
p.write_text(s2)
