use std::process::Command;

fn main() {
    println!("cargo:warning=(not a warning) Building BPF themis program");
    Command::new("cargo")
        .args(&[
            "build-bpf",
            "--manifest-path",
            "../program_ristretto/Cargo.toml",
        ])
        .status()
        .expect("Failed to build BPF themis program")
        .success();
}
