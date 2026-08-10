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
        return run();
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("omniphony_listen is only available on Windows");
    }
}

#[cfg(target_os = "windows")]
fn run() -> anyhow::Result<()> {
    let mut args = std::env::args_os().skip(1);
    let first = args.next().context(
        "usage: omniphony_listen.exe <file.wav>\n       omniphony_listen.exe --render-only <file.wav>",
    )?;

    let (render_only, source) = if first == "--render-only" {
        (
            true,
            PathBuf::from(args.next().context("--render-only requires a WAV path")?),
        )
    } else {
        (false, PathBuf::from(first))
    };

    if args.next().is_some() {
        bail!("expected exactly one WAV path");
    }

    let rendered = render_file(&source)?;
    if render_only {
        println!(
            "packaged file-render validation complete: {} stereo frames",
            rendered.samples.len() / 2
        );
        return Ok(());
    }

    let host = cpal::default_host();
    let device = host
        .default_output_device()
        .context("Windows has no default output device")?;

    println!("playing Omniphony render over native WASAPI...");
    play_stereo_f32(&device, rendered.samples, rendered.sample_rate_hz)?;
    println!("Omniphony file playback complete");
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
        require_file(&bundle.bridge, "reference WAV bridge")?;
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
#[derive(Clone, Copy)]
struct WavInfo {
    channels: u16,
    sample_rate_hz: u32,
}

#[cfg(target_os = "windows")]
fn parse_wav_info(bytes: &[u8]) -> anyhow::Result<WavInfo> {
    if bytes.len() < 12 || &bytes[0..4] != b"RIFF" || &bytes[8..12] != b"WAVE" {
        bail!("input is not a RIFF/WAVE file");
    }

    let mut offset = 12usize;
    while offset + 8 <= bytes.len() {
        let chunk_id = &bytes[offset..offset + 4];
        let chunk_size = u32::from_le_bytes(
            bytes[offset + 4..offset + 8]
                .try_into()
                .expect("four-byte chunk size"),
        ) as usize;
        let data_start = offset + 8;
        let data_end = data_start
            .checked_add(chunk_size)
            .context("WAV chunk size overflow")?;
        if data_end > bytes.len() {
            bail!("truncated WAV chunk");
        }

        if chunk_id == b"fmt " {
            if chunk_size < 8 {
                bail!("WAV fmt chunk is too short");
            }
            let channels = u16::from_le_bytes(
                bytes[data_start + 2..data_start + 4]
                    .try_into()
                    .expect("two-byte channel count"),
            );
            let sample_rate_hz = u32::from_le_bytes(
                bytes[data_start + 4..data_start + 8]
                    .try_into()
                    .expect("four-byte sample rate"),
            );
            if channels == 0 || sample_rate_hz == 0 {
                bail!("WAV declares an invalid channel count or sample rate");
            }
            return Ok(WavInfo {
                channels,
                sample_rate_hz,
            });
        }

        offset = data_end + (chunk_size & 1);
    }

    bail!("WAV has no fmt chunk")
}

#[cfg(target_os = "windows")]
struct RenderedFile {
    samples: Vec<f32>,
    sample_rate_hz: u32,
}

#[cfg(target_os = "windows")]
fn render_file(source: &Path) -> anyhow::Result<RenderedFile> {
    const OUTPUT_CHANNELS: u32 = 2;
    const INPUT_CHUNK_BYTES: usize = 64 * 1024;

    require_file(source, "input WAV")?;
    let bundle = Bundle::beside_executable()?;
    let input = std::fs::read(source)
        .with_context(|| format!("failed to read {}", source.display()))?;
    let info = parse_wav_info(&input)?;

    println!("Omniphony for Headphones file listening prototype");
    println!("  source: {}", source.display());
    println!("  input: {} channel(s) @ {} Hz", info.channels, info.sample_rate_hz);
    println!("  config: {}", bundle.config.display());
    println!("  layout: {}", bundle.layout.display());
    println!("  mode: channel content forced through protected binaural render path");
    println!("  note: this prototype renders the complete file before playback");

    let mut engine = Engine::from_paths(
        Some(&bundle.config),
        Some(&bundle.layout),
        Some(&bundle.bridge),
        None,
        info.sample_rate_hz,
    )
    .context("failed to construct Omniphony file renderer")?;

    // The embedded/mpv path is allowed to hand ordinary channel content back to
    // its host. This standalone listener has no second renderer to fall back to,
    // so explicitly spatialize channel beds through Omniphony.
    engine.set_channel_render_mode_code(1);

    if engine.channel_count() != OUTPUT_CHANNELS {
        bail!(
            "protected binaural configuration expected stereo output but engine reports {} channels",
            engine.channel_count()
        );
    }

    let mut rendered = Vec::<f32>::new();
    for chunk in input.chunks(INPUT_CHUNK_BYTES) {
        let blocks = engine
            .process(chunk, RInputTransport::Raw, 0)
            .context("WAV bridge/Omniphony renderer failed")?;
        for block in blocks {
            if block.n_channels != OUTPUT_CHANNELS {
                bail!("renderer changed output width to {} channels", block.n_channels);
            }
            rendered.extend_from_slice(&block.samples);
        }
    }

    validate_render(&rendered, info.sample_rate_hz)?;
    Ok(RenderedFile {
        samples: rendered,
        sample_rate_hz: info.sample_rate_hz,
    })
}

#[cfg(target_os = "windows")]
fn validate_render(samples: &[f32], sample_rate_hz: u32) -> anyhow::Result<()> {
    if samples.is_empty() {
        bail!("Omniphony produced no audio");
    }
    if samples.len() % 2 != 0 {
        bail!("Omniphony produced a partial stereo frame");
    }

    let mut peak = 0.0f32;
    let mut square_sum = 0.0f64;
    for &sample in samples {
        if !sample.is_finite() {
            bail!("Omniphony produced a non-finite sample");
        }
        peak = peak.max(sample.abs());
        square_sum += f64::from(sample) * f64::from(sample);
    }
    if peak <= 1.0e-7 {
        bail!("Omniphony produced only silence");
    }

    let frames = samples.len() / 2;
    let rms = (square_sum / samples.len() as f64).sqrt();
    println!(
        "  rendered: {:.2}s stereo @ {} Hz | peak {:.6} | rms {:.6}",
        frames as f64 / f64::from(sample_rate_hz),
        sample_rate_hz,
        peak,
        rms
    );
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
    fn new(sample_rate_hz: u32) -> anyhow::Result<Self> {
        let config = OmniphonyRealtimeConfig {
            sample_rate_hz,
            channels: 2,
        };
        let ptr = unsafe { omniphony_realtime_create(&config) };
        if ptr.is_null() {
            bail!("realtime PCM seam rejected {sample_rate_hz} Hz stereo");
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
            bail!("realtime PCM seam returned error {rc}");
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
    println!("  output: {device_name}");
    println!(
        "  stream: {} Hz / {}ch / {sample_format:?}",
        config.sample_rate.0, config.channels
    );
    println!("  realtime seam: omniphony_realtime identity");

    match sample_format {
        cpal::SampleFormat::I8 => run_typed_output::<i8>(device, &config, rendered),
        cpal::SampleFormat::I16 => run_typed_output::<i16>(device, &config, rendered),
        cpal::SampleFormat::I32 => run_typed_output::<i32>(device, &config, rendered),
        cpal::SampleFormat::I64 => run_typed_output::<i64>(device, &config, rendered),
        cpal::SampleFormat::U8 => run_typed_output::<u8>(device, &config, rendered),
        cpal::SampleFormat::U16 => run_typed_output::<u16>(device, &config, rendered),
        cpal::SampleFormat::U32 => run_typed_output::<u32>(device, &config, rendered),
        cpal::SampleFormat::U64 => run_typed_output::<u64>(device, &config, rendered),
        cpal::SampleFormat::F32 => run_typed_output::<f32>(device, &config, rendered),
        cpal::SampleFormat::F64 => run_typed_output::<f64>(device, &config, rendered),
        other => bail!("unsupported WASAPI sample format: {other:?}"),
    }
}

#[cfg(target_os = "windows")]
fn run_typed_output<T>(
    device: &cpal::Device,
    config: &cpal::StreamConfig,
    rendered: Vec<f32>,
) -> anyhow::Result<()>
where
    T: cpal::Sample + cpal::SizedSample + cpal::FromSample<f32>,
{
    let device_channels = usize::from(config.channels);
    if device_channels < 2 {
        bail!("Omniphony playback requires at least two output channels");
    }
    if rendered.len() % 2 != 0 {
        bail!("playback received a partial stereo frame");
    }

    let total_frames = rendered.len() / 2;
    let expected_duration =
        Duration::from_secs_f64(total_frames as f64 / f64::from(config.sample_rate.0));
    let done = Arc::new(AtomicBool::new(false));
    let done_for_callback = Arc::clone(&done);
    let mut cursor = 0usize;
    let mut scratch = Vec::<f32>::new();
    let mut processor = RealtimeProcessorHandle::new(config.sample_rate.0)?;
    let mut reported_processing_error = false;

    let err_fn = |err| eprintln!("WASAPI output stream error: {err}");
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
                        eprintln!("realtime seam failed; silencing output: {err:#}");
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
        .context("failed to create WASAPI stream for Omniphony file playback")?;

    stream
        .play()
        .context("failed to start Omniphony WASAPI playback")?;

    let deadline = Instant::now() + expected_duration + Duration::from_secs(5);
    while !done.load(Ordering::Acquire) && Instant::now() < deadline {
        std::thread::sleep(Duration::from_millis(20));
    }
    if !done.load(Ordering::Acquire) {
        bail!("WASAPI playback did not complete before safety deadline");
    }
    std::thread::sleep(Duration::from_millis(100));
    drop(stream);
    Ok(())
}
