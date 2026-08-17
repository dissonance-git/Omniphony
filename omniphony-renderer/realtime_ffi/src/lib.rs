//! Narrow PCM realtime ABI for native Omniphony hosts.
//!
//! Identity remains mode 0 as the deterministic transport oracle. Mode 1 runs
//! the retained stereo Current model on a dedicated worker thread. The host
//! callback only copies PCM into/out of preallocated SPSC rings; the existing
//! allocating renderer never runs on the audio callback thread.

use orender_engine::current_music_support::CurrentMusicSupportRenderer;
use renderer::music_field::{MUSIC_FIELD_CHANNELS, MusicFieldProcessor};
use renderer::music_foundation::MusicFoundationProcessor;
use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::ptr;
use std::sync::{Arc, mpsc::sync_channel};
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const ABI_MAJOR: u32 = 0;
const ABI_MINOR: u32 = 2;
const MODE_IDENTITY: u32 = 0;
const MODE_CURRENT: u32 = 1;
const PROCESS_BLOCK_MS: usize = 20;
const RING_SECONDS: usize = 2;
const FIELD_SUPPORT_GAIN: f32 = 1.0;
const LINEAR_OUTPUT_GAIN: f32 = 0.90;
const OUTPUT_MAKEUP_GAIN: f32 = 1.380_384_3;
const OUTPUT_CEILING: f32 = 0.891_250_9;
const OUTPUT_LOOKAHEAD_MS: usize = 5;
const OUTPUT_RELEASE_MS: f32 = 160.0;

#[repr(C)]
pub struct OmniphonyRealtimeConfig {
    pub sample_rate_hz: u32,
    pub channels: u32,
}

struct AudioRing {
    cells: Box<[UnsafeCell<f32>]>,
    capacity: usize,
    read: AtomicUsize,
    write: AtomicUsize,
}

// Exactly one producer and one consumer touch each ring. Indices publish cell
// ownership with release/acquire ordering before the opposite side reads it.
unsafe impl Send for AudioRing {}
unsafe impl Sync for AudioRing {}

impl AudioRing {
    fn new(capacity: usize) -> Self {
        let capacity = capacity.max(2);
        let cells = (0..capacity)
            .map(|_| UnsafeCell::new(0.0f32))
            .collect::<Vec<_>>()
            .into_boxed_slice();
        Self {
            cells,
            capacity,
            read: AtomicUsize::new(0),
            write: AtomicUsize::new(0),
        }
    }

    fn available(&self) -> usize {
        self.write
            .load(Ordering::Acquire)
            .wrapping_sub(self.read.load(Ordering::Acquire))
    }

    fn free(&self) -> usize {
        self.capacity.saturating_sub(self.available().min(self.capacity))
    }

    unsafe fn push_ptr(&self, input: *const f32, count: usize) -> bool {
        if count > self.free() {
            return false;
        }
        let write = self.write.load(Ordering::Relaxed);
        for i in 0..count {
            let slot = (write.wrapping_add(i)) % self.capacity;
            // SAFETY: producer exclusively owns all unpublished write slots.
            unsafe { *self.cells[slot].get() = *input.add(i) };
        }
        self.write.store(write.wrapping_add(count), Ordering::Release);
        true
    }

    fn push_slice(&self, input: &[f32]) -> bool {
        // SAFETY: the slice is valid for input.len() reads.
        unsafe { self.push_ptr(input.as_ptr(), input.len()) }
    }

    unsafe fn pop_ptr(&self, output: *mut f32, count: usize) -> usize {
        let read = self.read.load(Ordering::Relaxed);
        let available = self
            .write
            .load(Ordering::Acquire)
            .wrapping_sub(read)
            .min(self.capacity);
        let take = count.min(available);
        for i in 0..take {
            let slot = (read.wrapping_add(i)) % self.capacity;
            // SAFETY: consumer exclusively owns all published unread slots.
            unsafe { *output.add(i) = *self.cells[slot].get() };
        }
        self.read.store(read.wrapping_add(take), Ordering::Release);
        take
    }

