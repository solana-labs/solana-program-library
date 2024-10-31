//! Program entrypoint

#![cfg(not(feature = "no-entrypoint"))]

use {
    solana_account_info::AccountInfo, solana_program_entrypoint::ProgramResult,
    solana_pubkey::Pubkey,
};

solana_program_entrypoint::entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    crate::processor::process_instruction(program_id, accounts, instruction_data)
}
