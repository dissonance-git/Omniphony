use anyhow::Result;
use chrono::TimeZone;
use std::env;
use vergen_gitcl::{Emitter, GitclBuilder};

// Stamp this crate with a git-describe + build timestamp, exactly like the
// `orender` binary crate's root build.rs. `runtime_control` is linked by BOTH
// hosts (the standalone `orender` binary and the `liborender` cdylib embedded
// in mpv), so broadcasting this fingerprint over OSC lets Studio show the
// connected renderer's real build in About — making a liborender-vs-orender
// version skew immediately visible.
fn main() -> Result<()> {
    let gitcl = GitclBuilder::default()
        .describe(true, true, Some("[0-9]*"))
        .build()?;

    let gitcl_res = Emitter::default()
        .idempotent()
        .fail_on_error()
        .add_instructions(&gitcl)
        .and_then(|emitter| emitter.emit());

    if let Err(e) = gitcl_res {
        eprintln!("Warning: Failed to generate git information: {e:?}");
        eprintln!("Using fallback version information");
        println!("cargo:rustc-env=VERGEN_GIT_DESCRIBE=unknown");
    }

    let now = match env::var("SOURCE_DATE_EPOCH") {
        Ok(val) => chrono::Utc.timestamp_opt(val.parse::<i64>()?, 0).unwrap(),
        Err(_) => chrono::Utc::now(),
    };

    println!(
        "cargo:rustc-env=BUILD_TIMESTAMP={}",
        now.format("%Y-%m-%d %H:%M:%S UTC")
    );

    Ok(())
}
