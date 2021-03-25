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
pub enum TimelockError {
    /// Invalid instruction data passed in.
    #[error("Failed to unpack instruction data")]
    InstructionUnpackError,

    /// The account cannot be initialized because it is already in use.
    #[error("Account is already initialized")]
    AlreadyInitialized,

    /// Using the wrong version of the timelock set for this code version
    #[error("Using a timelock set from a different version than this program version")]
    InvalidTimelockSetVersionError,

    /// Too high position in txn array
    #[error("Too high a position given in txn array")]
    TooHighPositionInTxnArrayError,

    /// Invalid program derived address from a timelock account
    #[error("Invalid PDA given for a timelock program account")]
    InvalidTimelockAuthority,

    /// Using the wrong version of the timelock program for this code version
    #[error("Using a timelock program account from a different version than this program version")]
    InvalidTimelockVersionError,

    /// Timelock Transaction not found on the Timelock Set
    #[error("Timelock Transaction not found on the Timelock Set")]
    TimelockTransactionNotFoundError,

    /// The wrong signatory mint was given for this timelock set
    #[error("The wrong signatory mint was given for this timelock set")]
    InvalidSignatoryMintError,

    /// The timelock set is in the wrong state for this operation
    #[error("The timelock set is in the wrong state for this operation")]
    InvalidTimelockSetStateError,

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

    /// Invalid cursor given for temp file call
    #[error("Invalid cursor given for temp file call")]
    InvalidCursor,

    /// Too many accounts in your arbitrary instruction
    #[error("Too many accounts in your arbitrary instruction")]
    TooManyAccountsInInstruction,

    /// Invalid timelock type for this action
    #[error("Invalid timelock type for this action")]
    InvalidTimelockType,

    /// You have provided an account that doesnt match the pubkey on a timelock set or config object
    #[error("You have provided an account that doesnt match the pubkey on a timelock set or config object")]
    AccountsShouldMatch,

    /// Provided wrong mint type for a token holding account on timelock set
    #[error("Provided wrong mint type for a token holding account on timelock set")]
    MintsShouldMatch,

    /// Waiting period must be greater than or equal to minimum waiting period
    #[error("Waiting period must be greater than or equal to minimum waiting period")]
    MustBeAboveMinimumWaitingPeriod,

    /// Invalid Timelock config key given for a program-mint tuple
    #[error("Invalid timelock config key given for a program-mint tuple")]
    InvalidTimelockConfigKey,

    /// Cannot reimburse more tokens than you put in
    #[error("Cannot reimburse more tokens than you put in")]
    TokenAmountAboveGivenAmount,

    /// Numerical overflow
    #[error("Numerical overflow")]
    NumericalOverflow,
}

impl PrintProgramError for TimelockError {
    fn print<E>(&self) {
        msg!(&self.to_string());
    }
}

impl From<TimelockError> for ProgramError {
    fn from(e: TimelockError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for TimelockError {
    fn type_of() -> &'static str {
        "Timelock Error"
    }
}
