#[cfg(target_os = "windows")]
use anyhow::{Context, bail};
#[cfg(target_os = "windows")]
use bridge_api::RInputTransport;
#[cfg(target_os = "windows")]
use cpal::Sample;
#[cfg(target_os = "windows")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(target_os = "windows")]
use orender_engine::Engine;
#[cfg(target_os = "windows")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, TryRecvError, TrySendError, sync_channel},
};
#[cfg(target_os = "windows")]
use std::time::Duration;
#[cfg(target_os = "windows")]
use wasapi::{
    AudioCaptureClient, AudioClient, Direction, SampleType, StreamMode, WaveFormat, initialize_mta,
    make_channelmasks,
};

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
            "--output" => {
                parsed.output = Some(args.next().context("--output requires a device-name substring")?);
            }
            // Kept only so an older hand-written command fails gracefully after the
            // baseline stopped using recording/capture endpoints.
            "--input" => {
                let _ = args.next().context("--input requires a device-name substring")?;
                eprintln!(
                    "note: --input is no longer needed; Omniphony now uses self-excluding Windows process loopback"
                );
            }
            "--list-devices" => parsed.list_devices = true,
            "--start-off" => parsed.start_off = true,
            "-h" | "--help" => {
                println!(
                    "Omniphony live Windows baseline\n\n\
                     Usage:\n  \
                       omniphony_live.exe [--output <name>] [--start-off]\n  \
                       omniphony_live.exe --list-devices\n\n\
                     No recording/input device is required. Omniphony captures the Windows\n\
                     mix through self-excluding WASAPI process loopback. With no --output,\n\
                     the host prefers a FiiO output and otherwise uses a non-cable Windows\n\
                     default output when one is available.\n"
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
    const PLAYBACK_QUEUE_BLOCKS: usize = 16;

    let args = parse_args()?;

    // Process loopback requires MTA. Do this before CPAL/WASAPI device discovery
    // touches COM on the thread, otherwise Windows can reject the later apartment
    // change with RPC_E_CHANGED_MODE (0x80010106).
    initialize_mta()
        .ok()
        .context("failed to initialize COM MTA before Windows audio setup")?;

    let host = cpal::default_host();
    if args.list_devices {
        print_devices(&host)?;
        return Ok(());
    }

    let output_device = choose_output_device(&host, args.output.as_deref())?;
    let output_name = output_device
        .name()
        .unwrap_or_else(|_| "<unavailable output name>".to_string());
    let (output_format, output_config) = choose_output_config(&output_device, SAMPLE_RATE_HZ)?;

    let mut loopback = LoopbackCapture::open(SAMPLE_RATE_HZ)?;
    let input_channels = loopback.channels();

    println!("Omniphony for Headphones - live Windows baseline");
    println!("  capture: self-excluding Windows process loopback");
    println!("           {SAMPLE_RATE_HZ} Hz / {input_channels}ch / f32");
    println!("  output:  {output_name}");
    println!(
        "           {} Hz / {}ch / {output_format:?}",
        output_config.sample_rate.0, output_config.channels
    );
    println!("  renderer: protected Omniphony binaural path");
    println!("  input device selection: none required");

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

    // reference_bridge accepts a streaming WAV header whose data length is
    // 0xffffffff. Seed it once, then feed canonical interleaved f32 PCM forever.
    let header = streaming_f32_wav_header(input_channels as u16, SAMPLE_RATE_HZ);
    let header_output = engine
        .process(&header, RInputTransport::Raw, 0)
        .context("failed to seed live PCM bridge")?;
    if !header_output.is_empty() {
        bail!("streaming WAV header unexpectedly produced audio");
    }

    let enabled = Arc::new(AtomicBool::new(!args.start_off));
    let quit = Arc::new(AtomicBool::new(false));
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
    loopback.start()?;

    println!();
    println!(
        "LIVE. Omniphony is {}.",
        if enabled.load(Ordering::Relaxed) { "ON" } else { "OFF" }
    );
    println!("Press ENTER to toggle ON/OFF. Type q then ENTER to quit.");
    println!("Play normally in foobar or any Windows application.");

    let mut pcm_bytes = Vec::<u8>::new();
    while !quit.load(Ordering::Relaxed) {
        let Some(input) = loopback.next_block()? else {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        };
        if input.is_empty() || input.len() % input_channels != 0 {
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
                // This is a disposable listening baseline. Prefer a short dropout
                // to allowing latency to grow without bound.
            }
            Err(TrySendError::Disconnected(_)) => bail!("WASAPI playback stream disconnected"),
        }
    }

    let _ = loopback.stop();
    drop(playback_stream);
    println!("Omniphony live baseline stopped");
    Ok(())
}

#[cfg(target_os = "windows")]
struct LoopbackCapture {
    client: AudioClient,
    capture: AudioCaptureClient,
    channels: usize,
    scratch: Vec<u8>,
}

