use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use solana_sdk::{
    info,
    program_error::{PrintProgramError, ProgramError},
    program_utils::DecodeError,
};
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum MemoError {
    #[error("invalid UTF-8")]
    InvalidUtf8,
}

impl From<MemoError> for ProgramError {
    fn from(e: MemoError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for MemoError {
    fn type_of() -> &'static str {
        "MemoError"
    }
}

impl PrintProgramError for MemoError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            MemoError::InvalidUtf8 => info!("Error: invalid UTF-8"),
        }
    }
}

#[cfg(test)]
mod test {
    use super::*;

    fn return_memo_error_as_program_error() -> ProgramError {
        MemoError::InvalidUtf8.into()
    }

    #[test]
    fn test_print_error() {
        let error = return_memo_error_as_program_error();
        error.print::<MemoError>();
    }

    #[test]
    #[should_panic(expected = "Custom(0)")]
    fn test_error_unwrap() {
        Err::<(), ProgramError>(return_memo_error_as_program_error()).unwrap();
    }
}
