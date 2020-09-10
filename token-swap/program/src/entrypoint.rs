//! Program entrypoint definitions

#![cfg(feature = "program")]
#![cfg(not(feature = "no-entrypoint"))]

use crate::{error::SwapError, processor::Processor};
use solana_sdk::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult,
    program_error::PrintProgramError, pubkey::Pubkey,
};

entrypoint!(process_instruction);
fn process_instruction<'a>(
    program_id: &Pubkey,
    accounts: &'a [AccountInfo<'a>],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Err(error) = Processor::process(program_id, accounts, instruction_data) {
        // catch the error so we can print it
        error.print::<SwapError>();
        return Err(error);
    }
    Ok(())
}
