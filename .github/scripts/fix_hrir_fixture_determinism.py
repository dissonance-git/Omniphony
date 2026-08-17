from pathlib import Path

p = Path("omniphony-renderer/dsp_fixtures/src/scene.rs")
s = p.read_text()
old = '''    while r.binaural_rebuild_pending() {
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
'''
new = '''    while r.binaural_rebuild_pending() {
        assert!(
            std::time::Instant::now() < deadline,
            "binaural HRIR rebuild for {hrir_source:?} never landed"
        );
        std::thread::yield_now();
        settled += 1;
        buf = render_one(&mut r, buf, SETTLE_SEED_BASE + settled);
    }

    // The number of settle frames above is intentionally scheduler-dependent.
    // Their audio must therefore not become part of the measurement's initial
    // condition. Reset only stream-lifetime DSP history after the requested
    // grid is live; BinauralRenderer keeps that active HRIR grid and its worker
    // across reset. The first fixed prime frame consumes this reset request.
    r.reset_runtime_state();
    buf.clear();

    // Stage 2: now prime from a fixed excitation sequence. The measured state
    // is therefore a pure function of these fixed blocks, independent of how
    // many scheduler-dependent settle frames were needed above.
    for block in 0..PRIME_BLOCKS {
'''
if s.count(old) != 1:
    raise SystemExit(f"expected one HRIR settle block, found {s.count(old)}")
p.write_text(s.replace(old, new, 1))
