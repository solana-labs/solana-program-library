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
            .arg("build-bpf")
            .status()
            .expect("Failed to build BPF shared-memory program")
            .success()
        {
            exit(1);
        }
    }
}
