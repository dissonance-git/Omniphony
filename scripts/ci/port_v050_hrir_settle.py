from pathlib import Path


def replace_once(path: str, old: str, new: str, label: str) -> None:
    p = Path(path)
    text = p.read_text(encoding="utf-8")
    count = text.count(old)
    if count != 1:
        raise SystemExit(f"{label}: expected one match in {path}, found {count}")
    p.write_text(text.replace(old, new, 1), encoding="utf-8")


binaural = "omniphony-renderer/renderer/src/binaural/mod.rs"
replace_once(
    binaural,
    '''    /// HRIR source last *requested* (the active grid may briefly lag it while
    /// the worker builds — see [`Self::ensure_source`]).
    source: HrirSource,
''',
    '''    /// HRIR source last *requested* (the active grid may briefly lag it while
    /// the worker builds — see [`Self::ensure_source`]).
    source: HrirSource,
    /// HRIR source that actually produced `hrir`. Kept separate from the
    /// requested source so tests and attribution-sensitive callers can wait for
    /// the asynchronous grid swap rather than guessing from wall-clock time.
    active_source: HrirSource,
''',
    "active source field",
)
replace_once(
    binaural,
    '''            hrir: std::sync::Arc::new(Self::build_hrir(&source, sample_rate)),
            source,
            incoming,
''',
    '''            hrir: std::sync::Arc::new(Self::build_hrir(&source, sample_rate)),
            source: source.clone(),
            active_source: source,
            incoming,
''',
    "active source init",
)
replace_once(
    binaural,
    '''    /// Identity of the active HRIR grid (tests observe the async swap with it).
    #[cfg(test)]
    fn hrir_grid_id(&self) -> usize {
        std::sync::Arc::as_ptr(&self.hrir) as usize
    }

    fn build_hrir(source: &HrirSource, sample_rate: u32) -> HrirSet {
''',
    '''    /// Identity of the active HRIR grid (tests observe the async swap with it).
    #[cfg(test)]
    fn hrir_grid_id(&self) -> usize {
        std::sync::Arc::as_ptr(&self.hrir) as usize
    }

    /// Whether the HRIR set currently being convolved is older than the source
    /// last requested by live configuration. This is an observable state, not a
    /// timing estimate, so validation can settle deterministically under load.
    pub fn rebuild_pending(&self) -> bool {
        self.source != self.active_source
    }

    fn build_hrir(source: &HrirSource, sample_rate: u32) -> HrirSet {
''',
    "rebuild pending method",
)
replace_once(
    binaural,
    '''            if built.source == self.source {
                self.hrir = std::sync::Arc::clone(&built.set);
                // Same direction on a different HRTF set is a different kernel.
                self.hrir_generation = self.hrir_generation.wrapping_add(1);
''',
    '''            if built.source == self.source {
                self.hrir = std::sync::Arc::clone(&built.set);
                self.active_source = built.source.clone();
                // Same direction on a different HRTF set is a different kernel.
                self.hrir_generation = self.hrir_generation.wrapping_add(1);
''',
    "accepted build identity",
)

spatial = "omniphony-renderer/renderer/src/spatial_renderer/mod.rs"
replace_once(
    spatial,
    '''    pub fn virtual_bus(&self) -> Option<(&[f32], usize)> {
''',
    '''    /// Whether a requested binaural HRIR source has not become the active
    /// convolution grid yet. Exposed for deterministic validation and host
    /// diagnostics; normal realtime rendering continues on the old grid meanwhile.
    pub fn binaural_rebuild_pending(&self) -> bool {
        self.binaural.rebuild_pending()
    }

    pub fn virtual_bus(&self) -> Option<(&[f32], usize)> {
''',
    "spatial rebuild pending bridge",
)

scene = "omniphony-renderer/dsp_fixtures/src/scene.rs"
p = Path(scene)
text = p.read_text(encoding="utf-8")
start = text.index("pub fn render_single_object_binaural(")
end_marker = "\n#[cfg(test)]\nmod tests {"
end = text.index(end_marker, start)
replacement = r'''pub fn render_single_object_binaural(
    azimuth_deg: f32,
    blocks: usize,
    hrir_source: HrirSource,
) -> (Vec<f32>, Vec<f32>) {
    const PRIME_BLOCKS: usize = 64;
    const SETTLE_SEED_BASE: usize = 1 << 20;

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
        let f = r
            .render_frame(&make_pcm_block(1, seed), 1, &event, buf, false)
            .expect("binaural ITD render");
        let mut samples = f.samples;
        samples.clear();
        samples
    };

    // Stage 1: render-then-check because render_frame is what notices the live
    // source request and later installs a completed worker result. Settle blocks
    // use a disjoint seed range so scheduler-dependent count cannot influence
    // the deterministic measurement window.
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

    // Stage 2: only after the requested grid is genuinely active, discard a
    // fixed number of blocks so gain, position, delay and convolver histories
    // are a pure function of this fixed excitation rather than worker timing.
    for block in 0..PRIME_BLOCKS {
        buf = render_one(&mut r, buf, block);
    }

    let mut left = Vec::with_capacity(blocks * BLOCK_SAMPLES);
    let mut right = Vec::with_capacity(blocks * BLOCK_SAMPLES);
    for block in 0..blocks {
        let f = r
            .render_frame(
                &make_pcm_block(1, PRIME_BLOCKS + block),
                1,
                &event,
                buf,
                false,
            )
            .expect("binaural ITD render");
        for frame in f.samples.chunks_exact(2) {
            left.push(frame[0]);
            right.push(frame[1]);
        }
        buf = f.samples;
        buf.clear();
    }
    (left, right)
}
'''
p.write_text(text[:start] + replacement + text[end:], encoding="utf-8")
