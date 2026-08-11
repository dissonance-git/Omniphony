use anyhow::{Context, bail};
use bridge_api::RInputTransport;
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use orender_engine::Engine;
use renderer::music_field::{MUSIC_FIELD_CHANNELS, MusicFieldProcessor, MusicFieldSnapshot};
use renderer::music_foundation::MusicFoundationProcessor;
use std::collections::VecDeque;
use std::path::{Path, PathBuf};
use std::sync::{
    Arc,
    atomic::{AtomicBool, AtomicU64, Ordering},
    mpsc::{Receiver, TryRecvError, sync_channel},
};
use std::time::{Duration, Instant};
use wasapi::{
    AudioCaptureClient, AudioClient, Direction, SampleType, StreamMode, WaveFormat,
    make_channelmasks,
};

const SAMPLE_RATE_HZ: u32 = 48_000;
/// Burst cushion between the capture/DSP producer and WASAPI playback callback.
/// Capture is realtime, so a larger bounded capacity does not intentionally add
/// steady-state latency; it gives MMCSS a little more room to survive short CPU
/// scheduling stalls from heavy background Helix/research workloads.
const PLAYBACK_QUEUE_BLOCKS: usize = 32;
const FIELD_SUPPORT_GAIN: f32 = 1.00;
/// Fixed linear output headroom shared by ON and OFF.
///
/// The first clean-summing experiment reserved almost 7 dB unconditionally.
/// Physical listening showed that was too costly for an always-on music path.
/// Keep the summation purely linear but reclaim most of that level; this leaves
/// about 0.9 dB of fixed headroom while we gather real peak evidence from the
/// frontier build. Do not reintroduce sample-wise support clipping.
const LINEAR_OUTPUT_GAIN: f32 = 0.90;
const METER_INTERVAL_SECS: u64 = 5;

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
                    "Omniphony frequency-evidence full-sphere stereo prototype\n\n\
                     Usage:\n  Omniphony.exe (internal engine mode)\n\n\
                     ON preserves the captured stereo master and analyzes real\n\
                     L/R magnitude/phase relationships by frequency. Portable\n\
                     Omniphony evidence laws derive broad, lateral, diffuse and\n\
                     vertical support across a logical 7.1.4 shell while\n\
                     protecting center authority and bass.\n"
                );
                std::process::exit(0);
            }
            other => bail!("unknown argument: {other}"),
        }
    }
    Ok(parsed)
}

#[derive(Default)]
struct SignalMeter {
    sum_squares: f64,
    peak: f32,
    samples: u64,
}

impl SignalMeter {
    fn observe(&mut self, samples: &[f32]) {
        for &sample in samples {
            if !sample.is_finite() {
                continue;
            }
            self.sum_squares += f64::from(sample) * f64::from(sample);
            self.peak = self.peak.max(sample.abs());
            self.samples = self.samples.saturating_add(1);
        }
    }

    fn observe_delta(&mut self, mixed: &[f32], dry: &[f32]) {
        for (&wet, &base) in mixed.iter().zip(dry.iter()) {
            let delta = wet - base;
            if !delta.is_finite() {
                continue;
            }
            self.sum_squares += f64::from(delta) * f64::from(delta);
            self.peak = self.peak.max(delta.abs());
            self.samples = self.samples.saturating_add(1);
        }
    }

    fn rms_dbfs(&self) -> f32 {
        if self.samples == 0 {
            return -120.0;
        }
        to_dbfs((self.sum_squares / self.samples as f64).sqrt() as f32)
    }

    fn peak_dbfs(&self) -> f32 {
        to_dbfs(self.peak)
    }

    fn clear(&mut self) {
        *self = Self::default();
    }
}

fn to_dbfs(value: f32) -> f32 {
    if !value.is_finite() || value <= 1.0e-6 {
        -120.0
    } else {
        20.0 * value.log10()
    }
}

