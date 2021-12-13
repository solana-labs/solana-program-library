use lazy_static::lazy_static;
use solana_program_test::find_file;
use std::{process::Command, sync::Mutex};

lazy_static! {
    pub static ref VOTER_WEIGHT_ADDIN_BUILD_GUARD: Mutex::<u8> = Mutex::new(0);
}

pub fn ensure_voter_weight_addin_is_built() {
    if find_file("spl_governance_voter_weight_addin.so").is_none() {
        let _guard = VOTER_WEIGHT_ADDIN_BUILD_GUARD.lock().unwrap();
        if find_file("spl_governance_voter_weight_addin.so").is_none() {
            assert!(Command::new("cargo")
                .args(&[
                    "build-bpf",
                    "--manifest-path",
                    "../voter-weight-addin/program/Cargo.toml",
                ])
                .status()
                .expect("Failed to build voter-weight-addin program")
                .success());
        }
    }
}
