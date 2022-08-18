//! Program entrypoint definitions
#![cfg(all(target_os = "solana", not(feature = "no-entrypoint")))]

use crate::{error::GovernanceError, processor};
use solana_program::{
    account_info::AccountInfo, declare_id, entrypoint, entrypoint::ProgramResult,
    program_error::PrintProgramError, pubkey::Pubkey,
};

// Todo: Feature-gate this to only run when publishing shank IDL
declare_id!("GovER5Lthms3bLBqWub97yVrMmEogzX7xNjdXpPPCVZw");

entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    if let Err(error) = processor::process_instruction(program_id, accounts, instruction_data) {
        // catch the error so we can print it
        error.print::<GovernanceError>();
        return Err(error);
    }
    Ok(())
}
