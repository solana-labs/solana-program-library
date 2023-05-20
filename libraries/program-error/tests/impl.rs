//! Tests that all macros compile
use spl_program_error::*;

/// Example error
#[spl_program_error]
pub enum ExampleError {
    /// Mint has no mint authority
    #[error("Mint has no mint authority")]
    MintHasNoMintAuthority,
    /// Incorrect mint authority has signed the instruction
    #[error("Incorrect mint authority has signed the instruction")]
    IncorrectMintAuthority,
}

/// Tests that all macros compile
#[test]
fn _test() {
    let _ = ExampleError::MintHasNoMintAuthority;
}
