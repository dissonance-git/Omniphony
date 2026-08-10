#[cfg(target_os = "windows")]
mod capture;

#[cfg(target_os = "windows")]
use anyhow::{bail, Context};
#[cfg(target_os = "windows")]
use cpal::traits::{DeviceTrait, HostTrait, StreamTrait};
#[cfg(target_os = "windows")]
use omniphony_realtime::{
    omniphony_realtime_create, omniphony_realtime_destroy, omniphony_realtime_process_f32,
    OmniphonyRealtimeConfig, OmniphonyRealtimeProcessor,
};
#[cfg(target_os = "windows")]
use std::time::Duration;

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
    let host = cpal::default_host();

    println!("Omniphony for Headphones native Windows audio host");
    println!("backend: WASAPI (CPAL default Windows host)");
    println!("mode: native transport prototype; protected renderer integration follows");

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

    let args: Vec<_> = std::env::args_os().collect();

    if args.iter().any(|arg| arg == "--probe-loopback") {
        println!("probing self-excluding system loopback capture...");
        capture::probe_self_excluding_system_loopback()?;
    } else {
        println!("loopback capture: not activated (pass --probe-loopback to test it)");
    }

    if args.iter().any(|arg| arg == "--smoke-output") {
        let device = default_output.context("Windows has no default output device")?;
        run_output_smoke(&device)?;
    } else {
        println!("output smoke: not activated (pass --smoke-output to hear the native path)");
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
            omniphony_realtime_process_f32(self.ptr, samples.as_ptr(), samples.as_mut_ptr(), frames)
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
        cpal::SampleFormat::I8 => run_typed_output::<i8>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::I16 => run_typed_output::<i16>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::I32 => run_typed_output::<i32>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::I64 => run_typed_output::<i64>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::U8 => run_typed_output::<u8>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::U16 => run_typed_output::<u16>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::U32 => run_typed_output::<u32>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::U64 => run_typed_output::<u64>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::F32 => run_typed_output::<f32>(device, &config, SMOKE_DURATION),
        cpal::SampleFormat::F64 => run_typed_output::<f64>(device, &config, SMOKE_DURATION),
        other => bail!("unsupported default WASAPI sample format: {other:?}"),
    }
}

#[cfg(target_os = "windows")]
fn run_typed_output<T>(
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
