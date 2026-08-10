use anyhow::{Context, Result};
use wasapi::{AudioClient, Direction, SampleType, StreamMode, WaveFormat, initialize_mta};

const PROBE_SAMPLE_RATE_HZ: usize = 48_000;
const PROBE_CHANNELS: usize = 2;
const PROBE_BUFFER_DURATION_HNS: i64 = 200_000; // 20 ms in 100 ns units.

/// Open the modern Windows process-loopback path in exclusion mode.
///
/// The target process is this host itself and `include_tree = false` maps to
/// `PROCESS_LOOPBACK_MODE_EXCLUDE_TARGET_PROCESS_TREE` in `wasapi-rs`.
/// Consequently the capture stream can hear the rest of the system mix while
/// excluding Omniphony and any child processes from its own capture. That avoids
/// an immediate capture -> render -> capture feedback loop.
///
/// Important: process loopback is a COPY of the system mix, not an intercept.
/// The original dry audio still reaches its render endpoint. Therefore this is
/// only a transport/diagnostic primitive, not the final transparent HeSuVi
/// replacement route. The product path still needs an in-place APO or a routed
/// virtual endpoint so the listener receives only the Omniphony-rendered signal.
///
/// This function is deliberately only an activation/format probe. It does not
/// start the stream or consume samples yet, so adding it cannot change the
/// renderer's audible path.
pub fn probe_self_excluding_system_loopback() -> Result<()> {
    initialize_mta().context("failed to initialize COM MTA for WASAPI loopback")?;

    let desired_format = WaveFormat::new(
        32,
        32,
        &SampleType::Float,
        PROBE_SAMPLE_RATE_HZ,
        PROBE_CHANNELS,
        None,
    );

    let mut audio_client = AudioClient::new_application_loopback_client(
        std::process::id(),
        false,
    )
    .context(
        "failed to activate self-excluding process loopback; this route requires modern Windows process-loopback support",
    )?;

    let stream_mode = StreamMode::PollingShared {
        autoconvert: true,
        buffer_duration_hns: PROBE_BUFFER_DURATION_HNS,
    };

    audio_client
        .initialize_client(&desired_format, &Direction::Capture, &stream_mode)
        .context("failed to initialize the self-excluding WASAPI loopback stream")?;

    let _capture_client = audio_client
        .get_audiocaptureclient()
        .context("loopback stream initialized but did not expose an audio capture client")?;

    println!(
        "loopback capture: ready (48 kHz stereo float, shared, self process tree excluded)"
    );
    Ok(())
}