fn print_meters(
    direct: &mut SignalMeter,
    evidence: &mut SignalMeter,
    rendered: &mut SignalMeter,
    added: &mut SignalMeter,
    scene: MusicFieldSnapshot,
    bypass: bool,
) {
    if bypass {
        println!(
            "  meter OFF: direct rms={:.1} dBFS peak={:.1} dBFS",
            direct.rms_dbfs(),
            direct.peak_dbfs()
        );
    } else {
        let direct_rms = direct.rms_dbfs();
        let added_rms = added.rms_dbfs();
        println!(
            "  meter ON: direct={direct_rms:.1}/{:.1} dBFS | field={:.1}/{:.1} | rendered={:.1}/{:.1} | added={added_rms:.1}/{:.1} | added/direct={:+.1} dB",
            direct.peak_dbfs(),
            evidence.rms_dbfs(),
            evidence.peak_dbfs(),
            rendered.rms_dbfs(),
            rendered.peak_dbfs(),
            added.peak_dbfs(),
            added_rms - direct_rms,
        );
        println!(
            "  evidence: anchor={:.2} broad={:.2} lateral={:.2} diffuse={:.2} height={:.2} pan={:+.2} side={:.2}",
            scene.anchor,
            scene.broad,
            scene.lateral,
            scene.diffuse,
            scene.height,
            scene.lateral_pan,
            scene.side_fraction,
        );
    }
    direct.clear();
    evidence.clear();
    rendered.clear();
    added.clear();
}

fn report_playback_underruns(counter: &AtomicU64) {
    let frames = counter.swap(0, Ordering::Relaxed);
    if frames == 0 {
        return;
    }
    let duration_ms = frames as f64 * 1000.0 / SAMPLE_RATE_HZ as f64;
    eprintln!(
        "  realtime warning: WASAPI playback queue starved for {frames} frame(s) (~{duration_ms:.2} ms) in the last meter interval"
    );
}

