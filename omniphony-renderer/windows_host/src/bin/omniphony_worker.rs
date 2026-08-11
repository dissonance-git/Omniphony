#[cfg(target_os = "windows")]
#[path = "../music_worker_evidence.rs"]
mod music_worker;

fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use anyhow::Context;

        // Process-loopback activation requires an MTA. Claim it before CPAL or
        // any other Windows audio API touches COM on this thread.
        wasapi::initialize_mta()
            .ok()
            .context("failed to initialize COM MTA before Windows audio startup")?;
        return music_worker::run();
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("omniphony_worker is only available on Windows");
    }
}
