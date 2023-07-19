//! Interface error types

use spl_program_error::*;

/// Errors that may be returned by the interface.
#[spl_program_error]
pub enum TokenEditionsError {
    /// Supply is greater than proposed max supply
    #[error("Supply is greater than proposed max supply")]
    SupplyExceedsNewMaxSupply,
    /// Supply is greater than max supply
    #[error("Supply is greater than max supply")]
    SupplyExceedsMaxSupply,
    /// Incorrect mint authority has signed the instruction
    #[error("Incorrect mint authority has signed the instruction")]
    IncorrectMintAuthority,
    /// Incorrect original print update authority has signed the instruction
    #[error("Incorrect original print update authority has signed the instruction")]
    IncorrectUpdateAuthority,
    /// Original print has no update authority
    #[error("Original print has no update authority")]
    ImmutablePrint,
}
