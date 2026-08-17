//! Realtime-safe host ABI for authored multichannel beds.
//!
//! The APO callback only moves PCM through preallocated SPSC rings. The Current
//! renderer, personal output calibration and file-backed listening preferences
//! all execute on the dedicated worker thread.

#[path = "../../realtime_ffi/src/noire_x_profile.rs"]
mod noire_x_profile;

use noire_x_profile::NoireXPersonalEq;
use orender_engine::current_authored_bed::CurrentAuthoredBedRenderer;
use std::cell::UnsafeCell;
use std::collections::VecDeque;
use std::ptr;
use std::sync::atomic::{AtomicBool, AtomicU64, AtomicUsize, Ordering};
use std::sync::{Arc, mpsc::sync_channel};
use std::thread::{self, JoinHandle};
use std::time::Duration;

const ABI_MAJOR: u32 = 0;
const ABI_MINOR: u32 = 1;
const PROCESS_BLOCK_MS: usize = 20;
const HOST_LATENCY_MS: usize = 40;
const RING_SECONDS: usize = 2;
const OUTPUT_MAKEUP_GAIN: f32 = 1.380_384_3;
const OUTPUT_CEILING: f32 = 0.891_250_9;
const OUTPUT_LOOKAHEAD_MS: usize = 5;
const OUTPUT_RELEASE_MS: f32 = 160.0;
const SAFETY_DOWNMIX_GAIN: f32 = 0.65;

#[repr(C)]
pub struct OmniphonyBedConfig {
    pub sample_rate_hz: u32,
    pub input_channels: u32,
}

struct AudioRing {
    cells: Box<[UnsafeCell<f32>]>,
    capacity: usize,
    read: AtomicUsize,
    write: AtomicUsize,
}

unsafe impl Send for AudioRing {}
unsafe impl Sync for AudioRing {}

impl AudioRing {
    fn new(capacity: usize) -> Self {
        let capacity = capacity.max(2);
        Self {
            cells: (0..capacity)
                .map(|_| UnsafeCell::new(0.0f32))
                .collect::<Vec<_>>()
                .into_boxed_slice(),
            capacity,
            read: AtomicUsize::new(0),
            write: AtomicUsize::new(0),
        }
    }

    fn available(&self) -> usize {
        self.write
            .load(Ordering::Acquire)
            .wrapping_sub(self.read.load(Ordering::Acquire))
            .min(self.capacity)
    }

    fn free(&self) -> usize {
        self.capacity.saturating_sub(self.available())
    }

    unsafe fn push_ptr(&self, input: *const f32, count: usize) -> bool {
        if count > self.free() {
            return false;
        }
        let write = self.write.load(Ordering::Relaxed);
        for i in 0..count {
            let slot = write.wrapping_add(i) % self.capacity;
            unsafe { *self.cells[slot].get() = *input.add(i) };
        }
        self.write.store(write.wrapping_add(count), Ordering::Release);
        true
    }

    fn push_slice(&self, input: &[f32]) -> bool {
        unsafe { self.push_ptr(input.as_ptr(), input.len()) }
    }

    unsafe fn pop_ptr(&self, output: *mut f32, count: usize) -> usize {
        let read = self.read.load(Ordering::Relaxed);
        let take = count.min(self.available());
        for i in 0..take {
            let slot = read.wrapping_add(i) % self.capacity;
            unsafe { *output.add(i) = *self.cells[slot].get() };
        }
        self.read.store(read.wrapping_add(take), Ordering::Release);
        take
    }

    fn pop_slice(&self, output: &mut [f32]) -> usize {
        unsafe { self.pop_ptr(output.as_mut_ptr(), output.len()) }
    }

    fn discard(&self, count: usize) -> usize {
        let read = self.read.load(Ordering::Relaxed);
        let take = count.min(self.available());
        self.read.store(read.wrapping_add(take), Ordering::Release);
        take
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
        Self {
            frames: VecDeque::new(),
            peaks: VecDeque::new(),
            next_frame_index: 0,
            gain: 1.0,
            release_coeff,
            lookahead_frames: sample_rate_hz as usize * OUTPUT_LOOKAHEAD_MS / 1000,
        }
    }

