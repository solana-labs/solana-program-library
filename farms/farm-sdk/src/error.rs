//! Error types
use {solana_program::program_error::ProgramError, thiserror::Error};

/// General error
#[derive(Copy, Clone, Debug, Eq, Error, PartialEq)]
pub enum FarmError {
    #[error("Checked math operation overflow")]
    MathOverflow,
    #[error("Invalid argument value")]
    InvalidValue,
    #[error("Invalid RefDB record")]
    InvalidRefdbRecord,
    #[error("RefDB is too large to clear")]
    RefdbTooLarge,
    #[error("RefDB record counter mismatch")]
    RefdbRecordCounterMismatch,
    #[error("RefDB record name mismatch")]
    RefdbRecordNameMismatch,
    #[error("RefDB record type mismatch")]
    RefdbRecordTypeMismatch,
    #[error("RefDB record not found")]
    RefdbRecordNotFound,
    #[error("Unexpected token balance decrease")]
    UnexpectedBalanceDecrease,
    #[error("Unexpected token balance increase")]
    UnexpectedBalanceIncrease,
    #[error("Invoked program overspent")]
    ProgramOverspent,
    #[error("Invoked program didn't return enough tokens")]
    ProgramInsufficientTransfer,
    #[error("Liquidity Pool is empty")]
    EmptyPool,
    #[error("Invalid Oracle account")]
    OracleInvalidAccount,
    #[error("Invalid Oracle State")]
    OracleInvalidState,
    #[error("Stale Oracle price")]
    OracleStalePrice,
    #[error("Invalid Oracle price")]
    OracleInvalidPrice,
    #[error("Incorrect account address")]
    IncorrectAccountAddress,
    #[error("Account not authorized")]
    AccountNotAuthorized,
    #[error("Already signed")]
    AlreadySigned,
    #[error("Already executed")]
    AlreadyExecuted,
    #[error("Too early")]
    TooEarly,
}

impl From<FarmError> for ProgramError {
    fn from(e: FarmError) -> Self {
        ProgramError::Custom(1000u32 + e as u32)
    }
}
