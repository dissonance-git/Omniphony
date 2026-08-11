//! Minimal PCM realtime ABI for native Omniphony hosts.
//!
//! This is intentionally **not** a second renderer. It is the narrow seam an
//! endpoint APO, virtual-endpoint host, plugin or other native transport can
//! call after the operating system has already decoded audio to PCM.
//!
//! The first implementation is strict identity. That gives the host boundary a
//! deterministic regression oracle before the protected Omniphony binaural path
//! is connected behind it.

use std::ptr;

const ABI_MAJOR: u32 = 0;
const ABI_MINOR: u32 = 1;

#[repr(C)]
pub struct OmniphonyRealtimeConfig {
    pub sample_rate_hz: u32,
    pub channels: u32,
}

pub struct OmniphonyRealtimeProcessor {
    sample_rate_hz: u32,
    channels: u32,
}

#[unsafe(no_mangle)]
pub extern "C" fn omniphony_realtime_abi_major() -> u32 {
    ABI_MAJOR
}

#[unsafe(no_mangle)]
pub extern "C" fn omniphony_realtime_abi_minor() -> u32 {
    ABI_MINOR
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_realtime_create(
    config: *const OmniphonyRealtimeConfig,
) -> *mut OmniphonyRealtimeProcessor {
    if config.is_null() {
        return ptr::null_mut();
    }

    // SAFETY: null was rejected above; caller owns the config for this call.
    let config = unsafe { &*config };
    if config.sample_rate_hz == 0 || config.channels == 0 {
        return ptr::null_mut();
    }

    Box::into_raw(Box::new(OmniphonyRealtimeProcessor {
        sample_rate_hz: config.sample_rate_hz,
        channels: config.channels,
    }))
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_realtime_destroy(processor: *mut OmniphonyRealtimeProcessor) {
    if !processor.is_null() {
        // SAFETY: the ABI requires a pointer returned by create, exactly once.
        unsafe { drop(Box::from_raw(processor)) };
    }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_realtime_reset(
    processor: *mut OmniphonyRealtimeProcessor,
) -> i32 {
    if processor.is_null() {
        return -1;
    }

    // Identity currently has no history to clear. Keep the symbol now because
    // the protected binaural renderer does have stream-lifetime DSP state.
    0
}

/// Process interleaved f32 PCM.
///
/// Returns 0 on success and a negative value for invalid input. `input` and
/// `output` may be the same pointer. The frame count is supplied per callback;
/// audible behavior must not depend on callback partition size once real DSP is
/// connected behind this boundary.
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

    // SAFETY: null was rejected above; the caller must keep this processor alive
    // for the duration of the call.
    let processor = unsafe { &*processor };
    let Some(samples) = frames.checked_mul(processor.channels as usize) else {
        return -3;
    };

    // `ptr::copy` deliberately permits identical/overlapping buffers, which is
    // required for in-place APO-style processing. The initial contract is exact
    // identity, so this is the entire processing path until the protected
    // Omniphony renderer is connected here.
    unsafe { ptr::copy(input, output, samples) };
    0
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_realtime_sample_rate_hz(
    processor: *const OmniphonyRealtimeProcessor,
) -> u32 {
    if processor.is_null() {
        return 0;
    }
    // SAFETY: null was rejected above.
    unsafe { (*processor).sample_rate_hz }
}

#[unsafe(no_mangle)]
pub unsafe extern "C" fn omniphony_realtime_channels(
    processor: *const OmniphonyRealtimeProcessor,
) -> u32 {
    if processor.is_null() {
        return 0;
    }
    // SAFETY: null was rejected above.
    unsafe { (*processor).channels }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn config() -> OmniphonyRealtimeConfig {
        OmniphonyRealtimeConfig {
            sample_rate_hz: 48_000,
            channels: 2,
        }
    }

    #[test]
    fn rejects_invalid_configuration() {
        let bad_rate = OmniphonyRealtimeConfig {
            sample_rate_hz: 0,
            channels: 2,
        };
        let bad_channels = OmniphonyRealtimeConfig {
            sample_rate_hz: 48_000,
            channels: 0,
        };

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
            assert_eq!(
                omniphony_realtime_process_f32(processor, input.as_ptr(), output.as_mut_ptr(), 4,),
                0
            );
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
            assert_eq!(
                omniphony_realtime_process_f32(
                    processor,
                    samples.as_ptr(),
                    samples.as_mut_ptr(),
                    4,
                ),
                0
            );
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
            assert_eq!(
                omniphony_realtime_process_f32(
                    processor,
                    std::ptr::null(),
                    std::ptr::null_mut(),
                    0,
                ),
                0
            );
            omniphony_realtime_destroy(processor);
        }
    }
}