pub fn run() -> anyhow::Result<()> {
    let args = parse_args()?;
    let host = cpal::default_host();
    let output_device = choose_output_device(&host, args.output.as_deref())?;
    let output_name = output_device
        .name()
        .unwrap_or_else(|_| "<unavailable output name>".to_string());
    let (output_format, output_config) = choose_output_config(&output_device, SAMPLE_RATE_HZ)?;
    let mut loopback = LoopbackCapture::open_stereo(SAMPLE_RATE_HZ)?;

    println!("Omniphony for Headphones - frequency-evidence 7.1.4 shell prototype");
    println!("  capture: {SAMPLE_RATE_HZ} Hz / stereo / f32 process loopback");
    println!("  output:  {output_name}");
    println!("  direct:  captured stereo master remains authoritative");
    println!("  analysis: FFT magnitude + phase -> portable stereo/scene inference");
    println!("  field:   below 320 Hz protected; 320+ Hz derived 7.1.4 support");
    println!("  height:  vertical extent from already-spatial evidence");
    println!("  foundation: coherent pressure/body delta, no LFE/compression/saturation");
    println!(
        "  support: {:.0}% derived-field mix, linear master+foundation+support summing",
        FIELD_SUPPORT_GAIN * 100.0
    );
    println!(
        "  headroom: {:.1} dB fixed linear output gain, identical ON/OFF reference gain",
        20.0 * LINEAR_OUTPUT_GAIN.log10()
    );
    println!(
        "  acoustics: cascaded Omniphony virtual room / distance / HRTF / reflections / air cues"
    );
    println!("  realtime: producer + playback callback claim MMCSS; queue underruns are metered");

    let bundle = Bundle::embedded()?;
    orender_engine::bridge_loader::register_linked_bridge(reference_bridge::linked_library)
        .context("failed to register linked reference PCM bridge")?;
    let mut field_engine = Engine::from_paths(
        Some(&bundle.field_config),
        Some(&bundle.layout),
        None,
        None,
        SAMPLE_RATE_HZ,
    )
    .context("failed to construct Omniphony music-field engine")?;
    field_engine.set_channel_render_mode_code(1);
    if field_engine.channel_count() != 2 {
        bail!(
            "binaural support-field configuration expected 2 output channels but engine reports {}",
            field_engine.channel_count()
        );
    }

    let header = streaming_f32_wav_header(MUSIC_FIELD_CHANNELS as u16, SAMPLE_RATE_HZ);
    let header_output = field_engine
        .process(&header, RInputTransport::Raw, 0)
        .context("failed to seed 7.1.4 music-field PCM bridge")?;
    if !header_output.is_empty() {
        bail!("streaming WAV header unexpectedly produced audio");
    }

    let quit = Arc::new(AtomicBool::new(false));
    let playback_underrun_frames = Arc::new(AtomicU64::new(0));
    let playback_failed = Arc::new(AtomicBool::new(false));
    let (play_tx, play_rx) = sync_channel::<Vec<f32>>(PLAYBACK_QUEUE_BLOCKS);
    let playback_stream = build_playback_stream(
        &output_device,
        &output_config,
        output_format,
        play_rx,
        Arc::clone(&playback_underrun_frames),
        Arc::clone(&playback_failed),
    )?;
    spawn_quit_control(Arc::clone(&quit));
    playback_stream
        .play()
        .context("failed to start WASAPI playback stream")?;
    loopback.start()?;

    let mut field = MusicFieldProcessor::new(SAMPLE_RATE_HZ);
    let mut foundation = MusicFoundationProcessor::new(SAMPLE_RATE_HZ);
    let mut pcm_bytes = Vec::<u8>::new();
    let mut dry_fifo = VecDeque::<f32>::new();
    let mut foundation_fifo = VecDeque::<f32>::new();
    let mut direct_meter = SignalMeter::default();
    let mut evidence_meter = SignalMeter::default();
    let mut rendered_meter = SignalMeter::default();
    let mut added_meter = SignalMeter::default();
    let mut last_meter_report = Instant::now();

    println!();
    println!(
        "LIVE. Omniphony is {}.",
        if args.start_off { "OFF" } else { "ON" }
    );

    while !quit.load(Ordering::Relaxed) {
        if playback_failed.load(Ordering::Acquire) {
            bail!("WASAPI playback stream failed; supervisor will restart the audio engine");
        }
        let Some(input) = loopback.next_block()? else {
            std::thread::sleep(Duration::from_millis(1));
            continue;
        };
        if input.is_empty() || input.len() % 2 != 0 {
            continue;
        }

        // Meter the raw reference at the same fixed output gain used by the ON
        // path. The foundation delta is intentionally an ON-path enhancement;
        // it is not folded into the clean OFF reference.
        let output_reference = apply_output_headroom(&input);
        direct_meter.observe(&output_reference);

        if args.start_off {
            queue_block(&play_tx, output_reference)?;
            if last_meter_report.elapsed() >= Duration::from_secs(METER_INTERVAL_SECS) {
                print_meters(
                    &mut direct_meter,
                    &mut evidence_meter,
                    &mut rendered_meter,
                    &mut added_meter,
                    field.snapshot(),
                    true,
                );
                report_playback_underruns(&playback_underrun_frames);
                last_meter_report = Instant::now();
            }
            continue;
        }

        // Both additive branches are causal and produce one aligned stereo
        // foundation sample / one 7.1.4 support frame per input frame. Buffer
        // them beside the authoritative dry master while the inherited renderer
        // contributes its own bridge/binaural latency.
        dry_fifo.extend(input.iter().copied());
        let foundation_delta = foundation.process_interleaved_delta(&input);
        if foundation_delta.len() != input.len() {
            bail!(
                "music foundation width mismatch: stereo samples={} foundation samples={}",
                input.len(),
                foundation_delta.len()
            );
        }
        foundation_fifo.extend(foundation_delta);

        let field_input = field.process_interleaved_stereo(&input);
        if field_input.len() != (input.len() / 2) * MUSIC_FIELD_CHANNELS {
            bail!(
                "music field width mismatch: stereo samples={} field samples={}",
                input.len(),
                field_input.len()
            );
        }
        evidence_meter.observe(&field_input);
        f32_as_le_bytes(&field_input, &mut pcm_bytes);

        let rendered = field_engine
            .process(&pcm_bytes, RInputTransport::Raw, 0)
            .context("live Omniphony frequency-evidence field render failed")?;
        for block in rendered {
            if block.n_channels != 2 {
                bail!(
                    "music field renderer changed output width to {}",
                    block.n_channels
                );
            }
            if block.samples.is_empty() {
                continue;
            }
            if dry_fifo.len() < block.samples.len() || foundation_fifo.len() < block.samples.len() {
                bail!(
                    "music field produced {} samples with dry/foundation buffered at {}/{}",
                    block.samples.len(),
                    dry_fifo.len(),
                    foundation_fifo.len()
                );
            }
            rendered_meter.observe(&block.samples);
            let mut dry = Vec::with_capacity(block.samples.len());
            let mut foundation_delta = Vec::with_capacity(block.samples.len());
            for _ in 0..block.samples.len() {
                dry.push(dry_fifo.pop_front().expect("dry FIFO length checked above"));
                foundation_delta.push(
                    foundation_fifo
                        .pop_front()
                        .expect("foundation FIFO length checked above"),
                );
            }
            let mixed = mix_preserved_master_with_support(
                &dry,
                &foundation_delta,
                &block.samples,
                FIELD_SUPPORT_GAIN,
            )?;
            let dry_reference = apply_output_headroom(&dry);
            added_meter.observe_delta(&mixed, &dry_reference);
            queue_block(&play_tx, mixed)?;
        }

        if last_meter_report.elapsed() >= Duration::from_secs(METER_INTERVAL_SECS) {
            print_meters(
                &mut direct_meter,
                &mut evidence_meter,
                &mut rendered_meter,
                &mut added_meter,
                field.snapshot(),
                false,
            );
            report_playback_underruns(&playback_underrun_frames);
            last_meter_report = Instant::now();
        }
    }

    let _ = loopback.stop();
    drop(playback_stream);
    println!("Omniphony frequency-evidence prototype stopped");
    Ok(())
}

