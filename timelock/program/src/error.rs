//! Error types

use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// Errors that may be returned by the Timelock program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum GovernanceError {
    /// Invalid instruction data passed in.
    #[error("Failed to unpack instruction data")]
    InstructionUnpackError,

    /// The account cannot be initialized because it is already in use.
    #[error("Account is already initialized")]
    AlreadyInitialized,

    /// Too high position in txn array
    #[error("Too high a position given in txn array")]
    TooHighPositionInTxnArrayError,

    /// Invalid program derived address from a Governance account
    #[error("Invalid PDA given for a Governance program account")]
    InvalidTimelockAuthority,

    /// Timelock Transaction not found on the Timelock Set
    #[error("Timelock Transaction not found on the Timelock Set")]
    TimelockTransactionNotFoundError,

    /// Mint authority can't be deserialized
    #[error("Mint authority can't be deserialized")]
    MintAuthorityUnpackError,

    /// Wrong mint authority was provided for mint
    #[error("Wrong mint authority was provided for mint")]
    InvalidMintAuthorityError,

    /// Invalid mint owner program"
    #[error("Invalid mint owner program")]
    InvalidMintOwnerProgramError,

    /// Invalid account owner
    #[error("Invalid account owner")]
    InvalidAccountOwnerError,

    /// The wrong signatory mint was given for this Proposal
    #[error("The wrong signatory mint was given for this Proposal")]
    InvalidSignatoryMintError,

    /// The Proposal is in the wrong state for this operation
    #[error("The Proposal is in the wrong state for this operation")]
    InvalidProposalStateError,

    /// The account is uninitialized
    #[error("The account is uninitialized when it should have already been initialized")]
    Uninitialized,

    /// Lamport balance below rent-exempt threshold.
    #[error("Lamport balance below rent-exempt threshold")]
    NotRentExempt,

    /// Expected a different SPL Token program
    #[error("Input token program account is not valid")]
    InvalidTokenProgram,

    /// Expected an SPL Token mint
    #[error("Input token mint account is not valid")]
    InvalidTokenMint,

    /// Token initialize mint failed
    #[error("Token initialize mint failed")]
    TokenInitializeMintFailed,
    /// Token initialize account failed
    #[error("Token initialize account failed")]
    TokenInitializeAccountFailed,
    /// Token transfer failed
    #[error("Token transfer failed")]
    TokenTransferFailed,
    /// Token mint to failed
    #[error("Token mint to failed")]
    TokenMintToFailed,
    /// Token burn failed
    #[error("Token burn failed")]
    TokenBurnFailed,

    ///Timelock Transaction already executed
    #[error("Timelock Transaction already executed")]
    TimelockTransactionAlreadyExecuted,

    ///Timelock Transaction execution failed
    #[error("Timelock Transaction execution failed")]
    ExecutionFailed,

    ///Invalid instruction end index, above instruction limit
    #[error("Invalid instruction end index, above instruction limit")]
    InvalidInstructionEndIndex,

    /// Too early to execute this transaction
    #[error("Too early to execute this transaction")]
    TooEarlyToExecute,

    /// Too many accounts in your arbitrary instruction
    #[error("Too many accounts in your arbitrary instruction")]
    TooManyAccountsInInstruction,

    /// You have provided an account that doesnt match the pubkey on a Proposal or config object
    #[error(
        "You have provided an account that doesnt match the pubkey on a Proposal or config object"
    )]
    AccountsShouldMatch,

    /// Provided wrong mint type for a token holding account on Proposal
    #[error("Provided wrong mint type for a token holding account on Proposal")]
    MintsShouldMatch,

    /// Provided source mint decimals don't match voting mint decimals
    #[error("Provided source mint decimals don't match voting mint decimals")]
    MintsDecimalsShouldMatch,

    /// Waiting period must be greater than or equal to minimum waiting period
    #[error("Waiting period must be greater than or equal to minimum waiting period")]
    MustBeAboveMinimumWaitingPeriod,

    /// Invalid Governance key given for a program-mint tuple
    #[error("Invalid Governance config key given for a program-mint tuple")]
    InvalidGovernanceKey,

    /// Cannot reimburse more tokens than you put in
    #[error("Cannot reimburse more tokens than you put in")]
    TokenAmountAboveGivenAmount,

    /// Numerical overflow
    #[error("Numerical overflow")]
    NumericalOverflow,

    /// Invalid Governance Record Key, must use program account id, proposal key, and voting account as tuple seed
    #[error("Invalid Governance Record Key, must use program account id, proposal key, and voting account as tuple seed")]
    InvalidGovernanceVotingRecord,
}

impl PrintProgramError for GovernanceError {
    fn print<E>(&self) {
        msg!(&self.to_string());
    }
}

impl From<GovernanceError> for ProgramError {
    fn from(e: GovernanceError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for GovernanceError {
    fn type_of() -> &'static str {
        "Timelock Error"
    }
}
