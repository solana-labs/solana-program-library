use std::{fs::canonicalize, process::Command};

fn main() {
    println!("cargo:warning=(not a warning) Building SPL Token shared object");
    Command::new(canonicalize("../../do.sh").unwrap())
        .current_dir("../..")
        .arg("build")
        .arg("token/program")
        .status()
        .expect("Failed to build token program")
        .success();
}