#[cfg(target_os = "windows")]
impl LoopbackCapture {
    fn open(sample_rate_hz: u32) -> anyhow::Result<Self> {
        const BUFFER_DURATION_HNS: i64 = 200_000; // 20 ms in 100 ns units.

        let mode = StreamMode::PollingShared {
            autoconvert: true,
            buffer_duration_hns: BUFFER_DURATION_HNS,
        };
        let mut failures = Vec::<String>::new();

        // Preserve the richest practical bed first. The current private Windows
        // route is an 8-channel Hi-Fi Cable with a 5.1/side foobar bed inside it.
        // Stereo remains a fallback for ordinary Windows sources.
        for channels in [8usize, 6, 2] {
            let mask = make_channelmasks(channels).into_iter().next().unwrap_or(0);
            let format = WaveFormat::new(
                32,
                32,
                &SampleType::Float,
                sample_rate_hz as usize,
                channels,
                Some(mask),
            );
            let mut client = AudioClient::new_application_loopback_client(
                std::process::id(),
                false,
            )
            .context("failed to activate self-excluding Windows process loopback")?;

            match client.initialize_client(&format, &Direction::Capture, &mode) {
                Ok(()) => {
                    let capture = client
                        .get_audiocaptureclient()
                        .context("process loopback initialized but exposed no capture client")?;
                    return Ok(Self {
                        client,
                        capture,
                        channels,
                        scratch: Vec::new(),
                    });
                }
                Err(err) => failures.push(format!("{channels}ch: {err}")),
            }
        }

        bail!(
            "Windows process loopback rejected 8ch, 6ch and stereo 48 kHz float formats: {}",
            failures.join("; ")
        )
    }

    fn channels(&self) -> usize {
        self.channels
    }

    fn start(&self) -> anyhow::Result<()> {
        self.client
            .start_stream()
            .context("failed to start self-excluding Windows process loopback")
    }

    fn stop(&self) -> anyhow::Result<()> {
        self.client
            .stop_stream()
            .context("failed to stop Windows process loopback")
    }

    fn next_block(&mut self) -> anyhow::Result<Option<Vec<f32>>> {
        let frames = self
            .capture
            .get_next_packet_size()
            .context("failed to query Windows process-loopback packet size")?
            .unwrap_or(0) as usize;
        if frames == 0 {
            return Ok(None);
        }

        let bytes_per_frame = self.channels * 4;
        let needed = frames.saturating_mul(bytes_per_frame);
        self.scratch.resize(needed, 0);
        let (read_frames, info) = self
            .capture
            .read_from_device(&mut self.scratch)
            .context("failed to read Windows process-loopback packet")?;
        let read_frames = read_frames as usize;
        if read_frames == 0 {
            return Ok(None);
        }

        let sample_count = read_frames.saturating_mul(self.channels);
        if info.flags.silent {
            return Ok(Some(vec![0.0; sample_count]));
        }

        let byte_count = sample_count.saturating_mul(4);
        let mut samples = Vec::with_capacity(sample_count);
        for bytes in self.scratch[..byte_count].chunks_exact(4) {
            samples.push(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
        }

        if self.channels == 8 {
            Ok(Some(windows_71_to_bridge_order(&samples)))
        } else {
            Ok(Some(samples))
        }
    }
}

#[cfg(target_os = "windows")]
fn windows_71_to_bridge_order(input: &[f32]) -> Vec<f32> {
    // Standard Windows 7.1-surround interleaving follows channel-mask bit order:
    // L R C LFE Lb Rb Ls Rs.
    // reference_bridge expects:
    // L R C LFE Ls Rs Lb Rb.
    let mut out = Vec::with_capacity(input.len());
    for frame in input.chunks_exact(8) {
        out.extend_from_slice(&[
            frame[0], frame[1], frame[2], frame[3], frame[6], frame[7], frame[4], frame[5],
        ]);
    }
    out
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
    println!("Capture: self-excluding Windows process loopback (no input endpoint required)");
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
fn looks_like_virtual_cable(device: &cpal::Device) -> bool {
    device
        .name()
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            lower.contains("vb-audio") || lower.contains("hi-fi cable") || lower.contains("hifi cable")
        })
        .unwrap_or(false)
}

#[cfg(target_os = "windows")]
fn choose_output_device(host: &cpal::Host, requested: Option<&str>) -> anyhow::Result<cpal::Device> {
    if let Some(needle) = requested {
        return host
            .output_devices()?
            .find(|device| name_contains(device, needle))
            .with_context(|| format!("no WASAPI output device contains '{needle}'"));
    }

    // Private baseline: the physical endpoint is stable and should not be selected
    // every launch. Prefer the known FiiO automatically.
    if let Some(device) = host
        .output_devices()?
        .find(|device| name_contains(device, "fiio"))
    {
        return Ok(device);
    }

    // Generic fallback for development machines whose default is already physical.
    if let Some(device) = host.default_output_device() {
        if !looks_like_virtual_cable(&device) {
            return Ok(device);
        }
    }

    bail!(
        "no physical output was auto-detected; expected a FiiO device or a non-cable Windows default. Run --list-devices and use --output <substring> for diagnostics"
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
