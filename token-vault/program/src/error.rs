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

/// Errors that may be returned by the Vault program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum VaultError {
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

    /// Vault mint not empty on int
    #[error("Vault mint not empty on init")]
    VaultMintNotEmpty,

    /// Vault mint's authority not set to program
    #[error("Vault mint's authority not set to program")]
    VaultAuthorityNotProgram,

    /// Vault treasury not empty on init
    #[error("Vault treasury not empty on init")]
    TreasuryNotEmpty,

    /// Vault treasury's owner not set to program
    #[error("Vault treasury's owner not set to program")]
    TreasuryOwnerNotProgram,

    /// Vault should be inactive
    #[error("Vault should be inactive")]
    VaultShouldBeInactive,

    /// Vault should be active
    #[error("Pool should be active")]
    VaultShouldBeActive,

    /// Vault should be combined
    #[error("Pool should be combined")]
    VaultShouldBeCombined,

    /// Vault treasury needs to match fraction mint
    #[error("Vault treasury needs to match fraction mint")]
    VaultTreasuryMintDoesNotMatchVaultMint,

    /// Redeem Treasury cannot be same mint as fraction
    #[error("Redeem Treasury cannot be same mint as fraction")]
    RedeemTreasuryCantShareSameMintAsVault,

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

    /// Vault mint provided does not match that on the token vault
    #[error("Vault mint provided does not match that on the token vault")]
    VaultMintNeedsToMatchVault,

    /// Redeem treasury provided does not match that on the token vault
    #[error("Redeem treasury provided does not match that on the token vault")]
    RedeemTreasuryNeedsToMatchVault,

    /// Not allowed to combine at this time
    #[error("Not allowed to combine at this time")]
    NotAllowedToCombine,

    /// You cannot afford to combine this pool
    #[error("You cannot afford to combine this vault")]
    CannotAffordToCombineThisVault,

    /// You have no shares to redeem
    #[error("You have no shares to redeem")]
    NoShares,

    /// Your outstanding share account is the incorrect mint
    #[error("Your outstanding share account is the incorrect mint")]
    OutstandingShareAccountNeedsToMatchFractionalMint,

    /// Your destination account is the incorrect mint
    #[error("Your destination account is the incorrect mint")]
    DestinationAccountNeedsToMatchRedeemMint,

    /// Fractional mint is empty
    #[error("Fractional mint is empty")]
    FractionSupplyEmpty,

    /// Token Program Provided Needs To Match Vault
    #[error("Token Program Provided Needs To Match Vault")]
    TokenProgramProvidedDoesNotMatchVault,
}

impl PrintProgramError for VaultError {
    fn print<E>(&self) {
        msg!(&self.to_string());
    }
}

impl From<VaultError> for ProgramError {
    fn from(e: VaultError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for VaultError {
    fn type_of() -> &'static str {
        "Vault Error"
    }
}