fn apply_output_headroom(samples: &[f32]) -> Vec<f32> {
    samples
        .iter()
        .map(|&sample| {
            if sample.is_finite() {
                sample * LINEAR_OUTPUT_GAIN
            } else {
                0.0
            }
        })
        .collect()
}

fn mix_preserved_master_with_support(
    dry: &[f32],
    foundation: &[f32],
    support: &[f32],
    support_gain: f32,
) -> anyhow::Result<Vec<f32>> {
    if dry.len() != support.len() || dry.len() != foundation.len() {
        bail!(
            "support-field length mismatch: dry={} foundation={} field={} samples",
            dry.len(),
            foundation.len(),
            support.len()
        );
    }
    let gain = support_gain.clamp(0.0, 1.0);
    let mut out = Vec::with_capacity(dry.len());
    for ((&base, &body), &field) in dry.iter().zip(foundation.iter()).zip(support.iter()) {
        let base = if base.is_finite() { base } else { 0.0 };
        let body = if body.is_finite() { body } else { 0.0 };
        let field = if field.is_finite() { field } else { 0.0 };
        // Pure linear superposition: protected master + coherent foundation delta
        // + inherited-renderer support. Fixed downstream headroom owns level;
        // nothing here clips, limits, saturates or dynamically reshapes samples.
        out.push((base + body + field * gain) * LINEAR_OUTPUT_GAIN);
    }
    Ok(out)
}

fn queue_block(tx: &std::sync::mpsc::SyncSender<Vec<f32>>, block: Vec<f32>) -> anyhow::Result<()> {
    if block.is_empty() {
        return Ok(());
    }
    tx.send(block)
        .map_err(|_| anyhow::anyhow!("WASAPI playback stream disconnected"))
}

struct LoopbackCapture {
    client: AudioClient,
    capture: AudioCaptureClient,
    scratch: Vec<u8>,
}

