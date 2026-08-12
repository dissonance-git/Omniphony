use anyhow::{Context, bail};
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
use renderer::music_field::{MUSIC_FIELD_CHANNELS, MusicFieldProcessor, MusicFieldSnapshot};
use renderer::music_foundation::MusicFoundationProcessor;
use std::collections::VecDeque;
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

use crate::music_support::{MusicSupportRenderer, SpatialProfile};

const SAMPLE_RATE_HZ: u32 = 48_000;
/// Burst cushion between the capture/DSP producer and WASAPI playback callback.
/// Capture is realtime, so a larger bounded capacity does not intentionally add
/// steady-state latency; it gives MMCSS a little more room to survive short CPU
/// scheduling stalls from heavy background compute.
const PLAYBACK_QUEUE_BLOCKS: usize = 32;
const FIELD_SUPPORT_GAIN: f32 = 1.00;
/// Fixed linear output headroom shared by ON and OFF.
const LINEAR_OUTPUT_GAIN: f32 = 0.90;
/// Fixed listening-level reclaim downstream of every spatial mechanism.
const OUTPUT_MAKEUP_DB: f32 = 3.5;
const OUTPUT_MAKEUP_GAIN: f32 = 1.496_235_6;
/// Conservative sample ceiling leaves margin for inter-sample reconstruction.
const OUTPUT_CEILING_DBFS: f32 = -1.0;
const OUTPUT_CEILING: f32 = 0.891_250_9;
const OUTPUT_LOOKAHEAD_FRAMES: usize = 240; // 5 ms at 48 kHz.
const OUTPUT_RELEASE_MS: f32 = 160.0;
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
                    "Omniphony protected-master full-sphere stereo renderer\n\n\
                     Runtime profile is selected with OMNIPHONY_PROFILE.\n\
                     Profiles: control|all|hybrid|direct|external|prtf|close|tracked|diffuse\n"
                );
                std::process::exit(0);
            }
            other => bail!("unknown argument: {other}"),
        }
    }
    Ok(parsed)
}

/// Final-bus safety only. This is not a loudness leveller or spatial AGC.
///
/// The guard adds fixed makeup gain, delays both channels equally, and applies
/// one stereo-linked attenuation envelope only when a future peak would cross
/// the endpoint ceiling. Relative L/R amplitude and upstream spatial relations
/// are preserved.
struct StereoLookaheadPeakGuard {
    frames: VecDeque<[f32; 2]>,
    gain: f32,
    release_coeff: f32,
    min_gain_since_report: f32,
}

impl StereoLookaheadPeakGuard {
    fn new(sample_rate_hz: u32) -> Self {
        let release_seconds = OUTPUT_RELEASE_MS / 1000.0;
        let release_coeff = (-1.0 / (release_seconds * sample_rate_hz.max(1) as f32)).exp();
        Self {
            frames: VecDeque::with_capacity(OUTPUT_LOOKAHEAD_FRAMES + 2),
            gain: 1.0,
            release_coeff,
            min_gain_since_report: 1.0,
        }
    }

    fn process_interleaved(&mut self, input: &[f32]) -> anyhow::Result<Vec<f32>> {
        if input.len() % 2 != 0 {
            bail!("output peak guard requires interleaved stereo samples");
        }
        let mut out = Vec::with_capacity(input.len());
        for frame in input.chunks_exact(2) {
            let left = if frame[0].is_finite() { frame[0] } else { 0.0 };
            let right = if frame[1].is_finite() { frame[1] } else { 0.0 };
            self.frames
                .push_back([left * OUTPUT_MAKEUP_GAIN, right * OUTPUT_MAKEUP_GAIN]);

            if self.frames.len() <= OUTPUT_LOOKAHEAD_FRAMES {
                continue;
            }

            let mut future_peak = 0.0_f32;
            let mut peak_index = 0usize;
            for (index, queued) in self.frames.iter().enumerate() {
                let peak = queued[0].abs().max(queued[1].abs());
                if peak > future_peak {
                    future_peak = peak;
                    peak_index = index;
                }
            }
            let target_gain = if future_peak > OUTPUT_CEILING {
                OUTPUT_CEILING / future_peak
            } else {
                1.0
            };

            if target_gain < self.gain {
                if peak_index == 0 {
                    self.gain = target_gain;
                } else {
                    self.gain += (target_gain - self.gain) / peak_index as f32;
                }
            } else {
                self.gain = target_gain - (target_gain - self.gain) * self.release_coeff;
            }

            let current = self
                .frames
                .pop_front()
                .expect("lookahead queue is non-empty");
            let current_peak = current[0].abs().max(current[1].abs());
            let immediate_safe_gain = if current_peak > OUTPUT_CEILING {
                OUTPUT_CEILING / current_peak
            } else {
                1.0
            };
            let applied_gain = self.gain.min(immediate_safe_gain).clamp(0.0, 1.0);
            self.gain = self.gain.min(applied_gain);
            self.min_gain_since_report = self.min_gain_since_report.min(applied_gain);
            out.push(current[0] * applied_gain);
            out.push(current[1] * applied_gain);
        }
        Ok(out)
    }

