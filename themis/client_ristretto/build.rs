use std::process::{exit, Command};

fn main() {
    if std::env::var("XARGO").is_err() {
    println!("cargo:warning=(not a warning) Building BPF themis program");
    if !Command::new("cargo")
            .args(&[
                "build-bpf",
                "--manifest-path",
                "../program_ristretto/Cargo.toml",
            ])
            .status()
            .expect("Failed to build BPF themis program")
            .success() {
            exit(1);
        }
    }
}