    fn process_interleaved(&mut self, input: &[f32]) -> Vec<f32> {
        let mut out = Vec::with_capacity(input.len());
        for frame in input.chunks_exact(2) {
            let queued = [finite(frame[0]) * OUTPUT_MAKEUP_GAIN, finite(frame[1]) * OUTPUT_MAKEUP_GAIN];
            let peak = queued[0].abs().max(queued[1].abs());
            let index = self.next_frame_index;
            self.next_frame_index = self.next_frame_index.saturating_add(1);

            while self.peaks.back().is_some_and(|&(_, p)| p < peak) {
                self.peaks.pop_back();
            }
            self.peaks.push_back((index, peak));
            self.frames.push_back(queued);
            if self.frames.len() <= self.lookahead_frames {
                continue;
            }

            let oldest = index - self.lookahead_frames as u64;
            while self.peaks.front().is_some_and(|&(i, _)| i < oldest) {
                self.peaks.pop_front();
            }
            let (peak_index, future_peak) = self.peaks.front().copied().unwrap_or((oldest, 0.0));
            let distance = (peak_index - oldest) as usize;
            let target = if future_peak > OUTPUT_CEILING {
                OUTPUT_CEILING / future_peak
            } else {
                1.0
            };
            if target < self.gain {
                if distance == 0 {
                    self.gain = target;
                } else {
                    self.gain += (target - self.gain) / distance as f32;
                }
            } else {
                self.gain = target - (target - self.gain) * self.release_coeff;
            }

            let current = self.frames.pop_front().unwrap();
            let current_peak = current[0].abs().max(current[1].abs());
            let immediate = if current_peak > OUTPUT_CEILING {
                OUTPUT_CEILING / current_peak
            } else {
                1.0
            };
            let applied = self.gain.min(immediate).clamp(0.0, 1.0);
            self.gain = self.gain.min(applied);
            out.push(current[0] * applied);
            out.push(current[1] * applied);
        }
        out
    }
}

struct StereoDelay {
    frames: Box<[[f32; 2]]>,
    offset: usize,
    filled: usize,
}

impl StereoDelay {
    fn new(sample_rate_hz: u32) -> Self {
        let count = (sample_rate_hz as usize * HOST_LATENCY_MS / 1000).max(1);
        Self {
            frames: vec![[0.0, 0.0]; count].into_boxed_slice(),
            offset: 0,
            filled: 0,
        }
    }

    fn len(&self) -> usize {
        self.frames.len()
    }

    fn push(&mut self, input: [f32; 2]) -> ([f32; 2], bool) {
        let delayed = self.frames[self.offset];
        self.frames[self.offset] = input;
        self.offset += 1;
        if self.offset == self.frames.len() {
            self.offset = 0;
        }
        let primed = self.filled >= self.frames.len();
        if self.filled < self.frames.len() {
            self.filled += 1;
        }
        (delayed, primed)
    }
}

pub struct OmniphonyBedProcessor {
    input_channels: usize,
    input: Arc<AudioRing>,
    output: Arc<AudioRing>,
    stop: Arc<AtomicBool>,
    failed: Arc<AtomicBool>,
    processed_blocks: Arc<AtomicU64>,
    worker: Option<JoinHandle<()>>,
    safety_delay: StereoDelay,
    missed_frames: usize,
}

