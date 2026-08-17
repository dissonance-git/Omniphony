use omniphony_realtime::{
    OmniphonyRealtimeConfig, omniphony_realtime_create, omniphony_realtime_destroy,
    omniphony_realtime_process_f32, omniphony_realtime_processed_blocks,
    omniphony_realtime_set_mode,
};
use std::thread;
use std::time::{Duration, Instant};

const MODE_IDENTITY: u32 = 0;
const MODE_CURRENT: u32 = 1;
const BLOCK_FRAMES: usize = 960;

#[test]
fn current_worker_round_trips_audio_and_can_be_recreated_in_one_process() {
    let config = OmniphonyRealtimeConfig {
        sample_rate_hz: 48_000,
        channels: 2,
    };

    unsafe {
        let processor = omniphony_realtime_create(&config);
        assert!(!processor.is_null());

        assert_eq!(omniphony_realtime_set_mode(processor, MODE_CURRENT), 0);

        // Exactly one Current worker block (20 ms at 48 kHz). Use a quiet but
        // nonzero stereo signal so the test can distinguish real worker output
        // from the startup-silence fallback without turning this into a sound
        // tuning assertion.
        let mut input = vec![0.0f32; BLOCK_FRAMES * 2];
        for frame in 0..BLOCK_FRAMES {
            input[frame * 2] = 0.05;
            input[frame * 2 + 1] = -0.025;
        }
        let mut output = vec![f32::NAN; input.len()];
        assert_eq!(
            omniphony_realtime_process_f32(
                processor,
                input.as_ptr(),
                output.as_mut_ptr(),
                BLOCK_FRAMES,
            ),
            0
        );
        assert!(output.iter().all(|sample| sample.is_finite()));

        let deadline = Instant::now() + Duration::from_secs(10);
        while omniphony_realtime_processed_blocks(processor) == 0 && Instant::now() < deadline {
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            omniphony_realtime_processed_blocks(processor) > 0,
            "Current worker accepted PCM but never completed a render block"
        );

        // processed_blocks is incremented just before the worker publishes the
        // rendered block, so allow a few callback-sized polls. Each call remains
        // bounded and the two-second input ring keeps this comfortably below its
        // capacity even on a loaded CI runner.
        let zeros = vec![0.0f32; BLOCK_FRAMES * 2];
        let mut rendered_seen = false;
        for _ in 0..20 {
            output.fill(f32::NAN);
            assert_eq!(
                omniphony_realtime_process_f32(
                    processor,
                    zeros.as_ptr(),
                    output.as_mut_ptr(),
                    BLOCK_FRAMES,
                ),
                0
            );
            assert!(output.iter().all(|sample| sample.is_finite()));
            if output.iter().any(|sample| sample.abs() > 1.0e-6) {
                rendered_seen = true;
                break;
            }
            thread::sleep(Duration::from_millis(10));
        }
        assert!(
            rendered_seen,
            "Current worker completed work but no rendered PCM crossed back through the output ring"
        );

        // This is the lifecycle the eventual tray control will exercise. It
        // specifically guards against one-shot global bridge registration.
        assert_eq!(omniphony_realtime_set_mode(processor, MODE_IDENTITY), 0);
        assert_eq!(omniphony_realtime_set_mode(processor, MODE_CURRENT), 0);
        assert_eq!(omniphony_realtime_set_mode(processor, MODE_IDENTITY), 0);

        omniphony_realtime_destroy(processor);
    }
}