    fn pop_slice(&self, output: &mut [f32]) -> usize {
        // SAFETY: the slice is valid for output.len() writes.
        unsafe { self.pop_ptr(output.as_mut_ptr(), output.len()) }
    }
}

struct StereoLookaheadPeakGuard {
    frames: VecDeque<[f32; 2]>,
    peaks: VecDeque<(u64, f32)>,
    next_frame_index: u64,
    gain: f32,
    release_coeff: f32,
    lookahead_frames: usize,
}

impl StereoLookaheadPeakGuard {
    fn new(sample_rate_hz: u32) -> Self {
        let release_seconds = OUTPUT_RELEASE_MS / 1000.0;
        let release_coeff = (-1.0 / (release_seconds * sample_rate_hz.max(1) as f32)).exp();
        let lookahead_frames = (sample_rate_hz as usize * OUTPUT_LOOKAHEAD_MS) / 1000;
        Self {
            frames: VecDeque::with_capacity(lookahead_frames + 2),
            peaks: VecDeque::with_capacity(lookahead_frames + 2),
            next_frame_index: 0,
            gain: 1.0,
            release_coeff,
            lookahead_frames,
        }
    }

    fn process_interleaved(&mut self, input: &[f32]) -> Vec<f32> {
        let mut out = Vec::with_capacity(input.len());
        for frame in input.chunks_exact(2) {
            let left = if frame[0].is_finite() { frame[0] } else { 0.0 };
            let right = if frame[1].is_finite() { frame[1] } else { 0.0 };
            let queued = [left * OUTPUT_MAKEUP_GAIN, right * OUTPUT_MAKEUP_GAIN];
            let frame_peak = queued[0].abs().max(queued[1].abs());
            let frame_index = self.next_frame_index;
            self.next_frame_index = self.next_frame_index.saturating_add(1);

            while let Some(&(_, back_peak)) = self.peaks.back() {
                if back_peak >= frame_peak {
                    break;
                }
                self.peaks.pop_back();
            }
            self.peaks.push_back((frame_index, frame_peak));
            self.frames.push_back(queued);

            if self.frames.len() <= self.lookahead_frames {
                continue;
            }

            let oldest_index = frame_index - self.lookahead_frames as u64;
            while self.peaks.front().is_some_and(|&(index, _)| index < oldest_index) {
                self.peaks.pop_front();
            }
            let (peak_frame_index, future_peak) = self.peaks.front().copied().unwrap();
            let peak_index = (peak_frame_index - oldest_index) as usize;
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

            let current = self.frames.pop_front().unwrap();
            let current_peak = current[0].abs().max(current[1].abs());
            let immediate_safe_gain = if current_peak > OUTPUT_CEILING {
                OUTPUT_CEILING / current_peak
            } else {
                1.0
            };
            let applied_gain = self.gain.min(immediate_safe_gain).clamp(0.0, 1.0);
            self.gain = self.gain.min(applied_gain);
            out.push(current[0] * applied_gain);
            out.push(current[1] * applied_gain);
        }
        out
    }
}

struct CurrentPipeline {
    field: MusicFieldProcessor,
    foundation: MusicFoundationProcessor,
    support: CurrentMusicSupportRenderer,
    dry_fifo: VecDeque<f32>,
    foundation_fifo: VecDeque<f32>,
    peak_guard: StereoLookaheadPeakGuard,
}

impl CurrentPipeline {
    fn new(sample_rate_hz: u32) -> Result<Self, String> {
        Ok(Self {
            field: MusicFieldProcessor::new(sample_rate_hz),
            foundation: MusicFoundationProcessor::new(sample_rate_hz),
            support: CurrentMusicSupportRenderer::new(sample_rate_hz)
                .map_err(|error| error.to_string())?,
            dry_fifo: VecDeque::new(),
            foundation_fifo: VecDeque::new(),
            peak_guard: StereoLookaheadPeakGuard::new(sample_rate_hz),
        })
    }

