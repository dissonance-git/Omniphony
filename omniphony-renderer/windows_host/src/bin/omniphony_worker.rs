#[cfg(target_os = "windows")]
mod live_impl {
    include!("omniphony_live.rs");

    pub fn entry() -> anyhow::Result<()> {
        main()
    }
}

fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        use anyhow::Context;

        wasapi::initialize_mta()
            .ok()
            .context("failed to initialize COM MTA before Windows audio startup")?;
        return live_impl::entry();
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("omniphony_worker is only available on Windows");
    }
}
