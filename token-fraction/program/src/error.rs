//! Error types

use {
    num_derive::FromPrimitive,
    solana_program::{
        decode_error::DecodeError,
        msg,
        program_error::{PrintProgramError, ProgramError},
    },
    thiserror::Error,
};

/// Errors that may be returned by the Fraction program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum FractionError {
    /// Invalid instruction data passed in.
    #[error("Failed to unpack instruction data")]
    InstructionUnpackError,

    /// Lamport balance below rent-exempt threshold.
    #[error("Lamport balance below rent-exempt threshold")]
    NotRentExempt,

    /// Already initialized
    #[error("Already initialized")]
    AlreadyInitialized,

    /// Uninitialized
    #[error("Uninitialized")]
    Uninitialized,

    /// Account does not have correct owner
    #[error("Account does not have correct owner")]
    IncorrectOwner,

    /// NumericalOverflowError
    #[error("NumericalOverflowError")]
    NumericalOverflowError,

    /// Provided token account contains no tokens
    #[error("Provided token account contains no tokens")]
    TokenAccountContainsNoTokens,

    /// Provided token account cannot provide amount specified
    #[error("Provided token account cannot provide amount specified")]
    TokenAccountAmountLessThanAmountSpecified,

    /// Provided vault account contains is not empty
    #[error("Provided vault account contains is not empty")]
    VaultAccountIsNotEmpty,

    /// Provided vault account is not owned by program
    #[error("Provided vault account is not owned by program")]
    VaultAccountIsNotOwnedByProgram,

    /// The provided registry account address does not match the expected program derived address
    #[error(
        "The provided registry account address does not match the expected program derived address"
    )]
    RegistryAccountAddressInvalid,

    /// Token transfer failed
    #[error("Token transfer failed")]
    TokenTransferFailed,
    /// Token mint to failed
    #[error("Token mint to failed")]
    TokenMintToFailed,
    /// Token burn failed
    #[error("Token burn failed")]
    TokenBurnFailed,

    /// Fraction mint not empty on int
    #[error("Fraction mint not empty on init")]
    FractionMintNotEmpty,

    /// Fraction mint's authority not set to program
    #[error("Fraction mint's authority not set to program")]
    FractionAuthorityNotProgram,

    /// Fraction treasury not empty on init
    #[error("Fraction treasury not empty on init")]
    TreasuryNotEmpty,

    /// Fraction treasury's owner not set to program
    #[error("Fraction treasury's owner not set to program")]
    TreasuryOwnerNotProgram,

    /// Pool should be inactive
    #[error("Pool should be inactive")]
    PoolShouldBeInactive,

    /// Pool should be active
    #[error("Pool should be active")]
    PoolShouldBeActive,

    /// Fraction treasury needs to match fraction mint
    #[error("Fraction treasury needs to match fraction mint")]
    FractionTreasuryMintDoesNotMatchFractionMint,

    /// Redeem Treasury cannot be same mint as fraction
    #[error("Redeem Treasury cannot be same mint as fraction")]
    RedeemTreasuryCantShareSameMintAsFraction,

    /// Invalid program authority provided
    #[error("Invalid program authority provided")]
    InvalidAuthority,

    /// Redeem treasury mint must match lookup mint
    #[error("Redeem treasury mint must match lookup mint")]
    RedeemTreasuryMintMustMatchLookupMint,

    /// You must pay with the same mint as the external pricing oracle
    #[error("You must pay with the same mint as the external pricing oracle")]
    PaymentMintShouldMatchPricingMint,

    /// Your share account should match the mint of the fractional mint
    #[error("Your share account should match the mint of the fractional mint")]
    ShareMintShouldMatchFractionalMint,

    /// Fraction mint provided does not match that on the token pool
    #[error("Fraction mint provided does not match that on the token pool")]
    FractionMintNeedsToMatchPool,

    /// Redeem treasury provided does not match that on the token pool
    #[error("Redeem treasury provided does not match that on the token pool")]
    RedeemTreasuryNeedsToMatchPool,

    /// Not allowed to combine at this time
    #[error("Not allowed to combine at this time")]
    NotAllowedToCombine,

    /// You cannot afford to combine this pool
    #[error("You cannot afford to combine this pool")]
    CannotAffordToCombineThisPool,
}

impl PrintProgramError for FractionError {
    fn print<E>(&self) {
        msg!(&self.to_string());
    }
}

impl From<FractionError> for ProgramError {
    fn from(e: FractionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for FractionError {
    fn type_of() -> &'static str {
        "Fraction Error"
    }
}