impl LoopbackCapture {
    fn open_stereo(sample_rate_hz: u32) -> anyhow::Result<Self> {
        const BUFFER_DURATION_HNS: i64 = 200_000;
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
            .context("Windows process loopback rejected required stereo 48 kHz float format")?;
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
    field_config: PathBuf,
    layout: PathBuf,
}

impl Bundle {
    fn embedded() -> anyhow::Result<Self> {
        const FIELD_CONFIG: &str =
            include_str!("../../assets/binaural-baselines/stereo-field-prototype.yaml");
        const LAYOUT: &str = include_str!("../../../layouts/7.1.4.yaml");

        let root = std::env::var_os("LOCALAPPDATA")
            .map(PathBuf::from)
            .unwrap_or_else(std::env::temp_dir)
            .join("Omniphony")
            .join("runtime");
        std::fs::create_dir_all(&root)
            .context("failed to create embedded Omniphony runtime directory")?;
        let field_config = root.join("stereo-field-prototype.yaml");
        let layout = root.join("7.1.4.yaml");
        write_embedded_asset(&field_config, FIELD_CONFIG)?;
        write_embedded_asset(&layout, LAYOUT)?;
        Ok(Self {
            field_config,
            layout,
        })
    }
}

fn write_embedded_asset(path: &Path, content: &str) -> anyhow::Result<()> {
    let current = std::fs::read_to_string(path).ok();
    if current.as_deref() != Some(content) {
        std::fs::write(path, content)
            .with_context(|| format!("failed to materialize {}", path.display()))?;
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
    bail!("no physical output was auto-detected; expected FiiO or non-cable Windows default")
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
    let config = best
        .with_sample_rate(cpal::SampleRate(sample_rate_hz))
        .config();
    Ok((sample_format, config))
}

fn build_playback_stream(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    format: cpal::SampleFormat,
    rx: Receiver<Vec<f32>>,
    underrun_frames: Arc<AtomicU64>,
    stream_failed: Arc<AtomicBool>,
) -> anyhow::Result<cpal::Stream> {
    match format {
        cpal::SampleFormat::I8 => build_typed_playback::<i8>(
            device,
            config,
            rx,
            underrun_frames,
            Arc::clone(&stream_failed),
        ),
        cpal::SampleFormat::I16 => build_typed_playback::<i16>(
            device,
            config,
            rx,
            underrun_frames,
            Arc::clone(&stream_failed),
        ),
        cpal::SampleFormat::I32 => build_typed_playback::<i32>(
            device,
            config,
            rx,
            underrun_frames,
            Arc::clone(&stream_failed),
        ),
        cpal::SampleFormat::I64 => build_typed_playback::<i64>(
            device,
            config,
            rx,
            underrun_frames,
            Arc::clone(&stream_failed),
        ),
        cpal::SampleFormat::U8 => build_typed_playback::<u8>(
            device,
            config,
            rx,
            underrun_frames,
            Arc::clone(&stream_failed),
        ),
        cpal::SampleFormat::U16 => build_typed_playback::<u16>(
            device,
            config,
            rx,
            underrun_frames,
            Arc::clone(&stream_failed),
        ),
        cpal::SampleFormat::U32 => build_typed_playback::<u32>(
            device,
            config,
            rx,
            underrun_frames,
            Arc::clone(&stream_failed),
        ),
        cpal::SampleFormat::U64 => build_typed_playback::<u64>(
            device,
            config,
            rx,
            underrun_frames,
            Arc::clone(&stream_failed),
        ),
        cpal::SampleFormat::F32 => build_typed_playback::<f32>(
            device,
            config,
            rx,
            underrun_frames,
            Arc::clone(&stream_failed),
        ),
        cpal::SampleFormat::F64 => build_typed_playback::<f64>(
            device,
            config,
            rx,
            underrun_frames,
            Arc::clone(&stream_failed),
        ),
        other => bail!("unsupported WASAPI output sample format: {other:?}"),
    }
}

fn build_typed_playback<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    rx: Receiver<Vec<f32>>,
    underrun_frames: Arc<AtomicU64>,
    stream_failed: Arc<AtomicBool>,
) -> anyhow::Result<cpal::Stream>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    let channels = usize::from(config.channels);
    let mut current = Vec::<f32>::new();
    let mut cursor = 0usize;
    let mut callback_mmcss = None;
    let err_fn = move |err| {
        eprintln!("WASAPI playback stream error: {err}");
        stream_failed.store(true, Ordering::Release);
    };
    device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                // CPAL invokes this closure on its own realtime playback thread;
                // the producer thread's MMCSS registration does not cover it.
                if callback_mmcss.is_none() {
                    callback_mmcss = crate::realtime_priority::claim_realtime_audio();
                }
                for frame in data.chunks_exact_mut(channels) {
                    let (left, right) =
                        next_stereo_frame(&rx, &mut current, &mut cursor, &underrun_frames);
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
    underrun_frames: &AtomicU64,
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
                underrun_frames.fetch_add(1, Ordering::Relaxed);
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
    wav.extend_from_slice(&3u16.to_le_bytes());
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
