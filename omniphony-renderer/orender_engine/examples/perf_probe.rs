//! Controlled, mpv-free perf probe: stream a raw TrueHD/MLP elementary stream
//! through the real engine (decode → metadata → VBAP render), with the per-frame
//! perf instrumentation on. Lets us measure render/decode cost and its variance
//! on real Atmos content, and contrast ramp modes, without launching mpv.
//!
//! Unlike the `parity` test it does NOT slurp the whole file into RAM — it reads
//! in fixed chunks and stops after `--max-mb`, so it works on multi-GB movie
//! tracks.
//!
//! Usage:
//!   cargo run -p orender_engine --example perf_probe --release -- \
//!       <stream.thd> [--ramp frame|sample] [--max-mb N] [--bridge PATH]
//!
//! The bridge is taken from `--bridge` or the `ORENDER_BRIDGE` env var. Stats are
//! printed ~1 Hz (and once at the end) by the engine's ORENDER_PERF_LOG path,
//! which this example enables automatically.
//!
//! Example:
//!   cargo run -p orender_engine --example perf_probe --release -- \
//!     /path/Blade.Runner.…TrueHD.Atmos.thd --ramp sample --max-mb 200 \
//!     --bridge ../../harletty-bridge/target/release/libharletty_bridge.so

use std::io::Read;
use std::path::PathBuf;

use orender_engine::Engine;

fn main() {
    let mut args = std::env::args().skip(1);
    let mut stream: Option<PathBuf> = None;
    let mut ramp = "frame".to_string();
    let mut max_mb: u64 = 200;
    let mut bridge = std::env::var("ORENDER_BRIDGE").ok().map(PathBuf::from);

    while let Some(a) = args.next() {
        match a.as_str() {
            "--ramp" => ramp = args.next().expect("--ramp needs a value"),
            "--max-mb" => {
                max_mb = args
                    .next()
                    .and_then(|v| v.parse().ok())
                    .expect("--max-mb needs a number")
            }
            "--bridge" => bridge = Some(PathBuf::from(args.next().expect("--bridge needs a path"))),
            other => stream = Some(PathBuf::from(other)),
        }
    }

    let stream = stream.expect(
        "usage: perf_probe <stream.thd> [--ramp frame|sample] [--max-mb N] [--bridge PATH]",
    );
    let bridge = bridge.expect("set --bridge or ORENDER_BRIDGE");

    // Turn the engine's per-frame perf accumulator on for this process.
    unsafe { std::env::set_var("ORENDER_PERF_LOG", "1") };

    // Drive ramp_mode through the real config path (also exercises the engine's
    // config→live wiring). A temp YAML with just the bits from_paths reads.
    let cfg_path = std::env::temp_dir().join(format!("perf_probe_{}.yaml", std::process::id()));
    std::fs::write(
        &cfg_path,
        format!(
            "render:\n  bridge_path: {}\n  ramp_mode: {}\n",
            bridge.display(),
            ramp
        ),
    )
    .expect("write temp config");

    let mut engine = Engine::from_paths(Some(&cfg_path), None, None, None, 48_000)
        .expect("build engine from temp config");
    eprintln!(
        "perf_probe: stream={} ramp={ramp} max_mb={max_mb} channels={} spatial={}",
        stream.display(),
        engine.channel_count(),
        engine.is_spatial(),
    );

    let mut file = std::fs::File::open(&stream).expect("open stream");
    let mut chunk = vec![0u8; 64 * 1024];
    let max_bytes = max_mb.saturating_mul(1024 * 1024);
    let mut read_total: u64 = 0;

    while read_total < max_bytes {
        let n = file.read(&mut chunk).expect("read stream");
        if n == 0 {
            break; // EOF
        }
        read_total += n as u64;
        engine.process_raw(&chunk[..n]).expect("process_raw");
    }

    // Engine's Drop flushes the final perf line.
    drop(engine);
    let _ = std::fs::remove_file(&cfg_path);
    eprintln!(
        "perf_probe: done ({} MiB processed)",
        read_total / (1024 * 1024)
    );
}
