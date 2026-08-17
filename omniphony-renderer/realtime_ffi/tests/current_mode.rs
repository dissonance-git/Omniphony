use omniphony_realtime::{
    OmniphonyRealtimeConfig, omniphony_realtime_create, omniphony_realtime_destroy,
    omniphony_realtime_process_f32, omniphony_realtime_processed_blocks,
    omniphony_realtime_set_mode,
};
use std::thread;
use std::time::{Duration, Instant};

const MODE_IDENTITY: u32 = 0;
const MODE_CURRENT: u32 = 1;

#[test]
fn current_worker_processes_and_can_be_recreated_in_one_process() {
    let config = OmniphonyRealtimeConfig {
        sample_rate_hz: 48_000,
        channels: 2,
    };

    unsafe {
        let processor = omniphony_realtime_create(&config);
        assert!(!processor.is_null());

        assert_eq!(omniphony_realtime_set_mode(processor, MODE_CURRENT), 0);

        // Exactly one Current worker block (20 ms at 48 kHz). The callback ABI
        // may initially return silence while the worker catches up; this test is
        // about successful bounded handoff and worker progress, not sound tuning.
        let input = vec![0.0f32; 960 * 2];
        let mut output = vec![f32::NAN; input.len()];
        assert_eq!(
            omniphony_realtime_process_f32(
                processor,
                input.as_ptr(),
                output.as_mut_ptr(),
                960,
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

        // This is the lifecycle the eventual tray control will exercise. It
        // specifically guards against one-shot global bridge registration.
        assert_eq!(omniphony_realtime_set_mode(processor, MODE_IDENTITY), 0);
        assert_eq!(omniphony_realtime_set_mode(processor, MODE_CURRENT), 0);
        assert_eq!(omniphony_realtime_set_mode(processor, MODE_IDENTITY), 0);

        omniphony_realtime_destroy(processor);
    }
}
