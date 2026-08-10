#[cfg(target_os = "windows")]
mod capture;

#[cfg(target_os = "windows")]
use anyhow::{Context, bail};
#[cfg(target_os = "windows")]
use bridge_api::RInputTransport;
#[cfg(target_os = "windows")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(target_os = "windows")]
use omniphony_realtime::{
    OmniphonyRealtimeConfig, OmniphonyRealtimeProcessor, omniphony_realtime_create,
    omniphony_realtime_destroy, omniphony_realtime_process_f32,
};
#[cfg(target_os = "windows")]
use orender_engine::Engine;
#[cfg(target_os = "windows")]
use std::path::{Path, PathBuf};
#[cfg(target_os = "windows")]
use std::sync::{
    Arc,
    atomic::{AtomicBool, Ordering},
};
#[cfg(target_os = "windows")]
use std::time::{Duration, Instant};

fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        return run_windows_probe();
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("windows_host is only available on Windows");
    }
}

#[cfg(target_os = "windows")]
fn run_windows_probe() -> anyhow::Result<()> {
    let args: Vec<_> = std::env::args_os().collect();

    // CI can exercise the *packaged* bridge + config + layout + renderer without
    // requiring a cloud runner to expose a physical WASAPI endpoint. Keep this
    // branch before CPAL host/device discovery so render validation is genuinely
    // independent of Windows audio hardware.
    if args.iter().any(|arg| arg == "--render-reference-only") {
        println!("Omniphony for Headphones packaged reference validation");
        let rendered = render_reference_scene()?;
        println!(
            "packaged protected reference validated: {} stereo frames",
            rendered.len() / 2
        );
        return Ok(());
    }

    let host = cpal::default_host();

    println!("Omniphony for Headphones native Windows audio host");
    println!("backend: WASAPI (CPAL default Windows host)");
    println!("prototype: native transport + protected upstream binaural reference");

    let default_output = host.default_output_device();
    match default_output.as_ref() {
        Some(device) => {
            let name = device
                .name()
                .unwrap_or_else(|_| "<unavailable device name>".to_string());
            println!("default output: {name}");
        }
        None => println!("default output: <none>"),
    }

    println!("available outputs:");
    for device in host.output_devices()? {
        let name = device
            .name()
            .unwrap_or_else(|_| "<unavailable device name>".to_string());
        println!("  {name}");
    }

    if args.iter().any(|arg| arg == "--probe-loopback") {
        println!("probing self-excluding system loopback capture...");
        capture::probe_self_excluding_system_loopback()?;
    } else {
        println!("loopback capture: not activated (pass --probe-loopback to test it)");
    }

    if args.iter().any(|arg| arg == "--smoke-output") {
        let device = default_output
            .as_ref()
            .context("Windows has no default output device")?;
        run_output_smoke(device)?;
    } else {
        println!("output smoke: not activated (pass --smoke-output to hear the native path)");
    }

    if args.iter().any(|arg| arg == "--reference-demo") {
        let device = default_output.context("Windows has no default output device")?;
        run_reference_demo(&device)?;
    } else {
        println!(
            "reference demo: not activated (pass --reference-demo to hear protected Omniphony)"
        );
    }

    Ok(())
}

#[cfg(target_os = "windows")]
struct RealtimeProcessorHandle {
    ptr: *mut OmniphonyRealtimeProcessor,
}

#[cfg(target_os = "windows")]
unsafe impl Send for RealtimeProcessorHandle {}

#[cfg(target_os = "windows")]
impl RealtimeProcessorHandle {
    fn new(sample_rate_hz: u32, channels: u32) -> anyhow::Result<Self> {
        let config = OmniphonyRealtimeConfig {
            sample_rate_hz,
            channels,
        };
        let ptr = unsafe { omniphony_realtime_create(&config) };
        if ptr.is_null() {
            bail!(
                "realtime PCM processor rejected {sample_rate_hz} Hz / {channels} channel config"
            );
        }
        Ok(Self { ptr })
    }

    fn process_in_place(&mut self, samples: &mut [f32], frames: usize) -> anyhow::Result<()> {
        let rc = unsafe {
            omniphony_realtime_process_f32(
                self.ptr,
                samples.as_ptr(),
                samples.as_mut_ptr(),
                frames,
            )
        };
        if rc != 0 {
            bail!("realtime PCM processor returned error {rc}");
        }
        Ok(())
    }
}

#[cfg(target_os = "windows")]
impl Drop for RealtimeProcessorHandle {
    fn drop(&mut self) {
        unsafe { omniphony_realtime_destroy(self.ptr) };
    }
}

