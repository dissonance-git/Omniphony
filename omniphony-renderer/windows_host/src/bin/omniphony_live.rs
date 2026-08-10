#[cfg(target_os = "windows")]
use anyhow::{Context, bail};
#[cfg(target_os = "windows")]
use bridge_api::RInputTransport;
#[cfg(target_os = "windows")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(target_os = "windows")]
use orender_engine::Engine;
#[cfg(target_os = "windows")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{Receiver, SyncSender, TryRecvError, TrySendError, sync_channel},
};
#[cfg(target_os = "windows")]
use std::time::Duration;

fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        return run();
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("omniphony_live is only available on Windows");
    }
}

#[cfg(target_os = "windows")]
#[derive(Default)]
struct Args {
    input: Option<String>,
    output: Option<String>,
    list_devices: bool,
    start_off: bool,
}

#[cfg(target_os = "windows")]
fn parse_args() -> anyhow::Result<Args> {
    let mut parsed = Args::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--input" => {
                parsed.input = Some(args.next().context("--input requires a device-name substring")?);
            }
            "--output" => {
                parsed.output = Some(args.next().context("--output requires a device-name substring")?);
            }
            "--list-devices" => parsed.list_devices = true,
            "--start-off" => parsed.start_off = true,
            "-h" | "--help" => {
                println!(
                    "Omniphony live Windows baseline\n\n\
                     Usage:\n  \
                       omniphony_live.exe [--input <name>] [--output <name>] [--start-off]\n  \
                       omniphony_live.exe --list-devices\n\n\
                     The input should normally be the recording side of a clean virtual cable.\n\
                     With no --input, the host prefers a device containing 'Hi-Fi Cable' and\n\
                     falls back to the Windows default input. With no --output, it uses the\n\
                     Windows default output.\n"
                );
                std::process::exit(0);
            }
            other => bail!("unknown argument: {other}"),
        }
    }
    Ok(parsed)
}

