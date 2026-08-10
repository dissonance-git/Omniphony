use anyhow::{Context, bail};
use bridge_api::RInputTransport;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use orender_engine::Engine;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
    mpsc::{Receiver, TryRecvError, TrySendError, sync_channel},
};
use std::time::Duration;
use wasapi::{
    AudioCaptureClient, AudioClient, Direction, SampleType, StreamMode, WaveFormat,
    make_channelmasks,
};

const SAMPLE_RATE_HZ: u32 = 48_000;
const PLAYBACK_QUEUE_BLOCKS: usize = 16;
const FIELD_HIGHPASS_HZ: f32 = 220.0;
const FIELD_SUPPORT_GAIN: f32 = 0.14;

#[derive(Default)]
struct Args {
    output: Option<String>,
    start_off: bool,
}

fn parse_args() -> anyhow::Result<Args> {
    let mut parsed = Args::default();
    let mut args = std::env::args().skip(1);
    while let Some(arg) = args.next() {
        match arg.as_str() {
            "--output" => {
                parsed.output = Some(
                    args.next()
                        .context("--output requires a device-name substring")?,
                );
            }
            "--start-off" => parsed.start_off = true,
            "-h" | "--help" => {
                println!(
                    "Omniphony preserved-master stereo prototype\n\n\
                     Usage:\n  omniphony_worker.exe [--output <name>] [--start-off]\n\n\
                     Normal ON keeps the original stereo master as the direct signal.\n\
                     Only a bass-protected stereo side field is spatialized through\n\
                     upstream Omniphony and added back at low level. OFF is untouched\n\
                     captured stereo.\n"
                );
                std::process::exit(0);
            }
            other => bail!("unknown argument: {other}"),
        }
    }
    Ok(parsed)
}

pub fn run() -> anyhow::Result<()> {
    let args = parse_args()?;
    let host = cpal::default_host();

    let output_device = choose_output_device(&host, args.output.as_deref())?;
    let output_name = output_device
        .name()
        .unwrap_or_else(|_| "<unavailable output name>".to_string());
    let (output_format, output_config) = choose_output_config(&output_device, SAMPLE_RATE_HZ)?;

    // This worker is the stereo-music prototype. Refuse to silently call a
    // 5.1/7.1 fallback "stereo". Rich-source/session routing belongs to a
    // separate host path once source boundaries can be preserved correctly.
    let mut loopback = LoopbackCapture::open_stereo(SAMPLE_RATE_HZ)?;

    println!("Omniphony for Headphones - preserved-master stereo prototype");
    println!("  capture: self-excluding Windows process loopback");
    println!("           {SAMPLE_RATE_HZ} Hz / 2ch / f32");
    println!("  output:  {output_name}");
    println!(
        "           {} Hz / {}ch / {output_format:?}",
        output_config.sample_rate.0, output_config.channels
    );
    println!("  direct:  original captured stereo master");
    println!(
        "  support: side field above ~{FIELD_HIGHPASS_HZ:.0} Hz -> upstream Omniphony @ {:.0}%",
        FIELD_SUPPORT_GAIN * 100.0
    );
    println!("  room:    no reflections / no late reverb / no air absorption");

    let bundle = Bundle::beside_executable()?;
    let mut field_engine = Engine::from_paths(
        Some(&bundle.field_config),
        Some(&bundle.layout),
        Some(&bundle.bridge),
        None,
        SAMPLE_RATE_HZ,
    )
    .context("failed to construct Omniphony stereo support-field engine")?;
    field_engine.set_channel_render_mode_code(1);
    if field_engine.channel_count() != 2 {
        bail!(
            "stereo support-field configuration expected 2 output channels but engine reports {}",
            field_engine.channel_count()
        );
    }

    // Seed the streaming reference bridge once. Even though only a derived
    // support field enters the engine, it is ordinary canonical stereo f32 PCM.
    let header = streaming_f32_wav_header(2, SAMPLE_RATE_HZ);
    let header_output = field_engine
        .process(&header, RInputTransport::Raw, 0)
        .context("failed to seed stereo support-field PCM bridge")?;
    if !header_output.is_empty() {
        bail!("streaming WAV header unexpectedly produced audio");
    }

    let quit = Arc::new(AtomicBool::new(false));
    let (play_tx, play_rx) = sync_channel::<Vec<f32>>(PLAYBACK_QUEUE_BLOCKS);
    let playback_stream = build_playback_stream(
        &output_device,
        &output_config,
        output_format,
        play_rx,
    )?;
    spawn_quit_control(Arc::clone(&quit));

    playback_stream
        .play()
        .context("failed to start WASAPI playback stream")?;
    loopback.start()?;

    let mut extractor = StereoFieldExtractor::new(SAMPLE_RATE_HZ, FIELD_HIGHPASS_HZ);
    let mut pcm_bytes = Vec::<u8>::new();
    let mut dry_fifo = VecDeque::<f32>::new();

    println!();
    println!(
        "LIVE. Omniphony is {}.",
        if args.start_off { "OFF" } else { "ON" }
    );

    while !quit.load(Ordering::Relaxed) {
        let Some(input) = loopback.next_block()? else {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        };
        if input.is_empty() || input.len() % 2 != 0 {
            continue;
        }

        if args.start_off {
            // Clean bypass: the exact captured stereo samples go to output.
            try_queue_block(&play_tx, input)?;
            continue;
        }

        // Keep the exact stereo master until the matching spatial support exits
        // the engine. This aligns the direct path to the bridge/renderer latency
        // instead of mixing a delayed wet field against a current dry block.
        dry_fifo.extend(input.iter().copied());

        // Only a conservative side-derived field enters the spatial renderer.
        // The authored mid/direct signal never enters this branch.
        let field_input = extractor.process(&input);
        f32_as_le_bytes(&field_input, &mut pcm_bytes);
        let rendered = field_engine
            .process(&pcm_bytes, RInputTransport::Raw, 0)
            .context("live Omniphony support-field render failed")?;

        for block in rendered {
            if block.n_channels != 2 {
                bail!(
                    "stereo support-field renderer changed output width to {} channels",
                    block.n_channels
                );
            }
            if block.samples.is_empty() {
                continue;
            }
            if dry_fifo.len() < block.samples.len() {
                bail!(
                    "support field produced {} samples with only {} aligned dry samples buffered",
                    block.samples.len(),
                    dry_fifo.len()
                );
            }

            let mut dry = Vec::with_capacity(block.samples.len());
            for _ in 0..block.samples.len() {
                dry.push(dry_fifo.pop_front().expect("dry FIFO length checked above"));
            }
            let mixed = mix_preserved_master_with_support(
                &dry,
                &block.samples,
                FIELD_SUPPORT_GAIN,
            )?;
            try_queue_block(&play_tx, mixed)?;
        }
    }

    let _ = loopback.stop();
    drop(playback_stream);
    println!("Omniphony stereo prototype stopped");
    Ok(())
}