#[cfg(target_os = "windows")]
fn run_output_smoke(device: &cpal::Device) -> anyhow::Result<()> {
    const SMOKE_DURATION: Duration = Duration::from_millis(2_000);

    let supported = device
        .default_output_config()
        .context("failed to query default WASAPI output format")?;
    let sample_format = supported.sample_format();
    let config: cpal::StreamConfig = supported.into();
    let device_name = device
        .name()
        .unwrap_or_else(|_| "<unavailable device name>".to_string());

    println!("starting native output smoke test");
    println!("  device: {device_name}");
    println!("  sample rate: {} Hz", config.sample_rate.0);
    println!("  channels: {}", config.channels);
    println!("  device sample format: {sample_format:?}");
    println!("  realtime seam: omniphony_realtime identity");
    println!("  signal: 440 Hz, low level, 2 seconds");

    match sample_format {
        cpal::SampleFormat::I8 => run_typed_tone_output::<i8>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::I16 => run_typed_tone_output::<i16>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::I32 => run_typed_tone_output::<i32>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::I64 => run_typed_tone_output::<i64>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::U8 => run_typed_tone_output::<u8>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::U16 => run_typed_tone_output::<u16>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::U32 => run_typed_tone_output::<u32>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::U64 => run_typed_tone_output::<u64>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::F32 => run_typed_tone_output::<f32>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::F64 => run_typed_tone_output::<f64>(device, &config, SMOKE_DURATION),
        other => bail!("unsupported default WASAPI sample format: {other:?}"),
    }
}

#[cfg(target_os = "windows")]
fn run_typed_tone_output<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    duration: Duration,
) -> anyhow::Result<()>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    let sample_rate_hz = config.sample_rate.0;
    let channels = usize::from(config.channels);
    if channels == 0 {
        bail!("default WASAPI output reported zero channels");
    }

    let tone_frames = (duration.as_secs_f64() * f64::from(sample_rate_hz)).round() as u64;
    let ramp_frames = (sample_rate_hz / 100).max(1) as u64;
    let mut frame_index = 0u64;
    let mut scratch = Vec::<f32>::new();
    let mut processor = RealtimeProcessorHandle::new(sample_rate_hz, config.channels.into())?;
    let mut reported_processing_error = false;

    let err_fn = |err| eprintln!("WASAPI output stream error: {err}");

    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let frames = data.len() / channels;
                let sample_count = frames * channels;
                scratch.resize(sample_count, 0.0);

                for frame in scratch.chunks_exact_mut(channels) {
                    let value = if frame_index < tone_frames {
                        let remaining = tone_frames.saturating_sub(frame_index);
                        let fade_in = (frame_index as f32 / ramp_frames as f32).min(1.0);
                        let fade_out = (remaining as f32 / ramp_frames as f32).min(1.0);
                        let envelope = fade_in.min(fade_out);
                        let phase = 2.0
                            * std::f32::consts::PI
                            * 440.0
                            * frame_index as f32
                            / sample_rate_hz as f32;
                        0.08 * envelope * phase.sin()
                    } else {
                        0.0
                    };
                    frame.fill(value);
                    frame_index = frame_index.saturating_add(1);
                }

                if let Err(err) = processor.process_in_place(&mut scratch, frames) {
                    scratch.fill(0.0);
                    if !reported_processing_error {
                        eprintln!("native realtime seam failed; silencing output: {err:#}");
                        reported_processing_error = true;
                    }
                }

                for (dst, src) in data.iter_mut().zip(scratch.iter().copied()) {
                    *dst = T::from_sample(src);
                }
            },
            err_fn,
            None,
        )
        .context("failed to create default WASAPI output stream")?;

    stream.play().context("failed to start WASAPI output stream")?;
    std::thread::sleep(duration + Duration::from_millis(150));
    drop(stream);

    println!("native output smoke test complete");
    Ok(())
}

#[cfg(target_os = "windows")]
struct ReferenceBundle {
    bridge: PathBuf,
    config: PathBuf,
    layout: PathBuf,
    wav: PathBuf,
}

#[cfg(target_os = "windows")]
impl ReferenceBundle {
    fn beside_executable() -> anyhow::Result<Self> {
        let exe =
            std::env::current_exe().context("failed to resolve windows_host executable path")?;
        let root = exe
            .parent()
            .context("windows_host executable has no parent directory")?;
        let reference = root.join("reference-demo");
        let bundle = Self {
            bridge: root.join("reference_bridge.dll"),
            config: reference.join("upstream-demo-reference.yaml"),
            layout: reference.join("7.1.4.yaml"),
            wav: reference.join("spatial-demo.wav"),
        };
        bundle.validate()?;
        Ok(bundle)
    }

