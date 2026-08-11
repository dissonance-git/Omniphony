#[cfg(target_os = "windows")]
#[path = "../music_worker_evidence.rs"]
mod music_worker;

#[cfg(target_os = "windows")]
mod realtime_priority {
    use std::ffi::c_void;

    #[link(name = "Avrt")]
    unsafe extern "system" {
        fn AvSetMmThreadCharacteristicsW(task_name: *const u16, task_index: *mut u32)
        -> *mut c_void;
        fn AvRevertMmThreadCharacteristics(handle: *mut c_void) -> i32;
    }

    pub struct MmcssGuard(*mut c_void);

    impl Drop for MmcssGuard {
        fn drop(&mut self) {
            if !self.0.is_null() {
                unsafe {
                    let _ = AvRevertMmThreadCharacteristics(self.0);
                }
            }
        }
    }

    fn claim(task: &str) -> Option<MmcssGuard> {
        let mut wide: Vec<u16> = task.encode_utf16().collect();
        wide.push(0);
        let mut task_index = 0u32;
        let handle = unsafe { AvSetMmThreadCharacteristicsW(wide.as_ptr(), &mut task_index) };
        (!handle.is_null()).then_some(MmcssGuard(handle))
    }

    /// Protect the worker's capture/process/queue producer from background CPU
    /// contention. Prefer the high-priority Pro Audio MMCSS task and fall back
    /// to the more widely applicable Audio task if the former is unavailable.
    pub fn claim_realtime_audio() -> Option<MmcssGuard> {
        claim("Pro Audio").or_else(|| claim("Audio"))
    }
}

fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use anyhow::Context;

        // Process-loopback activation requires an MTA. Claim it before CPAL or
        // any other Windows audio API touches COM on this thread.
        wasapi::initialize_mta()
            .ok()
            .context("failed to initialize COM MTA before Windows audio startup")?;

        // This thread owns loopback capture, music DSP and playback-queue
        // production. Under heavy background compute, ordinary scheduling can
        // starve it long enough for the realtime playback callback to underrun.
        // MMCSS gives the audio producer prioritized CPU access without changing
        // any DSP behavior. The guard reverts the registration on shutdown.
        let _mmcss = realtime_priority::claim_realtime_audio();

        return music_worker::run();
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("omniphony_worker is only available on Windows");
    }
}
