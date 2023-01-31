use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};

use {num_derive::FromPrimitive, thiserror::Error};

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum ErrorCode {
    #[error("Invalid account type")]
    InvalidAccountType = 6000,
    #[error("Data type mismatch")]
    DataTypeMismatch,
}

impl PrintProgramError for ErrorCode {
    fn print<E>(&self) {
        msg!(&self.to_string());
    }
}

impl From<ErrorCode> for ProgramError {
    fn from(e: ErrorCode) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for ErrorCode {
    fn type_of() -> &'static str {
        "Idl Program Error"
    }
}