fn try_queue_block(
    tx: &std::sync::mpsc::SyncSender<Vec<f32>>,
    block: Vec<f32>,
) -> anyhow::Result<()> {
    if block.is_empty() {
        return Ok(());
    }
    match tx.try_send(block) {
        Ok(()) => Ok(()),
        Err(TrySendError::Full(_)) => {
            // Prototype policy: a short dropout is preferable to allowing
            // transport latency to grow without bound.
            Ok(())
        }
        Err(TrySendError::Disconnected(_)) => bail!("WASAPI playback stream disconnected"),
    }
}

/// Conservative field extraction for the first preserved-master experiment.
///
/// `(L-R)/2` contains the stereo side component. Only this evidence is promoted
/// into the Omniphony support field. A first-order high-pass prevents the support
/// branch from duplicating or phase-smearing the low-frequency foundation.
struct StereoFieldExtractor {
    alpha: f32,
    prev_input: f32,
    prev_output: f32,
}

impl StereoFieldExtractor {
    fn new(sample_rate_hz: u32, cutoff_hz: f32) -> Self {
        let dt = 1.0 / sample_rate_hz as f32;
        let rc = 1.0 / (2.0 * std::f32::consts::PI * cutoff_hz.max(1.0));
        Self {
            alpha: rc / (rc + dt),
            prev_input: 0.0,
            prev_output: 0.0,
        }
    }

    fn process(&mut self, input: &[f32]) -> Vec<f32> {
        let mut out = Vec::with_capacity(input.len());
        for frame in input.chunks_exact(2) {
            let side = 0.5 * (frame[0] - frame[1]);
            let high = self.alpha * (self.prev_output + side - self.prev_input);
            self.prev_input = side;
            self.prev_output = high;

            // Recreate the side component as a stereo pair. The dedicated
            // field config places the two virtual channels rearward/sideward.
            out.push(high);
            out.push(-high);
        }
        out
    }
}

/// Preserve the dry/master sample and let the spatial support use only remaining
/// headroom. The spatial layer gets reduced before the master ever gets clipped,
/// scaled or limited.
fn mix_preserved_master_with_support(
    dry: &[f32],
    support: &[f32],
    support_gain: f32,
) -> anyhow::Result<Vec<f32>> {
    if dry.len() != support.len() {
        bail!(
            "support-field length mismatch: dry={} samples, field={} samples",
            dry.len(),
            support.len()
        );
    }

    let gain = support_gain.clamp(0.0, 1.0);
    let mut out = Vec::with_capacity(dry.len());
    for (&base, &field) in dry.iter().zip(support.iter()) {
        if !base.is_finite() {
            out.push(0.0);
            continue;
        }
        if !field.is_finite() || base.abs() >= 1.0 {
            out.push(base.clamp(-1.0, 1.0));
            continue;
        }

        let wanted = field * gain;
        let minimum = -1.0 - base;
        let maximum = 1.0 - base;
        out.push(base + wanted.clamp(minimum, maximum));
    }
    Ok(out)
}

struct LoopbackCapture {
    client: AudioClient,
    capture: AudioCaptureClient,
    scratch: Vec<u8>,
}

