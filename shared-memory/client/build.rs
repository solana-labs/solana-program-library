use std::{fs::canonicalize, process::Command};

fn main() {
    println!("cargo:warning=(not a warning) Building SPL Shared-memory shared object");
    Command::new(canonicalize("../../do.sh").unwrap())
        .current_dir("../..")
        .arg("build")
        .arg("shared-memory/program")
        .status()
        .expect("Failed to build shared-memory program")
        .success();
}
