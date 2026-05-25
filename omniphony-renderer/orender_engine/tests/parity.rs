//! End-to-end functional harness: render a real TrueHD Atmos elementary stream
//! through the engine using the real decoder bridge, and check the output is
//! sane (correct channel count, frames produced, finite, non-silent).
//!
//! This exercises the exact path the FFI uses (`Engine::from_paths` →
//! `process_raw`). It is skipped unless both env vars are set, so the default
//! `cargo test` (which has no bridge/sample) stays green:
//!
//! ```sh
//! ORENDER_BRIDGE=../../harletty-bridge/target/release/libharletty_bridge.so \
//! ORENDER_SAMPLE=../../reference-sources/libstarmine_ad/crates/libstarmine_ad/tests/data/truehd_atmos_prefix_32k.mlp \
//! cargo test -p orender_engine --test parity -- --nocapture
//! ```
//!
//! Bit-exact comparison against the `orender` CLI is a separate step that first
//! needs a render-to-file output mode in the CLI.

use orender_engine::Engine;
use std::path::Path;

#[test]
fn renders_real_truehd_atmos_stream() {
    let (Ok(bridge), Ok(sample)) = (
        std::env::var("ORENDER_BRIDGE"),
        std::env::var("ORENDER_SAMPLE"),
    ) else {
        eprintln!("skipping renders_real_truehd_atmos_stream: set ORENDER_BRIDGE and ORENDER_SAMPLE");
        return;
    };

    let data = std::fs::read(&sample).expect("read sample file");
    let mut engine =
        Engine::from_paths(None, None, Path::new(&bridge), 48_000).expect("build engine");

    // Smoke the OSC live-control wiring: the listener binds (ephemeral port) and
    // attaches the renderer control without error. No client interaction here.
    engine
        .enable_osc(orender_engine::OscOptions {
            host: "127.0.0.1".to_string(),
            port_out: 9000,
            port_in: 0,
        })
        .expect("enable_osc should start the OSC listener");

    let channels = engine.channel_count();
    let spatial_before = engine.is_spatial();

    let mut total_frames = 0usize;
    let mut total_samples = 0usize;
    let mut peak = 0.0f32;

    for packet in data.chunks(4096) {
        let chunks = engine.process_raw(packet).expect("process_raw");
        for c in chunks {
            assert_eq!(c.n_channels, channels, "channel count must stay stable");
            assert_eq!(
                c.samples.len(),
                c.n_frames * c.n_channels as usize,
                "sample count must equal frames * channels"
            );
            for &s in &c.samples {
                assert!(s.is_finite(), "rendered sample must be finite");
                peak = peak.max(s.abs());
            }
            total_frames += c.n_frames;
            total_samples += c.samples.len();
        }
    }

    eprintln!(
        "parity harness: channels={channels} spatial_before={spatial_before} \
         is_spatial_after={} frames={total_frames} samples={total_samples} peak={peak:.6}",
        engine.is_spatial()
    );

    assert!(spatial_before, "Atmos stream must report is_spatial() = true");
    assert_eq!(channels, 12, "7.1.4 preset must yield 12 speakers");
    // The engine only emits chunks for frames carrying spatial objects (the
    // non-object path returns None), so frames > 0 proves the full Atmos object
    // path ran: bridge decode -> metadata -> events -> VBAP render.
    assert!(total_frames > 0, "expected at least one rendered object frame");

    // NOTE: this fixture (`truehd_atmos_prefix_32k.mlp`) is a metadata prefix
    // whose audio decodes to silence (verified: valid OAMD/bed metadata, zero
    // PCM), so `peak` is expected to be 0.0 here. We therefore don't assert on
    // it — verifying audible content needs a real (non-prefix) Atmos sample.
    let _ = peak;
}
