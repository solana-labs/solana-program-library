use thiserror::Error;

use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum BettingPoolError {
    #[error("PublicKeyMismatch")]
    PublicKeyMismatch,
    #[error("InvalidMintAuthority")]
    InvalidMintAuthority,
    #[error("NotMintAuthority")]
    NotMintAuthority,
    #[error("InvalidSupply")]
    InvalidSupply,
    #[error("InvalidWinner")]
    InvalidWinner,
    #[error("UninitializedAccount")]
    UninitializedAccount,
    #[error("IncorrectOwner")]
    IncorrectOwner,
    #[error("AlreadySettled")]
    AlreadySettled,
    #[error("BetNotSettled")]
    BetNotSettled,
    #[error("TokenNotFoundInPool")]
    TokenNotFoundInPool,
    #[error("PublicKeysShouldBeUnique")]
    PublicKeysShouldBeUnique,
    #[error("TradePricesIncorrect")]
    TradePricesIncorrect,
}

impl From<BettingPoolError> for ProgramError {
    fn from(e: BettingPoolError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
