use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use solana_sdk::{
    info,
    program_error::{PrintProgramError, ProgramError},
    program_utils::DecodeError,
};
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum TokenError {
    #[error("insufficient funds")]
    InsufficientFunds,
    #[error("token mismatch")]
    TokenMismatch,
    #[error("not a delegate")]
    NotDelegate,
    #[error("no owner")]
    NoOwner,
}

impl From<TokenError> for ProgramError {
    fn from(e: TokenError) -> Self {
        ProgramError::CustomError(e as u32)
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
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            TokenError::InsufficientFunds => info!("Error: insufficient funds"),
            TokenError::TokenMismatch => info!("Error: token mismatch"),
            TokenError::NotDelegate => info!("Error: not a delegate"),
            TokenError::NoOwner => info!("Error: no owner"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn return_token_error_as_program_error() -> ProgramError {
        TokenError::TokenMismatch.into()
    }

    #[test]
    fn test_print_error() {
        let error = return_token_error_as_program_error();
        error.print::<TokenError>();
    }

    #[test]
    #[should_panic(expected = "CustomError(1)")]
    fn test_error_unwrap() {
        Err::<(), ProgramError>(return_token_error_as_program_error()).unwrap();
    }
}
