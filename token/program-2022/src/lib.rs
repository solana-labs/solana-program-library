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
pub mod pod;
pub mod pod_instruction;
pub mod processor;
#[cfg(feature = "serde-traits")]
pub mod serialization;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

// Export current sdk types for downstream users building with a different sdk
// version
use {
    error::TokenError,
    solana_program::{
        entrypoint::ProgramResult, program_error::ProgramError, pubkey::Pubkey, system_program,
    },
    solana_zk_sdk::encryption::pod::elgamal::PodElGamalCiphertext,
};
pub use {solana_program, solana_zk_sdk};

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
    let s = amount_to_ui_amount_string(amount, decimals);
    trim_ui_amount_string(s, decimals)
}

/// Trims a string number by removing excess zeroes or unneeded decimal point
fn trim_ui_amount_string(mut ui_amount: String, decimals: u8) -> String {
    if decimals > 0 {
        let zeros_trimmed = ui_amount.trim_end_matches('0');
        ui_amount = zeros_trimmed.trim_end_matches('.').to_string();
    }
    ui_amount
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

/// Checks that the supplied program ID is correct for spl-token or
/// spl-token-2022
pub fn check_spl_token_program_account(spl_token_program_id: &Pubkey) -> ProgramResult {
    if spl_token_program_id != &id() && spl_token_program_id != &spl_token::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Checks that the supplied program ID is correct for the ZK ElGamal proof
/// program
pub fn check_zk_elgamal_proof_program_account(
    zk_elgamal_proof_program_id: &Pubkey,
) -> ProgramResult {
    if zk_elgamal_proof_program_id != &solana_zk_sdk::zk_elgamal_proof_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Checks if the supplied program ID is that of the system program
pub fn check_system_program_account(system_program_id: &Pubkey) -> ProgramResult {
    if system_program_id != &system_program::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Checks if the supplied program ID is that of the ElGamal registry program
pub(crate) fn check_elgamal_registry_program_account(
    elgamal_registry_account_program_id: &Pubkey,
) -> ProgramResult {
    if elgamal_registry_account_program_id != &spl_elgamal_registry::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    Ok(())
}

/// Check instruction data and proof data auditor ciphertext consistency
#[cfg(feature = "zk-ops")]
pub(crate) fn check_auditor_ciphertext(
    instruction_data_auditor_ciphertext_lo: &PodElGamalCiphertext,
    instruction_data_auditor_ciphertext_hi: &PodElGamalCiphertext,
    proof_context_auditor_ciphertext_lo: &PodElGamalCiphertext,
    proof_context_auditor_ciphertext_hi: &PodElGamalCiphertext,
) -> ProgramResult {
    if instruction_data_auditor_ciphertext_lo != proof_context_auditor_ciphertext_lo {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }
    if instruction_data_auditor_ciphertext_hi != proof_context_auditor_ciphertext_hi {
        return Err(TokenError::ConfidentialTransferBalanceMismatch.into());
    }
    Ok(())
}