    fn process(&mut self, input: &[f32]) -> Result<Vec<f32>, String> {
        if input.is_empty() || input.len() % 2 != 0 {
            return Err("Current model requires interleaved stereo".to_string());
        }

        self.dry_fifo.extend(input.iter().copied());
        let foundation = self.foundation.process_interleaved_delta(input);
        if foundation.len() != input.len() {
            return Err("foundation width mismatch".to_string());
        }
        self.foundation_fifo.extend(foundation);

        let field = self.field.process_interleaved_stereo(input);
        if field.len() != (input.len() / 2) * MUSIC_FIELD_CHANNELS {
            return Err("field width mismatch".to_string());
        }

        let rendered = self.support.process(&field).map_err(|error| error.to_string())?;
        let mut out = Vec::new();
        for block in rendered {
            if block.n_channels != 2 {
                return Err("support renderer changed output width".to_string());
            }
            if block.samples.is_empty() {
                continue;
            }
            if self.dry_fifo.len() < block.samples.len()
                || self.foundation_fifo.len() < block.samples.len()
            {
                return Err("support renderer outran aligned master".to_string());
            }

            let mut mixed = Vec::with_capacity(block.samples.len());
            for &support in &block.samples {
                let base = self.dry_fifo.pop_front().unwrap();
                let body = self.foundation_fifo.pop_front().unwrap();
                let base = if base.is_finite() { base } else { 0.0 };
                let body = if body.is_finite() { body } else { 0.0 };
                let support = if support.is_finite() { support } else { 0.0 };
                mixed.push((base + body + support * FIELD_SUPPORT_GAIN) * LINEAR_OUTPUT_GAIN);
            }
            out.extend(self.peak_guard.process_interleaved(&mixed));
        }
        Ok(out)
    }
}

struct AsyncCurrent {
    input: Arc<AudioRing>,
    output: Arc<AudioRing>,
    stop: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    processed_blocks: Arc<AtomicU64>,
    worker: Option<JoinHandle<()>>,
}

impl AsyncCurrent {
    fn new(sample_rate_hz: u32) -> Result<Self, String> {
        let capacity_samples = (sample_rate_hz as usize)
            .saturating_mul(2)
            .saturating_mul(RING_SECONDS);
        let input = Arc::new(AudioRing::new(capacity_samples));
        let output = Arc::new(AudioRing::new(capacity_samples));
        let stop = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(AtomicBool::new(false));
        let processed_blocks = Arc::new(AtomicU64::new(0));
        let (init_tx, init_rx) = sync_channel::<Result<(), String>>(1);

        let input_worker = Arc::clone(&input);
        let output_worker = Arc::clone(&output);
        let stop_worker = Arc::clone(&stop);
        let failed_worker = Arc::clone(&failed);
        let blocks_worker = Arc::clone(&processed_blocks);
        let process_frames = ((sample_rate_hz as usize) * PROCESS_BLOCK_MS / 1000).max(64);
        let process_samples = process_frames * 2;

        let worker = thread::Builder::new()
            .name("omniphony-current-model".to_string())
            .spawn(move || {
                let mut pipeline = match CurrentPipeline::new(sample_rate_hz) {
                    Ok(pipeline) => {
                        let _ = init_tx.send(Ok(()));
                        pipeline
                    }
                    Err(error) => {
                        let _ = init_tx.send(Err(error));
                        failed_worker.store(true, Ordering::Release);
                        return;
                    }
                };
                let mut block = vec![0.0f32; process_samples];

                while !stop_worker.load(Ordering::Acquire) {
                    if input_worker.available() < process_samples {
                        thread::sleep(Duration::from_micros(250));
                        continue;
                    }
                    let got = input_worker.pop_slice(&mut block);
                    if got != process_samples {
                        continue;
                    }

                    let rendered = match pipeline.process(&block) {
                        Ok(rendered) => rendered,
                        Err(_) => {
                            failed_worker.store(true, Ordering::Release);
                            return;
                        }
                    };
                    blocks_worker.fetch_add(1, Ordering::Relaxed);

                    if rendered.is_empty() {
                        continue;
                    }
                    while !stop_worker.load(Ordering::Acquire) {
                        if output_worker.free() >= rendered.len() {
                            if output_worker.push_slice(&rendered) {
                                break;
                            }
                        }
                        thread::sleep(Duration::from_micros(250));
                    }
                }
            })
            .map_err(|error| error.to_string())?;

        match init_rx.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(())) => Ok(Self {
                input,
                output,
                stop,
                failed,
                processed_blocks,
                worker: Some(worker),
            }),
            Ok(Err(error)) => {
                stop.store(true, Ordering::Release);
                let _ = worker.join();
                Err(error)
            }
            Err(error) => {
                stop.store(true, Ordering::Release);
                let _ = worker.join();
                Err(format!("Current model initialization timed out: {error}"))
            }
        }
    }

    unsafe fn process_raw(
        &self,
        input: *const f32,
        output: *mut f32,
        samples: usize,
    ) -> i32 {
        if self.failed.load(Ordering::Acquire) {
            return -10;
        }
        // SAFETY: caller validated both pointers for `samples` elements.
        if !unsafe { self.input.push_ptr(input, samples) } {
            return -11;
        }
        // SAFETY: caller validated output for `samples` elements.
        let got = unsafe { self.output.pop_ptr(output, samples) };
        for index in got..samples {
            // Startup/rare worker underrun is silence, never stale memory.
            unsafe { *output.add(index) = 0.0 };
        }
        0
    }
}