#[cfg(target_os = "windows")]
fn run() -> anyhow::Result<()> {
    const SAMPLE_RATE_HZ: u32 = 48_000;
    const CAPTURE_QUEUE_BLOCKS: usize = 8;
    const PLAYBACK_QUEUE_BLOCKS: usize = 16;

    let args = parse_args()?;
    let host = cpal::default_host();
    if args.list_devices {
        print_devices(&host)?;
        return Ok(());
    }

    let input_device = choose_input_device(&host, args.input.as_deref())?;
    let output_device = choose_output_device(&host, args.output.as_deref())?;
    let input_name = input_device
        .name()
        .unwrap_or_else(|_| "<unavailable input name>".to_string());
    let output_name = output_device
        .name()
        .unwrap_or_else(|_| "<unavailable output name>".to_string());

    let (input_format, input_config) = choose_input_config(&input_device, SAMPLE_RATE_HZ)?;
    let input_channels = usize::from(input_config.channels);
    let (output_format, output_config) = choose_output_config(&output_device, SAMPLE_RATE_HZ)?;

    println!("Omniphony for Headphones - crude live Windows baseline");
    println!("  input:  {input_name}");
    println!(
        "          {} Hz / {}ch / {input_format:?}",
        input_config.sample_rate.0, input_config.channels
    );
    println!("  output: {output_name}");
    println!(
        "          {} Hz / {}ch / {output_format:?}",
        output_config.sample_rate.0, output_config.channels
    );
    println!("  renderer: protected Omniphony binaural path");
    if input_channels != 2 {
        println!(
            "  note: input is {input_channels}ch; OFF uses a conservative bed downmix.\n         For the cleanest ON/OFF baseline, feed the cable plain stereo."
        );
    }

    let bundle = Bundle::beside_executable()?;
    let mut engine = Engine::from_paths(
        Some(&bundle.config),
        Some(&bundle.layout),
        Some(&bundle.bridge),
        None,
        SAMPLE_RATE_HZ,
    )
    .context("failed to construct live Omniphony engine")?;
    engine.set_channel_render_mode_code(1);
    if engine.channel_count() != 2 {
        bail!(
            "protected binaural configuration expected stereo output but engine reports {} channels",
            engine.channel_count()
        );
    }

    // reference_bridge already supports streaming WAV where data-size 0/0xffffffff
    // means "until EOF". Give it one header, then feed captured f32 PCM forever.
    let header = streaming_f32_wav_header(input_config.channels, SAMPLE_RATE_HZ);
    let header_output = engine
        .process(&header, RInputTransport::Raw, 0)
        .context("failed to seed live PCM bridge")?;
    if !header_output.is_empty() {
        bail!("streaming WAV header unexpectedly produced audio");
    }

    let enabled = Arc::new(AtomicBool::new(!args.start_off));
    let quit = Arc::new(AtomicBool::new(false));
    let capture_drops = Arc::new(AtomicU64::new(0));

    let (capture_tx, capture_rx) = sync_channel::<Vec<f32>>(CAPTURE_QUEUE_BLOCKS);
    let capture_stream = build_capture_stream(
        &input_device,
        &input_config,
        input_format,
        capture_tx,
        Arc::clone(&capture_drops),
    )?;

    let (play_tx, play_rx) = sync_channel::<Vec<f32>>(PLAYBACK_QUEUE_BLOCKS);
    let playback_stream = build_playback_stream(
        &output_device,
        &output_config,
        output_format,
        play_rx,
    )?;

    spawn_console_control(Arc::clone(&enabled), Arc::clone(&quit));

    playback_stream
        .play()
        .context("failed to start WASAPI playback stream")?;
    capture_stream
        .play()
        .context("failed to start WASAPI capture stream")?;

    println!();
    println!(
        "LIVE. Omniphony is {}.",
        if enabled.load(Ordering::Relaxed) { "ON" } else { "OFF" }
    );
    println!("Press ENTER to toggle ON/OFF. Type q then ENTER to quit.");
    println!("Play anything into the selected cable input.");

    let mut pcm_bytes = Vec::<u8>::new();
    while !quit.load(Ordering::Relaxed) {
        let input = match capture_rx.recv_timeout(Duration::from_millis(100)) {
            Ok(block) => block,
            Err(std::sync::mpsc::RecvTimeoutError::Timeout) => continue,
            Err(std::sync::mpsc::RecvTimeoutError::Disconnected) => {
                bail!("WASAPI capture stream disconnected")
            }
        };
        if input.is_empty() {
            continue;
        }
        if input.len() % input_channels != 0 {
            continue;
        }

        f32_as_le_bytes(&input, &mut pcm_bytes);
        let rendered = engine
            .process(&pcm_bytes, RInputTransport::Raw, 0)
            .context("live Omniphony render failed")?;

        let mut wet = Vec::<f32>::new();
        for block in rendered {
            if block.n_channels != 2 {
                bail!("live renderer changed output width to {} channels", block.n_channels);
            }
            wet.extend_from_slice(&block.samples);
        }

        let selected = if enabled.load(Ordering::Relaxed) {
            wet
        } else {
            dry_downmix(&input, input_channels)
        };
        if selected.is_empty() {
            continue;
        }

        match play_tx.try_send(selected) {
            Ok(()) => {}
            Err(TrySendError::Full(_)) => {
                // Prefer a short audible dropout to unbounded latency growth in this
                // disposable baseline host. The capture side is similarly bounded.
            }
            Err(TrySendError::Disconnected(_)) => bail!("WASAPI playback stream disconnected"),
        }
    }

    drop(capture_stream);
    drop(playback_stream);
    let dropped = capture_drops.load(Ordering::Relaxed);
    if dropped > 0 {
        println!("capture queue dropped {dropped} block(s) while the renderer was behind");
    }
    println!("Omniphony live baseline stopped");
    Ok(())
}

#[cfg(target_os = "windows")]
struct Bundle {
    bridge: PathBuf,
    config: PathBuf,
    layout: PathBuf,
}

#[cfg(target_os = "windows")]
impl Bundle {
    fn beside_executable() -> anyhow::Result<Self> {
        let exe = std::env::current_exe().context("failed to resolve executable path")?;
        let root = exe.parent().context("executable has no parent directory")?;
        let reference = root.join("reference-demo");
        let bundle = Self {
            bridge: root.join("reference_bridge.dll"),
            config: reference.join("upstream-demo-reference.yaml"),
            layout: reference.join("7.1.4.yaml"),
        };
        require_file(&bundle.bridge, "reference PCM bridge")?;
        require_file(&bundle.config, "protected Omniphony binaural config")?;
        require_file(&bundle.layout, "7.1.4 render layout")?;
        Ok(bundle)
    }
}

