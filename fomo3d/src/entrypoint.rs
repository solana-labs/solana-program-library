use solana_program::{
    account_info::AccountInfo, entrypoint, entrypoint::ProgramResult,
    program_error::PrintProgramError, pubkey::Pubkey,
};

use crate::{error::GameError, processor};

entrypoint!(process_instruction);
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    data: &[u8],
) -> ProgramResult {
    if let Err(e) = processor::processor::Processor::process_instruction(program_id, accounts, data)
    {
        e.print::<GameError>();
        return Err(e);
    }
    Ok(())
}