impl Drop for AsyncCurrent {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

enum ProcessorMode {
    Identity,
    Current(AsyncCurrent),
}

pub struct OmniphonyRealtimeProcessor {
    sample_rate_hz: u32,
    channels: u32,
    mode: ProcessorMode,
}

#[unsafe(no_mangle)]
pub extern "C" fn omniphony_realtime_abi_major() -> u32 { ABI_MAJOR }

#[unsafe(no_mangle)]
pub extern "C" fn omniphony_realtime_abi_minor() -> u32 { ABI_MINOR }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_realtime_create(
    config: *const OmniphonyRealtimeConfig,
) -> *mut OmniphonyRealtimeProcessor {
    if config.is_null() {
        return ptr::null_mut();
    }
    let config = unsafe { &*config };
    if config.sample_rate_hz == 0 || config.channels == 0 {
        return ptr::null_mut();
    }
    Box::into_raw(Box::new(OmniphonyRealtimeProcessor {
        sample_rate_hz: config.sample_rate_hz,
        channels: config.channels,
        mode: ProcessorMode::Identity,
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_realtime_destroy(processor: *mut OmniphonyRealtimeProcessor) {
    if !processor.is_null() {
        unsafe { drop(Box::from_raw(processor)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_realtime_set_mode(
    processor: *mut OmniphonyRealtimeProcessor,
    mode: u32,
) -> i32 {
    if processor.is_null() {
        return -1;
    }
    let processor = unsafe { &mut *processor };
    match mode {
        MODE_IDENTITY => {
            processor.mode = ProcessorMode::Identity;
            0
        }
        MODE_CURRENT => {
            if processor.channels != 2 {
                return -2;
            }
            match AsyncCurrent::new(processor.sample_rate_hz) {
                Ok(current) => {
                    processor.mode = ProcessorMode::Current(current);
                    0
                }
                Err(_) => -3,
            }
        }
        _ => -4,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_realtime_mode(
    processor: *const OmniphonyRealtimeProcessor,
) -> u32 {
    if processor.is_null() {
        return u32::MAX;
    }
    match unsafe { &(*processor).mode } {
        ProcessorMode::Identity => MODE_IDENTITY,
        ProcessorMode::Current(_) => MODE_CURRENT,
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_realtime_processed_blocks(
    processor: *const OmniphonyRealtimeProcessor,
) -> u64 {
    if processor.is_null() {
        return 0;
    }
    match unsafe { &(*processor).mode } {
        ProcessorMode::Identity => 0,
        ProcessorMode::Current(current) => current.processed_blocks.load(Ordering::Relaxed),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_realtime_reset(
    processor: *mut OmniphonyRealtimeProcessor,
) -> i32 {
    if processor.is_null() { -1 } else { 0 }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_realtime_process_f32(
    processor: *mut OmniphonyRealtimeProcessor,
    input: *const f32,
    output: *mut f32,
    frames: usize,
) -> i32 {
    if processor.is_null() {
        return -1;
    }
    if frames == 0 {
        return 0;
    }
    if input.is_null() || output.is_null() {
        return -2;
    }
    let processor = unsafe { &mut *processor };
    let Some(samples) = frames.checked_mul(processor.channels as usize) else {
        return -3;
    };
    match &processor.mode {
        ProcessorMode::Identity => {
            unsafe { ptr::copy(input, output, samples) };
            0
        }
        ProcessorMode::Current(current) => {
            if processor.channels != 2 {
                -4
            } else {
                unsafe { current.process_raw(input, output, samples) }
            }
        }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_realtime_sample_rate_hz(
    processor: *const OmniphonyRealtimeProcessor,
) -> u32 {
    if processor.is_null() { 0 } else { unsafe { (*processor).sample_rate_hz } }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_realtime_channels(
    processor: *const OmniphonyRealtimeProcessor,
) -> u32 {
    if processor.is_null() { 0 } else { unsafe { (*processor).channels } }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> OmniphonyRealtimeConfig {
        OmniphonyRealtimeConfig { sample_rate_hz: 48_000, channels: 2 }
    }

    #[test]
    fn rejects_invalid_configuration() {
        let bad_rate = OmniphonyRealtimeConfig { sample_rate_hz: 0, channels: 2 };
        let bad_channels = OmniphonyRealtimeConfig { sample_rate_hz: 48_000, channels: 0 };
        unsafe {
            assert!(omniphony_realtime_create(std::ptr::null()).is_null());
            assert!(omniphony_realtime_create(&bad_rate).is_null());
            assert!(omniphony_realtime_create(&bad_channels).is_null());
        }
    }

    #[test]
    fn identity_is_bit_exact_out_of_place() {
        let input = [0.0f32, -0.25, 0.5, 1.0, -1.0, 0.125, -0.75, 0.875];
        let mut output = [f32::NAN; 8];
        let cfg = config();
        unsafe {
            let processor = omniphony_realtime_create(&cfg);
            assert!(!processor.is_null());
            assert_eq!(omniphony_realtime_process_f32(processor, input.as_ptr(), output.as_mut_ptr(), 4), 0);
            omniphony_realtime_destroy(processor);
        }
        for (before, after) in input.iter().zip(output.iter()) {
            assert_eq!(before.to_bits(), after.to_bits());
        }
    }

    #[test]
    fn identity_is_bit_exact_in_place() {
        let mut samples = [0.0f32, -0.25, 0.5, 1.0, -1.0, 0.125, -0.75, 0.875];
        let before = samples.map(f32::to_bits);
        let cfg = config();
        unsafe {
            let processor = omniphony_realtime_create(&cfg);
            assert!(!processor.is_null());
            assert_eq!(omniphony_realtime_process_f32(processor, samples.as_ptr(), samples.as_mut_ptr(), 4), 0);
            omniphony_realtime_destroy(processor);
        }
        assert_eq!(before, samples.map(f32::to_bits));
    }

    #[test]
    fn zero_frames_accepts_null_audio_buffers() {
        let cfg = config();
        unsafe {
            let processor = omniphony_realtime_create(&cfg);
            assert!(!processor.is_null());
            assert_eq!(omniphony_realtime_process_f32(processor, std::ptr::null(), std::ptr::null_mut(), 0), 0);
            omniphony_realtime_destroy(processor);
        }
    }
}
