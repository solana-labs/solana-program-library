use {
    num_derive::FromPrimitive,
    solana_program::{decode_error::DecodeError, program_error::ProgramError},
    thiserror::Error,
};

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum NameServiceError {
    #[error("Out of space")]
    OutOfSpace,
}

pub type NameServiceResult = Result<(), NameServiceError>;

impl From<NameServiceError> for ProgramError {
    fn from(e: NameServiceError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for NameServiceError {
    fn type_of() -> &'static str {
        "NameServiceError"
    }
}
