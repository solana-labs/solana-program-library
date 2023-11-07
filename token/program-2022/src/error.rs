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

/// Errors that may be returned by the Token program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum TokenError {
    // 0
    /// Lamport balance below rent-exempt threshold.
    #[error("Lamport balance below rent-exempt threshold")]
    NotRentExempt,
    /// Insufficient funds for the operation requested.
    #[error("Insufficient funds")]
    InsufficientFunds,
    /// Invalid Mint.
    #[error("Invalid Mint")]
    InvalidMint,
    /// Account not associated with this Mint.
    #[error("Account not associated with this Mint")]
    MintMismatch,
    /// Owner does not match.
    #[error("Owner does not match")]
    OwnerMismatch,

    // 5
    /// This token's supply is fixed and new tokens cannot be minted.
    #[error("Fixed supply")]
    FixedSupply,
    /// The account cannot be initialized because it is already being used.
    #[error("Already in use")]
    AlreadyInUse,
    /// Invalid number of provided signers.
    #[error("Invalid number of provided signers")]
    InvalidNumberOfProvidedSigners,
    /// Invalid number of required signers.
    #[error("Invalid number of required signers")]
    InvalidNumberOfRequiredSigners,
    /// State is uninitialized.
    #[error("State is uninitialized")]
    UninitializedState,

    // 10
    /// Instruction does not support native tokens
    #[error("Instruction does not support native tokens")]
    NativeNotSupported,
    /// Non-native account can only be closed if its balance is zero
    #[error("Non-native account can only be closed if its balance is zero")]
    NonNativeHasBalance,
    /// Invalid instruction
    #[error("Invalid instruction")]
    InvalidInstruction,
    /// State is invalid for requested operation.
    #[error("State is invalid for requested operation")]
    InvalidState,
    /// Operation overflowed
    #[error("Operation overflowed")]
    Overflow,

    // 15
    /// Account does not support specified authority type.
    #[error("Account does not support specified authority type")]
    AuthorityTypeNotSupported,
    /// This token mint cannot freeze accounts.
    #[error("This token mint cannot freeze accounts")]
    MintCannotFreeze,
    /// Account is frozen; all account operations will fail
    #[error("Account is frozen")]
    AccountFrozen,
    /// Mint decimals mismatch between the client and mint
    #[error("The provided decimals value different from the Mint decimals")]
    MintDecimalsMismatch,
    /// Instruction does not support non-native tokens
    #[error("Instruction does not support non-native tokens")]
    NonNativeNotSupported,

    // 20
    /// Extension type does not match already existing extensions
    #[error("Extension type does not match already existing extensions")]
    ExtensionTypeMismatch,
    /// Extension does not match the base type provided
    #[error("Extension does not match the base type provided")]
    ExtensionBaseMismatch,
    /// Extension already initialized on this account
    #[error("Extension already initialized on this account")]
    ExtensionAlreadyInitialized,
    /// An account can only be closed if its confidential balance is zero
    #[error("An account can only be closed if its confidential balance is zero")]
    ConfidentialTransferAccountHasBalance,
    /// Account not approved for confidential transfers
    #[error("Account not approved for confidential transfers")]
    ConfidentialTransferAccountNotApproved,

    // 25
    /// Account not accepting deposits or transfers
    #[error("Account not accepting deposits or transfers")]
    ConfidentialTransferDepositsAndTransfersDisabled,
    /// ElGamal public key mismatch
    #[error("ElGamal public key mismatch")]
    ConfidentialTransferElGamalPubkeyMismatch,
    /// Balance mismatch
    #[error("Balance mismatch")]
    ConfidentialTransferBalanceMismatch,
    /// Mint has non-zero supply. Burn all tokens before closing the mint.
    #[error("Mint has non-zero supply. Burn all tokens before closing the mint")]
    MintHasSupply,
    /// No authority exists to perform the desired operation
    #[error("No authority exists to perform the desired operation")]
    NoAuthorityExists,

    // 30
    /// Transfer fee exceeds maximum of 10,000 basis points
    #[error("Transfer fee exceeds maximum of 10,000 basis points")]
    TransferFeeExceedsMaximum,
    /// Mint required for this account to transfer tokens, use
    /// `transfer_checked` or `transfer_checked_with_fee`
    #[error("Mint required for this account to transfer tokens, use `transfer_checked` or `transfer_checked_with_fee`")]
    MintRequiredForTransfer,
    /// Calculated fee does not match expected fee
    #[error("Calculated fee does not match expected fee")]
    FeeMismatch,
    /// Fee parameters associated with confidential transfer zero-knowledge
    /// proofs do not match fee parameters in mint
    #[error(
        "Fee parameters associated with zero-knowledge proofs do not match fee parameters in mint"
    )]
    FeeParametersMismatch,
    /// The owner authority cannot be changed
    #[error("The owner authority cannot be changed")]
    ImmutableOwner,

    // 35
    /// An account can only be closed if its withheld fee balance is zero,
    /// harvest fees to the mint and try again
    #[error("An account can only be closed if its withheld fee balance is zero, harvest fees to the mint and try again")]
    AccountHasWithheldTransferFees,
    /// No memo in previous instruction; required for recipient to receive a
    /// transfer
    #[error("No memo in previous instruction; required for recipient to receive a transfer")]
    NoMemo,
    /// Transfer is disabled for this mint
    #[error("Transfer is disabled for this mint")]
    NonTransferable,
    /// Non-transferable tokens can't be minted to an account without immutable
    /// ownership
    #[error("Non-transferable tokens can't be minted to an account without immutable ownership")]
    NonTransferableNeedsImmutableOwnership,
    /// The total number of `Deposit` and `Transfer` instructions to an account
    /// cannot exceed the associated
    /// `maximum_pending_balance_credit_counter`
    #[error(
        "The total number of `Deposit` and `Transfer` instructions to an account cannot exceed
            the associated `maximum_pending_balance_credit_counter`"
    )]
    MaximumPendingBalanceCreditCounterExceeded,

    // 40
    /// The deposit amount for the confidential extension exceeds the maximum
    /// limit
    #[error("Deposit amount exceeds maximum limit")]
    MaximumDepositAmountExceeded,
    /// CPI Guard cannot be enabled or disabled in CPI
    #[error("CPI Guard cannot be enabled or disabled in CPI")]
    CpiGuardSettingsLocked,
    /// CPI Guard is enabled, and a program attempted to transfer user funds
    /// without using a delegate
    #[error("CPI Guard is enabled, and a program attempted to transfer user funds via CPI without using a delegate")]
    CpiGuardTransferBlocked,
    /// CPI Guard is enabled, and a program attempted to burn user funds without
    /// using a delegate
    #[error(
        "CPI Guard is enabled, and a program attempted to burn user funds via CPI without using a delegate"
    )]
    CpiGuardBurnBlocked,
    /// CPI Guard is enabled, and a program attempted to close an account
    /// without returning lamports to owner
    #[error("CPI Guard is enabled, and a program attempted to close an account via CPI without returning lamports to owner")]
    CpiGuardCloseAccountBlocked,

    // 45
    /// CPI Guard is enabled, and a program attempted to approve a delegate
    #[error("CPI Guard is enabled, and a program attempted to approve a delegate via CPI")]
    CpiGuardApproveBlocked,
    /// CPI Guard is enabled, and a program attempted to add or replace an
    /// authority
    #[error(
        "CPI Guard is enabled, and a program attempted to add or replace an authority via CPI"
    )]
    CpiGuardSetAuthorityBlocked,
    /// Account ownership cannot be changed while CPI Guard is enabled
    #[error("Account ownership cannot be changed while CPI Guard is enabled")]
    CpiGuardOwnerChangeBlocked,
    /// Extension not found in account data
    #[error("Extension not found in account data")]
    ExtensionNotFound,
    /// Account does not accept non-confidential transfers
    #[error("Non-confidential transfers disabled")]
    NonConfidentialTransfersDisabled,

    // 50
    /// An account can only be closed if the confidential withheld fee is zero
    #[error("An account can only be closed if the confidential withheld fee is zero")]
    ConfidentialTransferFeeAccountHasWithheldFee,
    /// A mint or an account is initialized to an invalid combination of
    /// extensions
    #[error("A mint or an account is initialized to an invalid combination of extensions")]
    InvalidExtensionCombination,
    /// Extension allocation with overwrite must use the same length
    #[error("Extension allocation with overwrite must use the same length")]
    InvalidLengthForAlloc,
    /// Failed to decrypt a confidential transfer account
    #[error("Failed to decrypt a confidential transfer account")]
    AccountDecryption,
    /// Failed to generate a zero-knowledge proof needed for a token instruction
    #[error("Failed to generate proof")]
    ProofGeneration,

    // 55
    /// An invalid proof instruction offset was provided
    #[error("An invalid proof instruction offset was provided")]
    InvalidProofInstructionOffset,
    /// Harvest of withheld tokens to mint is disabled
    #[error("Harvest of withheld tokens to mint is disabled")]
    HarvestToMintDisabled,
    /// Split proof context state accounts not supported for instruction
    #[error("Split proof context state accounts not supported for instruction")]
    SplitProofContextStateAccountsNotSupported,
    /// Not enough proof context state accounts provided
    #[error("Not enough proof context state accounts provided")]
    NotEnoughProofContextStateAccounts,
    /// Ciphertext is malformed
    #[error("Ciphertext is malformed")]
    MalformedCiphertext,

    // 60
    /// Ciphertext arithmetic failed
    #[error("Ciphertext arithmetic failed")]
    CiphertextArithmeticFailed,
}
impl From<TokenError> for ProgramError {
    fn from(e: TokenError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for TokenError {
    fn type_of() -> &'static str {
        "TokenError"
    }
}

impl PrintProgramError for TokenError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + num_traits::FromPrimitive,
    {
        match self {
            TokenError::NotRentExempt => msg!("Error: Lamport balance below rent-exempt threshold"),
            TokenError::InsufficientFunds => msg!("Error: insufficient funds"),
            TokenError::InvalidMint => msg!("Error: Invalid Mint"),
            TokenError::MintMismatch => msg!("Error: Account not associated with this Mint"),
            TokenError::OwnerMismatch => msg!("Error: owner does not match"),
            TokenError::FixedSupply => msg!("Error: the total supply of this token is fixed"),
            TokenError::AlreadyInUse => msg!("Error: account or token already in use"),
            TokenError::InvalidNumberOfProvidedSigners => {
                msg!("Error: Invalid number of provided signers")
            }
            TokenError::InvalidNumberOfRequiredSigners => {
                msg!("Error: Invalid number of required signers")
            }
            TokenError::UninitializedState => msg!("Error: State is uninitialized"),
            TokenError::NativeNotSupported => {
                msg!("Error: Instruction does not support native tokens")
            }
            TokenError::NonNativeHasBalance => {
                msg!("Error: Non-native account can only be closed if its balance is zero")
            }
            TokenError::InvalidInstruction => msg!("Error: Invalid instruction"),
            TokenError::InvalidState => msg!("Error: Invalid account state for operation"),
            TokenError::Overflow => msg!("Error: Operation overflowed"),
            TokenError::AuthorityTypeNotSupported => {
                msg!("Error: Account does not support specified authority type")
            }
            TokenError::MintCannotFreeze => msg!("Error: This token mint cannot freeze accounts"),
            TokenError::AccountFrozen => msg!("Error: Account is frozen"),
            TokenError::MintDecimalsMismatch => {
                msg!("Error: decimals different from the Mint decimals")
            }
            TokenError::NonNativeNotSupported => {
                msg!("Error: Instruction does not support non-native tokens")
            }
            TokenError::ExtensionTypeMismatch => {
                msg!("Error: New extension type does not match already existing extensions")
            }
            TokenError::ExtensionBaseMismatch => {
                msg!("Error: Extension does not match the base type provided")
            }
            TokenError::ExtensionAlreadyInitialized => {
                msg!("Error: Extension already initialized on this account")
            }
            TokenError::ConfidentialTransferAccountHasBalance => {
                msg!("Error: An account can only be closed if its confidential balance is zero")
            }
            TokenError::ConfidentialTransferAccountNotApproved => {
                msg!("Error: Account not approved for confidential transfers")
            }
            TokenError::ConfidentialTransferDepositsAndTransfersDisabled => {
                msg!("Error: Account not accepting deposits or transfers")
            }
            TokenError::ConfidentialTransferElGamalPubkeyMismatch => {
                msg!("Error: ElGamal public key mismatch")
            }
            TokenError::ConfidentialTransferBalanceMismatch => {
                msg!("Error: Balance mismatch")
            }
            TokenError::MintHasSupply => {
                msg!("Error: Mint has non-zero supply. Burn all tokens before closing the mint")
            }
            TokenError::NoAuthorityExists => {
                msg!("Error: No authority exists to perform the desired operation");
            }
            TokenError::TransferFeeExceedsMaximum => {
                msg!("Error: Transfer fee exceeds maximum of 10,000 basis points");
            }
            TokenError::MintRequiredForTransfer => {
                msg!("Mint required for this account to transfer tokens, use `transfer_checked` or `transfer_checked_with_fee`");
            }
            TokenError::FeeMismatch => {
                msg!("Calculated fee does not match expected fee");
            }
            TokenError::FeeParametersMismatch => {
                msg!("Fee parameters associated with zero-knowledge proofs do not match fee parameters in mint")
            }
            TokenError::ImmutableOwner => {
                msg!("The owner authority cannot be changed");
            }
            TokenError::AccountHasWithheldTransferFees => {
                msg!("Error: An account can only be closed if its withheld fee balance is zero, harvest fees to the mint and try again");
            }
            TokenError::NoMemo => {
                msg!("Error: No memo in previous instruction; required for recipient to receive a transfer");
            }
            TokenError::NonTransferable => {
                msg!("Transfer is disabled for this mint");
            }
            TokenError::NonTransferableNeedsImmutableOwnership => {
                msg!("Non-transferable tokens can't be minted to an account without immutable ownership");
            }
            TokenError::MaximumPendingBalanceCreditCounterExceeded => {
                msg!("The total number of `Deposit` and `Transfer` instructions to an account cannot exceed the associated `maximum_pending_balance_credit_counter`");
            }
            TokenError::MaximumDepositAmountExceeded => {
                msg!("Deposit amount exceeds maximum limit")
            }
            TokenError::CpiGuardSettingsLocked => {
                msg!("CPI Guard status cannot be changed in CPI")
            }
            TokenError::CpiGuardTransferBlocked => {
                msg!("CPI Guard is enabled, and a program attempted to transfer user funds without using a delegate")
            }
            TokenError::CpiGuardBurnBlocked => {
                msg!("CPI Guard is enabled, and a program attempted to burn user funds without using a delegate")
            }
            TokenError::CpiGuardCloseAccountBlocked => {
                msg!("CPI Guard is enabled, and a program attempted to close an account without returning lamports to owner")
            }
            TokenError::CpiGuardApproveBlocked => {
                msg!("CPI Guard is enabled, and a program attempted to approve a delegate")
            }
            TokenError::CpiGuardSetAuthorityBlocked => {
                msg!("CPI Guard is enabled, and a program attempted to add or change an authority")
            }
            TokenError::CpiGuardOwnerChangeBlocked => {
                msg!("Account ownership cannot be changed while CPI Guard is enabled")
            }
            TokenError::ExtensionNotFound => {
                msg!("Extension not found in account data")
            }
            TokenError::NonConfidentialTransfersDisabled => {
                msg!("Non-confidential transfers disabled")
            }
            TokenError::ConfidentialTransferFeeAccountHasWithheldFee => {
                msg!("Account has non-zero confidential withheld fee")
            }
            TokenError::InvalidExtensionCombination => {
                msg!("Mint or account is initialized to an invalid combination of extensions")
            }
            TokenError::InvalidLengthForAlloc => {
                msg!("Extension allocation with overwrite must use the same length")
            }
            TokenError::AccountDecryption => {
                msg!("Failed to decrypt a confidential transfer account")
            }
            TokenError::ProofGeneration => {
                msg!("Failed to generate proof")
            }
            TokenError::InvalidProofInstructionOffset => {
                msg!("An invalid proof instruction offset was provided")
            }
            TokenError::HarvestToMintDisabled => {
                msg!("Harvest of withheld tokens to mint is disabled")
            }
            TokenError::SplitProofContextStateAccountsNotSupported => {
                msg!("Split proof context state accounts not supported for instruction")
            }
            TokenError::NotEnoughProofContextStateAccounts => {
                msg!("Not enough proof context state accounts provided")
            }
            TokenError::MalformedCiphertext => {
                msg!("Ciphertext is malformed")
            }
            TokenError::CiphertextArithmeticFailed => {
                msg!("Ciphertext arithmetic failed")
            }
        }
    }
}