#[cfg(target_os = "windows")]
fn require_file(path: &Path, label: &str) -> anyhow::Result<()> {
    if !path.is_file() {
        bail!("missing {label}: {}", path.display());
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn print_devices(host: &cpal::Host) -> anyhow::Result<()> {
    println!("WASAPI input devices:");
    for device in host.input_devices()? {
        println!(
            "  {}",
            device
                .name()
                .unwrap_or_else(|_| "<unavailable device name>".to_string())
        );
    }
    println!("WASAPI output devices:");
    for device in host.output_devices()? {
        println!(
            "  {}",
            device
                .name()
                .unwrap_or_else(|_| "<unavailable device name>".to_string())
        );
    }
    Ok(())
}

#[cfg(target_os = "windows")]
fn name_contains(device: &cpal::Device, needle: &str) -> bool {
    device
        .name()
        .map(|name| name.to_ascii_lowercase().contains(&needle.to_ascii_lowercase()))
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn choose_input_device(host: &cpal::Host, requested: Option<&str>) -> anyhow::Result<cpal::Device> {
    if let Some(needle) = requested {
        return host
            .input_devices()?
            .find(|device| name_contains(device, needle))
            .with_context(|| format!("no WASAPI input device contains '{needle}'"));
    }

    for hint in ["hi-fi cable", "hifi cable", "cable output"] {
        if let Some(device) = host.input_devices()?.find(|device| name_contains(device, hint)) {
            return Ok(device);
        }
    }

    host.default_input_device().context(
        "Windows has no default input device; pass --list-devices then --input <substring>",
    )
}

#[cfg(target_os = "windows")]
fn choose_output_device(host: &cpal::Host, requested: Option<&str>) -> anyhow::Result<cpal::Device> {
    if let Some(needle) = requested {
        return host
            .output_devices()?
            .find(|device| name_contains(device, needle))
            .with_context(|| format!("no WASAPI output device contains '{needle}'"));
    }
    host.default_output_device().context(
        "Windows has no default output device; pass --list-devices then --output <substring>",
    )
}

#[cfg(target_os = "windows")]
fn sample_format_rank(format: cpal::SampleFormat) -> u8 {
    match format {
        cpal::SampleFormat::F32 => 0,
        cpal::SampleFormat::I32 => 1,
        cpal::SampleFormat::I16 => 2,
        cpal::SampleFormat::F64 => 3,
        cpal::SampleFormat::U32 => 4,
        cpal::SampleFormat::U16 => 5,
        cpal::SampleFormat::I8 | cpal::SampleFormat::U8 => 6,
        cpal::SampleFormat::I64 | cpal::SampleFormat::U64 => 7,
        _ => 8,
    }
}

#[cfg(target_os = "windows")]
fn supported_input_width(channels: u16) -> bool {
    matches!(channels, 1 | 2 | 6 | 8 | 12)
}

#[cfg(target_os = "windows")]
fn choose_input_config(
    device: &cpal::Device,
    sample_rate_hz: u32,
) -> anyhow::Result<(cpal::SampleFormat, cpal::StreamConfig)> {
    let best = device
        .supported_input_configs()
        .context("failed to enumerate WASAPI input formats")?
        .filter(|range| {
            supported_input_width(range.channels())
                && range.min_sample_rate().0 <= sample_rate_hz
                && range.max_sample_rate().0 >= sample_rate_hz
        })
        // Preserve as much channel truth as the cable exposes; format quality is
        // the tiebreaker. Stereo still works when it is the only supported width.
        .min_by_key(|range| (u16::MAX - range.channels(), sample_format_rank(range.sample_format())))
        .with_context(|| {
            format!(
                "input device has no supported 1/2/6/8/12-channel {sample_rate_hz} Hz format"
            )
        })?;
    let sample_format = best.sample_format();
    let config = best.with_sample_rate(cpal::SampleRate(sample_rate_hz)).config();
    Ok((sample_format, config))
}

#[cfg(target_os = "windows")]
fn choose_output_config(
    device: &cpal::Device,
    sample_rate_hz: u32,
) -> anyhow::Result<(cpal::SampleFormat, cpal::StreamConfig)> {
    let best = device
        .supported_output_configs()
        .context("failed to enumerate WASAPI output formats")?
        .filter(|range| {
            range.channels() >= 2
                && range.min_sample_rate().0 <= sample_rate_hz
                && range.max_sample_rate().0 >= sample_rate_hz
        })
        .min_by_key(|range| (range.channels(), sample_format_rank(range.sample_format())))
        .with_context(|| format!("output device has no >=2ch {sample_rate_hz} Hz format"))?;
    let sample_format = best.sample_format();
    let config = best.with_sample_rate(cpal::SampleRate(sample_rate_hz)).config();
    Ok((sample_format, config))
}

#[cfg(target_os = "windows")]
fn build_capture_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    format: cpal::SampleFormat,
    tx: SyncSender<Vec<f32>>,
    drops: Arc<AtomicU64>,
) -> anyhow::Result<cpal::Stream> {
    match format {
        cpal::SampleFormat::I8 => build_typed_capture::<i8>(device, config, tx, drops),
        cpal::SampleFormat::I16 => build_typed_capture::<i16>(device, config, tx, drops),
        cpal::SampleFormat::I32 => build_typed_capture::<i32>(device, config, tx, drops),
        cpal::SampleFormat::I64 => build_typed_capture::<i64>(device, config, tx, drops),
        cpal::SampleFormat::U8 => build_typed_capture::<u8>(device, config, tx, drops),
        cpal::SampleFormat::U16 => build_typed_capture::<u16>(device, config, tx, drops),
        cpal::SampleFormat::U32 => build_typed_capture::<u32>(device, config, tx, drops),
        cpal::SampleFormat::U64 => build_typed_capture::<u64>(device, config, tx, drops),
        cpal::SampleFormat::F32 => build_typed_capture::<f32>(device, config, tx, drops),
        cpal::SampleFormat::F64 => build_typed_capture::<f64>(device, config, tx, drops),
        other => bail!("unsupported WASAPI input sample format: {other:?}"),
    }
}

#[cfg(target_os = "windows")]
fn build_typed_capture<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    tx: SyncSender<Vec<f32>>,
    drops: Arc<AtomicU64>,
) -> anyhow::Result<cpal::Stream>
where
    T: cpal::Sample + cpal::SizedSample + Copy + Send + 'static,
    f32: cpal::FromSample<T>,
{
    let err_fn = |err| eprintln!("WASAPI capture stream error: {err}");
    device
        .build_input_stream(
            config,
            move |data: &[T], _: &cpal::InputCallbackInfo| {
                let mut block = Vec::with_capacity(data.len());
                block.extend(data.iter().copied().map(f32::from_sample));
                match tx.try_send(block) {
                    Ok(()) => {}
                    Err(TrySendError::Full(_)) => {
                        drops.fetch_add(1, Ordering::Relaxed);
                    }
                    Err(TrySendError::Disconnected(_)) => {}
                }
            },
            err_fn,
            None,
        )
        .context("failed to create WASAPI capture stream")
}

#[cfg(target_os = "windows")]
fn build_playback_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    format: cpal::SampleFormat,
    rx: Receiver<Vec<f32>>,
) -> anyhow::Result<cpal::Stream> {
    match format {
        cpal::SampleFormat::I8 => build_typed_playback::<i8>(device, config, rx),
        cpal::SampleFormat::I16 => build_typed_playback::<i16>(device, config, rx),
        cpal::SampleFormat::I32 => build_typed_playback::<i32>(device, config, rx),
        cpal::SampleFormat::I64 => build_typed_playback::<i64>(device, config, rx),
        cpal::SampleFormat::U8 => build_typed_playback::<u8>(device, config, rx),
        cpal::SampleFormat::U16 => build_typed_playback::<u16>(device, config, rx),
        cpal::SampleFormat::U32 => build_typed_playback::<u32>(device, config, rx),
        cpal::SampleFormat::U64 => build_typed_playback::<u64>(device, config, rx),
        cpal::SampleFormat::F32 => build_typed_playback::<f32>(device, config, rx),
        cpal::SampleFormat::F64 => build_typed_playback::<f64>(device, config, rx),
        other => bail!("unsupported WASAPI output sample format: {other:?}"),
    }
}