    fn take_max_reduction_db(&mut self) -> f32 {
        let reduction = if self.min_gain_since_report < 1.0 {
            -20.0 * self.min_gain_since_report.max(1.0e-6).log10()
        } else {
            0.0
        };
        self.min_gain_since_report = 1.0;
        reduction
    }
}

fn report_output_peak_guard(guard: &mut StereoLookaheadPeakGuard) {
    let reduction_db = guard.take_max_reduction_db();
    println!(
        "  output: +{OUTPUT_MAKEUP_DB:.1} dB makeup, ceiling={OUTPUT_CEILING_DBFS:.1} dBFS, max peak reduction={reduction_db:.2} dB"
    );
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
    let profile = SpatialProfile::from_env()?;
    let host = cpal::default_host();
    let output_device = choose_output_device(&host, args.output.as_deref())?;
    let output_name = output_device
        .name()
        .unwrap_or_else(|_| "<unavailable output name>".to_string());
    let (output_format, output_config) = choose_output_config(&output_device, SAMPLE_RATE_HZ)?;
    let mut loopback = LoopbackCapture::open_stereo(SAMPLE_RATE_HZ)?;
    let mut support_renderer = MusicSupportRenderer::new(profile, SAMPLE_RATE_HZ)?;

    println!("Omniphony for Headphones - protected-master full-sphere renderer");
    println!("  profile: {}", profile.as_str());
    println!("  capture: {SAMPLE_RATE_HZ} Hz / stereo / f32 process loopback");
    println!("  output:  {output_name}");
    println!("  direct:  captured stereo master remains authoritative");
    println!("  analysis: FFT magnitude + phase -> portable stereo/scene inference");
    println!("  field:   below 320 Hz protected; 320+ Hz uses 12 evidence lanes");
    println!("  height:  vertical extent from already-spatial evidence + coherent transfer");
    println!("  foundation: coherent pressure/body delta, no LFE/compression/saturation");
    println!(
        "  support route: {}",
        if support_renderer.is_hybrid() {
            "8 non-height lanes -> cascaded world; 4 height lanes -> direct HRTF; exclusive recombination"
        } else {
            "single native Omniphony spatial path"
        }
    );
    println!(
        "  support: {:.0}% derived-field mix, linear master+foundation+support summing",
        FIELD_SUPPORT_GAIN * 100.0
    );
    println!(
        "  output: {:.1} dB base trim + {OUTPUT_MAKEUP_DB:.1} dB makeup; {OUTPUT_CEILING_DBFS:.1} dBFS stereo-linked look-ahead safety ceiling",
        20.0 * LINEAR_OUTPUT_GAIN.log10()
    );
    println!("  realtime: producer + playback callback claim MMCSS; queue underruns are metered");

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
    let mut output_peak_guard = StereoLookaheadPeakGuard::new(SAMPLE_RATE_HZ);
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

        let output_reference = apply_output_headroom(&input);
        direct_meter.observe(&output_reference);

        if args.start_off {
            let output_reference = output_peak_guard.process_interleaved(&output_reference)?;
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
                report_output_peak_guard(&mut output_peak_guard);
                last_meter_report = Instant::now();
            }
            continue;
        }

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

        let rendered = support_renderer
            .process(&field_input)
            .context("live Omniphony support render failed")?;
        for block in rendered {
            if block.n_channels != 2 {
                bail!(
                    "music support renderer changed output width to {}",
                    block.n_channels
                );
            }
            if block.samples.is_empty() {
                continue;
            }
            if dry_fifo.len() < block.samples.len() || foundation_fifo.len() < block.samples.len() {
                bail!(
                    "music support produced {} samples with dry/foundation buffered at {}/{}",
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
            let mixed = output_peak_guard.process_interleaved(&mixed)?;
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
            report_output_peak_guard(&mut output_peak_guard);
            last_meter_report = Instant::now();
        }
    }

    let _ = loopback.stop();
    drop(playback_stream);
    println!("Omniphony frequency-evidence renderer stopped");
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
