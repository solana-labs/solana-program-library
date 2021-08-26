use num_derive::FromPrimitive;
use num_traits::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum GameError {
    //NOTE: do not change error order - js tests depend on them
    #[error("General calculation failure due to overflow or underflow")]
    CalculationFailure, //0
    #[error("Conversion to u64 failed with an overflow or underflow")]
    ConversionFailure, //1
    #[error("Supplied amount is above threshold")]
    AboveThreshold, //2
    #[error("Supplied amount is below floor")]
    BelowFloor, //3
    #[error("Failed to invoke the SPL Token Program")]
    TokenProgramInvocationFailure, //4
    #[error("Failed to match mint of provided token account")]
    MintMatchFailure, //5
    #[error("Failed to unpack account")]
    UnpackingFailure, //6
    #[error("Failed to match the provided PDA with that internally derived")]
    PDAMatchFailure, //7
    #[error("Invalid owner passed")]
    InvalidOwner, //8
    #[error("Game/round account already initialized")]
    AlreadyInitialized, //9
    #[error("Missing an expected signature")]
    MissingSignature, //a
    #[error("An additional account was expected")]
    MissingAccount, //b
    #[error("Wrong account has been passed")]
    WrongAccount, //c
    #[error("Previous round hasn't yet ended")]
    NotYetEnded, //d
    #[error("Previous round has already ended")]
    AlreadyEnded, //e
    #[error("Token program passed is invalid")]
    InvalidTokenProgram, //f
    #[error("Passed state account if of the wrong state type")]
    InvalidStateType, //10
    #[error("Too few or too many accounts have been passed")]
    InvalidAccountCount, //11
    #[error("Passed account is not rent exempt")]
    NotRentExempt, //12
}

// --------------------------------------- so that fn return type is happy

impl From<GameError> for ProgramError {
    fn from(e: GameError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

// --------------------------------------- to be able to print the error

impl<T> DecodeError<T> for GameError {
    fn type_of() -> &'static str {
        "ouch some error happened"
    }
}

impl PrintProgramError for GameError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            GameError::CalculationFailure => {
                msg!("General calculation failure due to overflow or underflow")
            }
            GameError::ConversionFailure => {
                msg!("Conversion to u64 failed with an overflow or underflow")
            }
            GameError::AboveThreshold => msg!("Supplied amount is above threshold"),
            GameError::BelowFloor => msg!("Supplied amount is below floor"),
            GameError::TokenProgramInvocationFailure => {
                msg!("Failed to invoke the SPL Token Program")
            }
            GameError::MintMatchFailure => msg!("Failed to match mint of provided token account"),
            GameError::UnpackingFailure => msg!("Failed to unpack account"),
            GameError::PDAMatchFailure => {
                msg!("Failed to match the provided PDA with that internally derived")
            }
            GameError::InvalidOwner => msg!("Invalid owner passed"),
            GameError::AlreadyInitialized => msg!("Game/round account already initialized"),
            GameError::MissingSignature => msg!("Missing an expected signature"),
            GameError::MissingAccount => msg!("An additional account was expected"),
            GameError::WrongAccount => msg!("Wrong account has been passed"),
            GameError::NotYetEnded => msg!("Previous round hasn't yet ended"),
            GameError::AlreadyEnded => msg!("Previous round has already ended"),
            GameError::InvalidTokenProgram => msg!("Token program passed is invalid"),
            GameError::InvalidStateType => msg!("Passed state account if of the wrong state type"),
            GameError::InvalidAccountCount => msg!("Too few or too many accounts have been passed"),
            GameError::NotRentExempt => msg!("Passed account is not rent exempt"),
        }
    }
}
