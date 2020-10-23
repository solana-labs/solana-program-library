//! Program state processor

use crate::error::LendingError;
use num_traits::FromPrimitive;
use solana_program::{
    account_info::AccountInfo, decode_error::DecodeError, entrypoint::ProgramResult, info,
    program_error::PrintProgramError, pubkey::Pubkey,
};

/// Program state handler.
pub struct Processor {}

impl Processor {
    /// Processes an instruction
    pub fn process(
        _program_id: &Pubkey,
        _accounts: &[AccountInfo],
        _input: &[u8],
    ) -> ProgramResult {
        Ok(())
    }
}

impl PrintProgramError for LendingError {
    fn print<E>(&self)
    where
        E: 'static + std::error::Error + DecodeError<E> + PrintProgramError + FromPrimitive,
    {
        match self {
            LendingError::AlreadyInUse => info!("Error: Lending account already in use"),
        }
    }
}
