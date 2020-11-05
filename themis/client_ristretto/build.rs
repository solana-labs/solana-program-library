use std::process::{exit, Command};

fn main() {
    if std::env::var("XARGO").is_err()
        && std::env::var("RUSTC_WRAPPER").is_err()
        && std::env::var("RUSTC_WORKSPACE_WRAPPER").is_err()
    {
        println!(
            "cargo:warning=(not a warning) Building BPF {} program",
            std::env::var("CARGO_PKG_NAME").unwrap()
        );
        if !Command::new("cargo")
            .args(&[
                "build-bpf",
                "--manifest-path",
                "../program_ristretto/Cargo.toml",
            ])
            .status()
            .expect("Failed to build BPF themis program")
            .success()
        {
            exit(1);
        }
    }
}