    fn validate(&self) -> anyhow::Result<()> {
        require_file(&self.bridge, "reference decoder bridge")?;
        require_file(&self.config, "protected upstream binaural config")?;
        require_file(&self.layout, "7.1.4 reference layout")?;
        require_file(&self.wav, "rotating reference WAV")?;
        Ok(())
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
fn render_reference_scene() -> anyhow::Result<Vec<f32>> {
    const REFERENCE_SAMPLE_RATE: u32 = 48_000;
    const REFERENCE_CHANNELS: u32 = 2;
    const INPUT_CHUNK_BYTES: usize = 64 * 1024;

    let bundle = ReferenceBundle::beside_executable()?;
    println!("rendering protected Omniphony reference scene...");
    println!("  bridge: {}", bundle.bridge.display());
    println!("  config: {}", bundle.config.display());
    println!("  layout: {}", bundle.layout.display());
    println!("  source: {}", bundle.wav.display());

    let mut engine = Engine::from_paths(
        Some(&bundle.config),
        Some(&bundle.layout),
        Some(&bundle.bridge),
        None,
        REFERENCE_SAMPLE_RATE,
    )
    .context("failed to construct protected Omniphony reference engine")?;

    if engine.channel_count() != REFERENCE_CHANNELS {
        bail!(
            "protected reference expected binaural stereo but engine reports {} channels",
            engine.channel_count()
        );
    }

    let input = std::fs::read(&bundle.wav)
        .with_context(|| format!("failed to read {}", bundle.wav.display()))?;
    let mut rendered = Vec::<f32>::new();

    for chunk in input.chunks(INPUT_CHUNK_BYTES) {
        let blocks = engine
            .process(chunk, RInputTransport::Raw, 0)
            .context("reference bridge/renderer failed while processing demo WAV")?;
        for block in blocks {
            if block.n_channels != REFERENCE_CHANNELS {
                bail!(
                    "renderer changed output width inside reference demo: {} channels",
                    block.n_channels
                );
            }
            rendered.extend_from_slice(&block.samples);
        }
    }

    if rendered.is_empty() {
        bail!("protected reference renderer produced no audio");
    }
    if rendered.len() % REFERENCE_CHANNELS as usize != 0 {
        bail!("protected reference renderer produced a partial stereo frame");
    }

    let mut peak = 0.0f32;
    let mut square_sum = 0.0f64;
    for &sample in &rendered {
        if !sample.is_finite() {
            bail!("protected reference renderer produced a non-finite sample");
        }
        peak = peak.max(sample.abs());
        square_sum += f64::from(sample) * f64::from(sample);
    }
    if peak <= 1.0e-7 {
        bail!("protected reference renderer produced only silence");
    }

    let frame_count = rendered.len() / REFERENCE_CHANNELS as usize;
    let rms = (square_sum / rendered.len() as f64).sqrt();
    println!(
        "reference render complete: {:.2}s stereo @ {} Hz | peak {:.6} | rms {:.6}",
        frame_count as f64 / f64::from(REFERENCE_SAMPLE_RATE),
        REFERENCE_SAMPLE_RATE,
        peak,
        rms
    );
    Ok(rendered)
}

#[cfg(target_os = "windows")]
fn run_reference_demo(device: &cpal::Device) -> anyhow::Result<()> {
    const REFERENCE_SAMPLE_RATE: u32 = 48_000;

    let rendered = render_reference_scene()?;
    println!("playing reference over native WASAPI...");
    play_stereo_f32(device, rendered, REFERENCE_SAMPLE_RATE)?;
    println!("protected Omniphony reference playback complete");
    Ok(())
}

#[cfg(target_os = "windows")]
fn choose_stereo_output_config(
    device: &cpal::Device,
    sample_rate_hz: u32,
) -> anyhow::Result<(cpal::SampleFormat, cpal::StreamConfig)> {
    let supported = device
        .supported_output_configs()
        .context("failed to enumerate WASAPI output formats")?;

    let best = supported
        .filter(|range| {
            range.channels() >= 2
                && range.min_sample_rate().0 <= sample_rate_hz
                && range.max_sample_rate().0 >= sample_rate_hz
        })
        .min_by_key(|range| {
            let format_rank = match range.sample_format() {
                cpal::SampleFormat::F32 => 0u8,
                cpal::SampleFormat::F64 => 1,
                cpal::SampleFormat::I32 | cpal::SampleFormat::U32 => 2,
                cpal::SampleFormat::I16 | cpal::SampleFormat::U16 => 3,
                cpal::SampleFormat::I8 | cpal::SampleFormat::U8 => 4,
                cpal::SampleFormat::I64 | cpal::SampleFormat::U64 => 5,
                _ => 6,
            };
            (range.channels(), format_rank)
        })
        .with_context(|| format!("default WASAPI device has no >=2ch {sample_rate_hz} Hz output"))?;

    let sample_format = best.sample_format();
    let config = best.with_sample_rate(cpal::SampleRate(sample_rate_hz)).config();
    Ok((sample_format, config))
}

#[cfg(target_os = "windows")]
fn play_stereo_f32(
    device: &cpal::Device,
    rendered: Vec<f32>,
    sample_rate_hz: u32,
) -> anyhow::Result<()> {
    let (sample_format, config) = choose_stereo_output_config(device, sample_rate_hz)?;
    let device_name = device
        .name()
        .unwrap_or_else(|_| "<unavailable device name>".to_string());
    println!("  device: {device_name}");
    println!(
        "  stream: {} Hz / {}ch / {sample_format:?}",
        config.sample_rate.0, config.channels
    );
    println!("  renderer payload: stereo f32");
    println!("  realtime seam: omniphony_realtime identity");

    match sample_format {
        cpal::SampleFormat::I8 => run_typed_stereo_output::<i8>(device, &config, rendered),
        cpal::SampleFormat::I16 => run_typed_stereo_output::<i16>(device, &config, rendered),
        cpal::SampleFormat::I32 => run_typed_stereo_output::<i32>(device, &config, rendered),
        cpal::SampleFormat::I64 => run_typed_stereo_output::<i64>(device, &config, rendered),
        cpal::SampleFormat::U8 => run_typed_stereo_output::<u8>(device, &config, rendered),
        cpal::SampleFormat::U16 => run_typed_stereo_output::<u16>(device, &config, rendered),
        cpal::SampleFormat::U32 => run_typed_stereo_output::<u32>(device, &config, rendered),
        cpal::SampleFormat::U64 => run_typed_stereo_output::<u64>(device, &config, rendered),
        cpal::SampleFormat::F32 => run_typed_stereo_output::<f32>(device, &config, rendered),
        cpal::SampleFormat::F64 => run_typed_stereo_output::<f64>(device, &config, rendered),
        other => bail!("unsupported WASAPI sample format for reference demo: {other:?}"),
    }
}

#[cfg(target_os = "windows")]
fn run_typed_stereo_output<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    rendered: Vec<f32>,
) -> anyhow::Result<()>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    let device_channels = usize::from(config.channels);
    if device_channels < 2 {
        bail!("reference playback requires at least two output channels");
    }
    if rendered.len() % 2 != 0 {
        bail!("reference playback received a partial stereo frame");
    }

