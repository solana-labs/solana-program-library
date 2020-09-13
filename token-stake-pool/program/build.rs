extern crate cbindgen;

use std::env;

fn main() {
    println!("cargo:rerun-if-env-changed=SPL_CBINDGEN");
    println!("cargo:rerun-if-changed=inc/token-gov.h");
    if std::path::Path::new("inc/token-gov.h").exists() && env::var("SPL_CBINDGEN").is_err() {
        return;
    }

    println!("cargo:warning=Generating inc/token-gov.h");
    let crate_dir = env::var("CARGO_MANIFEST_DIR").unwrap();
    cbindgen::generate(&crate_dir)
        .unwrap()
        .write_to_file("inc/token-gov.h");
}
