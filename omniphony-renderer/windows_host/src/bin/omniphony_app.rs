#![cfg_attr(target_os = "windows", windows_subsystem = "windows")]

#[cfg(target_os = "windows")]
#[path = "../supervisor.rs"]
mod supervisor;

fn main() -> anyhow::Result<()> {
    #[cfg(target_os = "windows")]
    {
        return supervisor::run();
    }

    #[cfg(not(target_os = "windows"))]
    {
        anyhow::bail!("Omniphony.exe is only available on Windows");
    }
}