    let total_frames = rendered.len() / 2;
    let expected_duration =
        Duration::from_secs_f64(total_frames as f64 / f64::from(config.sample_rate.0));
    let done = Arc::new(AtomicBool::new(false));
    let done_for_callback = Arc::clone(&done);
    let mut cursor = 0usize;
    let mut scratch = Vec::<f32>::new();
    let mut processor = RealtimeProcessorHandle::new(config.sample_rate.0, 2)?;
    let mut reported_processing_error = false;

    let err_fn = |err| eprintln!("WASAPI reference output stream error: {err}");
    let stream = device
        .build_output_stream(
            config,
            move |data: &mut [T], _: &cpal::OutputCallbackInfo| {
                let frames = data.len() / device_channels;
                scratch.resize(frames * 2, 0.0);

                for frame in scratch.chunks_exact_mut(2) {
                    if cursor < total_frames {
                        frame[0] = rendered[cursor * 2];
                        frame[1] = rendered[cursor * 2 + 1];
                        cursor += 1;
                    } else {
                        frame.fill(0.0);
                    }
                }

                if let Err(err) = processor.process_in_place(&mut scratch, frames) {
                    scratch.fill(0.0);
                    if !reported_processing_error {
                        eprintln!("native realtime seam failed; silencing output: {err:#}");
                        reported_processing_error = true;
                    }
                }

                let zero = T::from_sample(0.0f32);
                for (frame_index, output_frame) in
                    data.chunks_exact_mut(device_channels).enumerate()
                {
                    output_frame.fill(zero);
                    output_frame[0] = T::from_sample(scratch[frame_index * 2]);
                    output_frame[1] = T::from_sample(scratch[frame_index * 2 + 1]);
                }

                if cursor >= total_frames {
                    done_for_callback.store(true, Ordering::Release);
                }
            },
            err_fn,
            None,
        )
        .context("failed to create WASAPI stream for protected reference")?;

    stream
        .play()
        .context("failed to start protected reference WASAPI stream")?;

    let deadline = Instant::now() + expected_duration + Duration::from_secs(5);
    while !done.load(Ordering::Acquire) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    if !done.load(Ordering::Acquire) {
        bail!("WASAPI reference playback did not complete before safety deadline");
    }
    std::thread::sleep(Duration::from_millis(100));
    drop(stream);
    Ok(())
}
