#[cfg(target_os = "windows")]
mod capture;

#[cfg(target_os = "windows")]
use cpal::traits::{DeviceTrait, HostTrait};

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

    println!("Omniphony native Windows audio host");
    println!("backend: WASAPI (CPAL default Windows host)");
    println!("mode: transport probe only; renderer integration follows in a separate step");

    match host.default_output_device() {
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

    if std::env::args_os().any(|arg| arg == "--probe-loopback") {
        println!("probing self-excluding system loopback capture...");
        capture::probe_self_excluding_system_loopback()?;
    } else {
        println!("loopback capture: not activated (pass --probe-loopback to test it)");
    }

    Ok(())
}
