use std::{fs::canonicalize, process::Command};

fn main() {
    println!("cargo:warning=(not a warning) Building SPL Themis shared object");
    Command::new(canonicalize("../../do.sh").unwrap())
        .current_dir("../..")
        .arg("build")
        .arg("themis/program_ristretto")
        .status()
        .expect("Failed to build themis program")
        .success();
}
