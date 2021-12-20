use thiserror::Error;

use solana_program::program_error::ProgramError;

#[derive(Error, Debug, Copy, Clone)]
pub enum BinaryOptionError {
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

impl From<BinaryOptionError> for ProgramError {
    fn from(e: BinaryOptionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