#[cfg(target_os = "windows")]
fn build_typed_playback<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    rx: Receiver<Vec<f32>>,
) -> anyhow::Result<cpal::Stream>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = usize::from(config.channels);
    let mut current = Vec::<f32>::new();
    let mut cursor = 0usize;
    let err_fn = |err| eprintln!("WASAPI playback stream error: {err}");

    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                for frame in data.chunks_exact_mut(channels) {
                    let (left, right) = next_stereo_frame(&rx, &mut current, &mut cursor);
                    frame[0] = T::from_sample(left);
                    frame[1] = T::from_sample(right);
                    for sample in &mut frame[2..] {
                        *sample = T::from_sample(0.0);
                    }
                }
            },
            err_fn,
            None,
        )
        .context("failed to create WASAPI playback stream")
}

#[cfg(target_os = "windows")]
fn next_stereo_frame(
    rx: &Receiver<Vec<f32>>,
    current: &mut Vec<f32>,
    cursor: &mut usize,
) -> (f32, f32) {
    loop {
        if *cursor + 1 < current.len() {
            let pair = (current[*cursor], current[*cursor + 1]);
            *cursor += 2;
            return pair;
        }

        match rx.try_recv() {
            Ok(block) => {
                *current = block;
                *cursor = 0;
            }
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => return (0.0, 0.0),
        }
    }
}