impl LoopbackCapture {
    fn open_stereo(sample_rate_hz: u32) -> anyhow::Result<Self> {
        const BUFFER_DURATION_HNS: i64 = 200_000; // 20 ms in 100 ns units.
        let mode = StreamMode::PollingShared {
            autoconvert: true,
            buffer_duration_hns: BUFFER_DURATION_HNS,
        };
        let mask = make_channelmasks(2).into_iter().next().unwrap_or(0);
        let format = WaveFormat::new(
            32,
            32,
            &SampleType::Float,
            sample_rate_hz as usize,
            2,
            Some(mask),
        );
        let mut client = AudioClient::new_application_loopback_client(std::process::id(), false)
            .context("failed to activate self-excluding Windows process loopback")?;
        client
            .initialize_client(&format, &Direction::Capture, &mode)
            .context("Windows process loopback rejected the required stereo 48 kHz float format")?;
        let capture = client
            .get_audiocaptureclient()
            .context("process loopback initialized but exposed no capture client")?;
        Ok(Self {
            client,
            capture,
            scratch: Vec::new(),
        })
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

        let needed = frames.saturating_mul(2).saturating_mul(4);
        self.scratch.resize(needed, 0);
        let (read_frames, info) = self
            .capture
            .read_from_device(&mut self.scratch)
            .context("failed to read Windows process-loopback packet")?;
        let read_frames = read_frames as usize;
        if read_frames == 0 {
            return Ok(None);
        }

        let sample_count = read_frames.saturating_mul(2);
        if info.flags.silent {
            return Ok(Some(vec![0.0; sample_count]));
        }

        let byte_count = sample_count.saturating_mul(4);
        let mut samples = Vec::with_capacity(sample_count);
        for bytes in self.scratch[..byte_count].chunks_exact(4) {
            samples.push(f32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]]));
        }
        Ok(Some(samples))
    }
}

struct Bundle {
    bridge: PathBuf,
    field_config: PathBuf,
    layout: PathBuf,
}

impl Bundle {
    fn beside_executable() -> anyhow::Result<Self> {
        let exe = std::env::current_exe().context("failed to resolve executable path")?;
        let root = exe.parent().context("executable has no parent directory")?;
        let reference = root.join("reference-demo");
        let bundle = Self {
            bridge: root.join("reference_bridge.dll"),
            field_config: reference.join("stereo-field-prototype.yaml"),
            layout: reference.join("7.1.4.yaml"),
        };
        require_file(&bundle.bridge, "reference PCM bridge")?;
        require_file(&bundle.field_config, "stereo support-field Omniphony config")?;
        require_file(&bundle.layout, "7.1.4 render layout")?;
        Ok(bundle)
    }
}

fn require_file(path: &Path, label: &str) -> anyhow::Result<()> {
    if !path.is_file() {
        bail!("missing {label}: {}", path.display());
    }
    Ok(())
}

fn name_contains(device: &cpal::Device, needle: &str) -> bool {
    device
        .name()
        .map(|name| {
            name.to_ascii_lowercase()
                .contains(&needle.to_ascii_lowercase())
        })
        .unwrap_or(false)
}

fn looks_like_virtual_cable(device: &cpal::Device) -> bool {
    device
        .name()
        .map(|name| {
            let lower = name.to_ascii_lowercase();
            lower.contains("vb-audio")
                || lower.contains("hi-fi cable")
                || lower.contains("hifi cable")
        })
        .unwrap_or(false)
}

fn choose_output_device(
    host: &cpal::Host,
    requested: Option<&str>,
) -> anyhow::Result<cpal::Device> {
    if let Some(needle) = requested {
        return host
            .output_devices()?
            .find(|device| name_contains(device, needle))
            .with_context(|| format!("no WASAPI output device contains '{needle}'"));
    }

    if let Some(device) = host
        .output_devices()?
        .find(|device| name_contains(device, "fiio"))
    {
        return Ok(device);
    }

    if let Some(device) = host.default_output_device() {
        if !looks_like_virtual_cable(&device) {
            return Ok(device);
        }
    }

    bail!(
        "no physical output was auto-detected; expected a FiiO device or a non-cable Windows default"
    )
}

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
            Err(TryRecvError::Empty) | Err(TryRecvError::Disconnected) => {
                return (0.0, 0.0);
            }
        }
    }
}

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

fn f32_as_le_bytes(samples: &[f32], out: &mut Vec<u8>) {
    out.clear();
    out.reserve(samples.len() * 4);
    for &sample in samples {
        out.extend_from_slice(&sample.to_le_bytes());
    }
}

fn spawn_quit_control(quit: Arc<AtomicBool>) {
    std::thread::spawn(move || {
        let stdin = std::io::stdin();
        loop {
            let mut line = String::new();
            match stdin.read_line(&mut line) {
                Ok(0) | Err(_) => {
                    quit.store(true, Ordering::Relaxed);
                    break;
                }
                Ok(_) if line.trim().eq_ignore_ascii_case("q") => {
                    quit.store(true, Ordering::Relaxed);
                    break;
                }
                Ok(_) => {}
            }
        }
    });
}