impl OmniphonyBedProcessor {
    fn new(sample_rate_hz: u32, input_channels: usize) -> Result<Self, String> {
        if sample_rate_hz == 0 || !CurrentAuthoredBedRenderer::supports_channels(input_channels) {
            return Err("unsupported authored-bed configuration".to_string());
        }
        let input_capacity = sample_rate_hz as usize * input_channels * RING_SECONDS;
        let output_capacity = sample_rate_hz as usize * 2 * RING_SECONDS;
        let input = Arc::new(AudioRing::new(input_capacity));
        let output = Arc::new(AudioRing::new(output_capacity));
        let stop = Arc::new(AtomicBool::new(false));
        let failed = Arc::new(AtomicBool::new(false));
        let processed_blocks = Arc::new(AtomicU64::new(0));
        let (init_tx, init_rx) = sync_channel::<Result<(), String>>(1);

        let input_worker = Arc::clone(&input);
        let output_worker = Arc::clone(&output);
        let stop_worker = Arc::clone(&stop);
        let failed_worker = Arc::clone(&failed);
        let blocks_worker = Arc::clone(&processed_blocks);
        let frames_per_block = (sample_rate_hz as usize * PROCESS_BLOCK_MS / 1000).max(64);
        let input_samples = frames_per_block * input_channels;

        let worker = thread::Builder::new()
            .name("omniphony-authored-bed".to_string())
            .spawn(move || {
                let mut renderer = match CurrentAuthoredBedRenderer::new(sample_rate_hz, input_channels) {
                    Ok(renderer) => renderer,
                    Err(error) => {
                        let _ = init_tx.send(Err(error.to_string()));
                        failed_worker.store(true, Ordering::Release);
                        return;
                    }
                };
                let mut eq = NoireXPersonalEq::new(sample_rate_hz);
                let mut guard = StereoLookaheadPeakGuard::new(sample_rate_hz);
                let mut block = vec![0.0f32; input_samples];
                let _ = init_tx.send(Ok(()));

                while !stop_worker.load(Ordering::Acquire) {
                    if input_worker.available() < input_samples {
                        thread::sleep(Duration::from_micros(250));
                        continue;
                    }
                    if input_worker.pop_slice(&mut block) != input_samples {
                        continue;
                    }
                    let mut rendered = match renderer.process(&block) {
                        Ok(rendered) => rendered,
                        Err(_) => {
                            failed_worker.store(true, Ordering::Release);
                            return;
                        }
                    };
                    eq.process_interleaved(&mut rendered);
                    let rendered = guard.process_interleaved(&rendered);
                    blocks_worker.fetch_add(1, Ordering::Relaxed);
                    if rendered.is_empty() {
                        continue;
                    }
                    while !stop_worker.load(Ordering::Acquire) {
                        if output_worker.free() >= rendered.len() && output_worker.push_slice(&rendered) {
                            break;
                        }
                        thread::sleep(Duration::from_micros(250));
                    }
                }
            })
            .map_err(|error| error.to_string())?;

        match init_rx.recv_timeout(Duration::from_secs(30)) {
            Ok(Ok(())) => Ok(Self {
                input_channels,
                input,
                output,
                stop,
                failed,
                processed_blocks,
                worker: Some(worker),
                safety_delay: StereoDelay::new(sample_rate_hz),
                missed_frames: 0,
            }),
            Ok(Err(error)) => {
                stop.store(true, Ordering::Release);
                let _ = worker.join();
                Err(error)
            }
            Err(error) => {
                stop.store(true, Ordering::Release);
                let _ = worker.join();
                Err(format!("authored-bed initialization timed out: {error}"))
            }
        }
    }

    unsafe fn process(&mut self, input: *const f32, output: *mut f32, frames: usize) -> i32 {
        if frames == 0 {
            return 0;
        }
        if input.is_null() || output.is_null() {
            return -2;
        }
        let Some(input_samples) = frames.checked_mul(self.input_channels) else {
            return -3;
        };

        let mut use_current = !self.failed.load(Ordering::Acquire);
        if use_current && !unsafe { self.input.push_ptr(input, input_samples) } {
            self.failed.store(true, Ordering::Release);
            use_current = false;
        }

        for frame in 0..frames {
            let source = unsafe {
                std::slice::from_raw_parts(input.add(frame * self.input_channels), self.input_channels)
            };
            let (dry, primed) = self.safety_delay.push(safety_downmix(source));
            let mut rendered = if primed { dry } else { [0.0, 0.0] };

            if primed && use_current && !self.failed.load(Ordering::Acquire) {
                while self.missed_frames > 0 && self.output.available() >= 2 {
                    if self.output.discard(2) != 2 {
                        break;
                    }
                    self.missed_frames -= 1;
                }
                if self.missed_frames == 0 && self.output.available() >= 2 {
                    let mut current = [0.0f32; 2];
                    if self.output.pop_slice(&mut current) == 2 {
                        rendered = current;
                    } else {
                        self.missed_frames = self.missed_frames.saturating_add(1);
                    }
                } else {
                    self.missed_frames = self.missed_frames.saturating_add(1);
                }
            }

            rendered[0] = finite(rendered[0]);
            rendered[1] = finite(rendered[1]);
            let peak = rendered[0].abs().max(rendered[1].abs());
            if peak > OUTPUT_CEILING {
                let gain = OUTPUT_CEILING / peak;
                rendered[0] *= gain;
                rendered[1] *= gain;
            }
            unsafe {
                *output.add(frame * 2) = rendered[0];
                *output.add(frame * 2 + 1) = rendered[1];
            }
        }
        0
    }
}

