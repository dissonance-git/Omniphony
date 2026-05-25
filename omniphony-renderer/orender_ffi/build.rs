//! Generate the C header (include/orender.h) from the FFI surface via cbindgen.

use std::path::PathBuf;

fn main() {
    let crate_dir = std::env::var("CARGO_MANIFEST_DIR").expect("CARGO_MANIFEST_DIR not set");
    let out = PathBuf::from(&crate_dir).join("include/orender.h");

    println!("cargo:rerun-if-changed=src/lib.rs");
    println!("cargo:rerun-if-changed=cbindgen.toml");

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
