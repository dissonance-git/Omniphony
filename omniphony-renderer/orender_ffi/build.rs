//! Generate the C header (include/orender.h) from the FFI surface via cbindgen.

use std::path::PathBuf;

fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let out = PathBuf::from(&crate_dir).join("include/orender.h");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");

    // On Linux, stamp the release cdylib with a SemVer soname (`liborender.so.0`)
    // so the packaged library participates in normal shared-object versioning:
    // consumers (mpv) record `liborender.so.0` as DT_NEEDED, resolved at runtime
    // via the symlinks the PKGBUILD installs. Debug builds skip this so the C
    // smoke test can link and run against the bare `target/debug/liborender.so`
    // without needing the versioned symlinks.
    let target_os = std::env::var("CARGO_CFG_TARGET_OS").unwrap_or_default();
    let profile = std::env::var("PROFILE").unwrap_or_default();
    if target_os == "linux" && profile == "release" {
        println!("cargo:rustc-cdylib-link-arg=-Wl,-soname,liborender.so.0");
    }
    // On macOS, stamp the release dylib with an `@rpath` install name so a
    // bundled consumer (mpv, Studio) resolves `liborender.dylib` via its own
    // rpath / `@loader_path` rather than the absolute build-tree path that the
    // linker would otherwise record. Debug builds keep the default install
    // name so the C smoke test links against `target/debug/liborender.dylib`
    // directly, mirroring the Linux soname handling above.
    if target_os == "macos" && profile == "release" {
        println!("cargo:rustc-cdylib-link-arg=-Wl,-install_name,@rpath/liborender.dylib");
    }

    match cbindgen::Builder::new()
        .with_crate(&crate_dir)
        .with_language(cbindgen::Language::C)
        .with_include_guard("ORENDER_H")
        .with_pragma_once(true)
        .generate()
    {
        Ok(bindings) => {
            bindings.write_to_file(&out);
        }
        // Don't fail the cdylib build if header generation hits a snag during
        // development; surface it as a warning instead.
        Err(e) => {
            println!("cargo:warning=cbindgen failed to generate orender.h: {e}");
        }
    }
}
