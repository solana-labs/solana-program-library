//! Error types

use {
    num_derive::FromPrimitive,
    solana_program::{decode_error::DecodeError, program_error::ProgramError},
    thiserror::Error,
};

/// Errors that may be returned by the Math program.
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum MathError {
    /// Calculation overflowed the destination number
    #[error("Calculation overflowed the destination number")]
    Overflow,
    /// Calculation underflowed the destination number
    #[error("Calculation underflowed the destination number")]
    Underflow,
}
impl From<MathError> for ProgramError {
    fn from(e: MathError) -> Self {
        ProgramError::Custom(e as u32)
    }
}
impl<T> DecodeError<T> for MathError {
    fn type_of() -> &'static str {
        "Math Error"
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use solana_program::program_error::ProgramError;

    #[test]
    fn test_math_error_from() {
        let program_error = ProgramError::from(MathError::Overflow);
        assert_eq!(program_error, ProgramError::Custom(0));

        let program_error = ProgramError::from(MathError::Underflow);
        assert_eq!(program_error, ProgramError::Custom(1));
    }
}