#[cfg(target_os = "windows")]
fn streaming_f32_wav_header(channels: u16, sample_rate_hz: u32) -> Vec<u8> {
    let block_align = channels.saturating_mul(4);
    let byte_rate = sample_rate_hz.saturating_mul(u32::from(block_align));
    let mut wav = Vec::with_capacity(44);
    wav.extend_from_slice(b"RIFF");
    wav.extend_from_slice(&u32::MAX.to_le_bytes());
    wav.extend_from_slice(b"WAVE");
    wav.extend_from_slice(b"fmt ");
    wav.extend_from_slice(&16u32.to_le_bytes());
    wav.extend_from_slice(&3u16.to_le_bytes()); // IEEE float
    wav.extend_from_slice(&channels.to_le_bytes());
    wav.extend_from_slice(&sample_rate_hz.to_le_bytes());
    wav.extend_from_slice(&byte_rate.to_le_bytes());
    wav.extend_from_slice(&block_align.to_le_bytes());
    wav.extend_from_slice(&32u16.to_le_bytes());
    wav.extend_from_slice(b"data");
    wav.extend_from_slice(&u32::MAX.to_le_bytes());
    wav
}

#[cfg(target_os = "windows")]
fn f32_as_le_bytes(samples: &[f32], out: &mut Vec<u8>) {
    out.clear();
    out.reserve(samples.len() * 4);
    for &sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
}

#[cfg(target_os = "windows")]
fn dry_downmix(input: &[f32], channels: usize) -> Vec<f32> {
    if channels == 0 || input.len() % channels != 0 {
        return Vec::new();
    }
    if channels == 2 {
        return input.to_vec();
    }

    let frames = input.len() / channels;
    let mut out = Vec::with_capacity(frames * 2);
    for frame in input.chunks_exact(channels) {
        if channels == 1 {
            out.push(frame[0]);
            out.push(frame[0]);
            continue;
        }

        // Canonical reference_bridge order:
        // L R C LFE Ls Rs Lb Rb Tfl Tfr Tbl Tbr.
        let mut l = frame[0];
        let mut r = frame[1];
        if channels >= 3 {
            let c = frame[2] * std::f32::consts::FRAC_1_SQRT_2;
            l += c;
            r += c;
        }
        if channels >= 4 {
            let lfe = frame[3] * 0.5;
            l += lfe;
            r += lfe;
        }
        if channels >= 6 {
            l += frame[4] * std::f32::consts::FRAC_1_SQRT_2;
            r += frame[5] * std::f32::consts::FRAC_1_SQRT_2;
        }
        if channels >= 8 {
            l += frame[6] * 0.5;
            r += frame[7] * 0.5;
        }
        if channels >= 10 {
            l += frame[8] * 0.5;
            r += frame[9] * 0.5;
        }
        if channels >= 12 {
            l += frame[10] * 0.5;
            r += frame[11] * 0.5;
        }
        out.push(l.clamp(-1.0, 1.0));
        out.push(r.clamp(-1.0, 1.0));
    }
    out
}

#[cfg(target_os = "windows")]
fn spawn_console_control(enabled: Arc<AtomicBool>, quit: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        loop {
            let mut line = String::new();
            if stdin.read_line(&mut line).is_err() {
                quit.store(true, Ordering::Relaxed);
                break;
            }
            if line.trim().eq_ignore_ascii_case("q") {
                quit.store(true, Ordering::Relaxed);
                break;
            }
            let next = !enabled.load(Ordering::Relaxed);
            enabled.store(next, Ordering::Relaxed);
            println!("Omniphony {}", if next { "ON" } else { "OFF" });
        }
    });
}
