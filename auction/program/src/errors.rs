use {
    num_derive::FromPrimitive,
    solana_program::{
        decode_error::DecodeError,
        msg,
        program_error::{PrintProgramError, ProgramError},
    },
    thiserror::Error,
};

/// Errors that may be returned by the Auction program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum AuctionError {
    /// Account does not have correct owner
    #[error("Account does not have correct owner")]
    IncorrectOwner,

    /// Lamport balance below rent-exempt threshold.
    #[error("Lamport balance below rent-exempt threshold")]
    NotRentExempt,

    /// Bid account provided does not match the derived address.
    #[error("Bid account provided does not match the derived address.")]
    InvalidBidAccount,

    /// Auction account specified is invalid.
    #[error("Auction account specified is invalid.")]
    InvalidAuctionAccount,

    /// Balance too low to make bid.
    #[error("Balance too low to make bid.")]
    BalanceTooLow,

    /// Auction is not currently running.
    #[error("Auction is not currently running.")]
    InvalidState,
}

impl PrintProgramError for AuctionError {
    fn print<E>(&self) {
        msg!(&self.to_string());
    }
}

impl From<AuctionError> for ProgramError {
    fn from(e: AuctionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for AuctionError {
    fn type_of() -> &'static str {
        "Vault Error"
    }
}
