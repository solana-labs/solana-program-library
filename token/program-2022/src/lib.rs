#![allow(clippy::arithmetic_side_effects)]
#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

//! An ERC20-like Token program for the Solana blockchain

pub mod error;
pub mod extension;
pub mod generic_token_account;
pub mod instruction;
pub mod native_mint;
pub mod offchain;
pub mod onchain;
pub mod processor;
pub mod proof;
#[cfg(feature = "serde-traits")]
pub mod serialization;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

// Export current sdk types for downstream users building with a different sdk
// version
use solana_program::{
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_memory::sol_memcmp,
    pubkey::{Pubkey, PUBKEY_BYTES},
    system_program,
};
pub use {solana_program, solana_zk_token_sdk};

/// Convert the UI representation of a token amount (using the decimals field
/// defined in its mint) to the raw amount
pub fn ui_amount_to_amount(ui_amount: f64, decimals: u8) -> u64 {
    (ui_amount * 10_usize.pow(decimals as u32) as f64) as u64
}

/// Convert a raw amount to its UI representation (using the decimals field
/// defined in its mint)
pub fn amount_to_ui_amount(amount: u64, decimals: u8) -> f64 {
    amount as f64 / 10_usize.pow(decimals as u32) as f64
}

/// Convert a raw amount to its UI representation (using the decimals field
/// defined in its mint)
pub fn amount_to_ui_amount_string(amount: u64, decimals: u8) -> String {
    let decimals = decimals as usize;
    if decimals > 0 {
        // Left-pad zeros to decimals + 1, so we at least have an integer zero
        let mut s = format!("{:01$}", amount, decimals + 1);
        // Add the decimal point (Sorry, "," locales!)
        s.insert(s.len() - decimals, '.');
        s
    } else {
        amount.to_string()
    }
}

/// Convert a raw amount to its UI representation using the given decimals field
/// Excess zeroes or unneeded decimal point are trimmed.
pub fn amount_to_ui_amount_string_trimmed(amount: u64, decimals: u8) -> String {
    let mut s = amount_to_ui_amount_string(amount, decimals);
    if decimals > 0 {
        let zeros_trimmed = s.trim_end_matches('0');
        s = zeros_trimmed.trim_end_matches('.').to_string();
    }
    s
}

/// Try to convert a UI representation of a token amount to its raw amount using
/// the given decimals field
pub fn try_ui_amount_into_amount(ui_amount: String, decimals: u8) -> Result<u64, ProgramError> {
    let decimals = decimals as usize;
    let mut parts = ui_amount.split('.');
    // splitting a string, even an empty one, will always yield an iterator of at
    // least length == 1
    let mut amount_str = parts.next().unwrap().to_string();
    let after_decimal = parts.next().unwrap_or("");
    let after_decimal = after_decimal.trim_end_matches('0');
    if (amount_str.is_empty() && after_decimal.is_empty())
        || parts.next().is_some()
        || after_decimal.len() > decimals
    {
        return Err(ProgramError::InvalidArgument);
    }

    amount_str.push_str(after_decimal);
    for _ in 0..decimals.saturating_sub(after_decimal.len()) {
        amount_str.push('0');
    }
    amount_str
        .parse::<u64>()
        .map_err(|_| ProgramError::InvalidArgument)
}

solana_program::declare_id!("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");

/// Checks that the supplied program ID is correct for spl-token-2022
pub fn check_program_account(spl_token_program_id: &Pubkey) -> ProgramResult {
    if spl_token_program_id != &id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Checks that the supplied program ID is corect for spl-token or
/// spl-token-2022
pub fn check_spl_token_program_account(spl_token_program_id: &Pubkey) -> ProgramResult {
    if spl_token_program_id != &id() && spl_token_program_id != &spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Checks that the supplied program ID is correct for the ZK Token proof
/// program
pub fn check_zk_token_proof_program_account(zk_token_proof_program_id: &Pubkey) -> ProgramResult {
    if zk_token_proof_program_id != &solana_zk_token_sdk::zk_token_proof_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Checks if the spplied program ID is that of the system program
pub fn check_system_program_account(system_program_id: &Pubkey) -> ProgramResult {
    if system_program_id != &system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Checks two pubkeys for equality in a computationally cheap way using
/// `sol_memcmp`
pub fn cmp_pubkeys(a: &Pubkey, b: &Pubkey) -> bool {
    sol_memcmp(a.as_ref(), b.as_ref(), PUBKEY_BYTES) == 0
}
