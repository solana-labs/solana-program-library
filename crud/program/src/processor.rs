//! Program state processor

use crate::{instruction::CrudInstruction, state::Document};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program::{invoke, invoke_signed},
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
    sysvar::Sysvar,
};

/// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = CrudInstruction<Document>::try_from_slice(input)?;
    let account_info_iter = &mut accounts.iter();

    match instruction {
        CrudInstruction::Create {
            document,
        } => {
            msg!("CrudInstruction::Create");

            let data_info = next_account_info(account_info_iter)?;
            let rent_sysvar_info = next_account_info(account_info_iter)?;
            let rent = &Rent::from_account_info(rent_sysvar_info)?;
        }

        CrudInstruction::Update {
            document,
        } => {
            msg!("CrudInstruction::Update");

            let data_info = next_account_info(account_info_iter)?;
            let existing_document = Document::try_from_slice(&data_info.data.borrow())?;
            let owner_info = next_account_info(account_info_iter)?;
        }
    }

    Ok(())
}
