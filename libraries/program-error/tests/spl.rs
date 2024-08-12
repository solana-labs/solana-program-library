//! Tests `#[spl_program_error]`

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
fn test_macros_compile() {
    let _ = ExampleError::MintHasNoMintAuthority;
}

/// Example library error with namespace
#[spl_program_error(hash_error_code_start = 2_056_342_880)]
enum ExampleLibraryError {
    /// This is a very informative error
    #[error("This is a very informative error")]
    VeryInformativeError,
    /// This is a super important error
    #[error("This is a super important error")]
    SuperImportantError,
    /// This is a mega serious error
    #[error("This is a mega serious error")]
    MegaSeriousError,
    /// You are toast
    #[error("You are toast")]
    YouAreToast,
}

/// Tests hashing of error codes into unique `u32` values
#[test]
fn test_library_error_codes() {
    fn get_error_code_check(hash_input: &str) -> u32 {
        let mut nonce: u32 = 0;
        loop {
            let hash = solana_program::hash::hashv(&[hash_input.as_bytes(), &nonce.to_le_bytes()]);
            let mut bytes = [0u8; 4];
            bytes.copy_from_slice(&hash.to_bytes()[13..17]);
            let error_code = u32::from_le_bytes(bytes);
            if error_code >= 10_000 {
                return error_code;
            }
            nonce += 1;
        }
    }

    let first_error_as_u32 = ExampleLibraryError::VeryInformativeError as u32;

    assert_eq!(
        ExampleLibraryError::VeryInformativeError as u32,
        get_error_code_check("spl_program_error:ExampleLibraryError"),
    );
    assert_eq!(
        ExampleLibraryError::SuperImportantError as u32,
        first_error_as_u32 + 1,
    );
    assert_eq!(
        ExampleLibraryError::MegaSeriousError as u32,
        first_error_as_u32 + 2,
    );
    assert_eq!(
        ExampleLibraryError::YouAreToast as u32,
        first_error_as_u32 + 3,
    );
}

/// Example error with solana_program crate set
#[spl_program_error(solana_program = "solana_program")]
enum ExampleSolanaProgramCrateError {
    /// This is a very informative error
    #[error("This is a very informative error")]
    VeryInformativeError,
    /// This is a super important error
    #[error("This is a super important error")]
    SuperImportantError,
}

/// Tests that all macros compile
#[test]
fn test_macros_compile_with_solana_program_crate() {
    let _ = ExampleSolanaProgramCrateError::VeryInformativeError;
}
