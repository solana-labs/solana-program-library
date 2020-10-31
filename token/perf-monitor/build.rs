use std::process::Command;

fn main() {
    println!("cargo:warning=(not a warning) Building BPF token program");
    Command::new("cargo")
        .args(&["build-bpf", "--manifest-path", "../program/Cargo.toml"])
        .status()
        .expect("Failed to build BPF token program")
        .success();
}