impl Drop for OmniphonyBedProcessor {
    fn drop(&mut self) {
        self.stop.store(true, Ordering::Release);
        if let Some(worker) = self.worker.take() {
            let _ = worker.join();
        }
    }
}

fn safety_downmix(frame: &[f32]) -> [f32; 2] {
    let at = |index: usize| frame.get(index).copied().map(finite).unwrap_or(0.0);
    let mut left = at(0);
    let mut right = at(1);
    let center = at(2) * 0.707_106_77;
    let lfe = at(3) * 0.5;
    left += center + lfe;
    right += center + lfe;
    left += at(4) * 0.5;
    right += at(5) * 0.5;
    if frame.len() >= 8 {
        left += at(6) * 0.5;
        right += at(7) * 0.5;
    }
    match frame.len() {
        12 | 16 => {
            left += (at(8) + at(10)) * 0.35;
            right += (at(9) + at(11)) * 0.35;
        }
        17 => {
            let back_center = at(8) * 0.35;
            left += back_center + (at(9) + at(11)) * 0.35;
            right += back_center + (at(10) + at(12)) * 0.35;
        }
        _ => {}
    }
    if frame.len() == 16 {
        left += (at(12) + at(14)) * 0.25;
        right += (at(13) + at(15)) * 0.25;
    } else if frame.len() == 17 {
        left += (at(13) + at(15)) * 0.25;
        right += (at(14) + at(16)) * 0.25;
    }
    [left * SAFETY_DOWNMIX_GAIN, right * SAFETY_DOWNMIX_GAIN]
}

#[inline]
fn finite(sample: f32) -> f32 {
    if sample.is_finite() { sample } else { 0.0 }
}

#[unsafe(no_mangle)]
pub extern "C" fn omniphony_bed_abi_major() -> u32 { ABI_MAJOR }

#[unsafe(no_mangle)]
pub extern "C" fn omniphony_bed_abi_minor() -> u32 { ABI_MINOR }

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_bed_create(
    config: *const OmniphonyBedConfig,
) -> *mut OmniphonyBedProcessor {
    if config.is_null() {
        return ptr::null_mut();
    }
    let config = unsafe { &*config };
    match OmniphonyBedProcessor::new(config.sample_rate_hz, config.input_channels as usize) {
        Ok(processor) => Box::into_raw(Box::new(processor)),
        Err(_) => ptr::null_mut(),
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_bed_destroy(processor: *mut OmniphonyBedProcessor) {
    if !processor.is_null() {
        unsafe { drop(Box::from_raw(processor)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_bed_process_f32(
    processor: *mut OmniphonyBedProcessor,
    input: *const f32,
    output: *mut f32,
    frames: usize,
) -> i32 {
    if processor.is_null() {
        return -1;
    }
    unsafe { (&mut *processor).process(input, output, frames) }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_bed_latency_frames(
    processor: *const OmniphonyBedProcessor,
) -> usize {
    if processor.is_null() {
        0
    } else {
        unsafe { (*processor).safety_delay.len() }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_bed_processed_blocks(
    processor: *const OmniphonyBedProcessor,
) -> u64 {
    if processor.is_null() {
        0
    } else {
        unsafe { (*processor).processed_blocks.load(Ordering::Relaxed) }
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_bed_input_channels(
    processor: *const OmniphonyBedProcessor,
) -> u32 {
    if processor.is_null() {
        0
    } else {
        unsafe { (*processor).input_channels as u32 }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn safety_downmix_preserves_left_right_identity_for_quiet_7_1() {
        let frame = [0.25f32, -0.25, 0.0, 0.0, 0.0, 0.0, 0.0, 0.0];
        let out = safety_downmix(&frame);
        assert!(out[0] > 0.0);
        assert!(out[1] < 0.0);
        assert!((out[0].abs() - out[1].abs()).abs() < 1.0e-6);
    }

    #[test]
    fn invalid_width_is_rejected() {
        assert!(OmniphonyBedProcessor::new(48_000, 7).is_err());
    }

    #[test]
    fn fixed_latency_is_40ms_at_48k() {
        let delay = StereoDelay::new(48_000);
        assert_eq!(delay.len(), 1_920);
    }
}
