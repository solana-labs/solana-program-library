//! Program state processor

use {
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
    spl_transfer_hook_interface::{
        instruction::TransferHookInstruction, manager::TransferHookManager,
    },
};

/// Processes an [Execute](enum.TransferHookInstruction.html) instruction.
pub fn process_execute(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    _amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let _source_account_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let _destination_account_info = next_account_info(account_info_iter)?;
    let _authority_info = next_account_info(account_info_iter)?;
    let extra_account_metas_info = next_account_info(account_info_iter)?;

    // Validates extra account metas are provided
    TransferHookManager::check(
        program_id,
        &extra_account_metas_info,
        &mint_info,
        account_info_iter.as_slice(),
    )?;

    // Some other program logic goes here

    Ok(())
}

/// Processes a [InitializeExtraAccountMetas](enum.TransferHookInstruction.html) instruction.
pub fn process_initialize_extra_account_metas(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let extra_account_metas_info = next_account_info(account_info_iter)?;
    let mint_info = next_account_info(account_info_iter)?;
    let authority_info = next_account_info(account_info_iter)?;
    let _system_program_info = next_account_info(account_info_iter)?;

    // Create the account & write the data - runs checks internally
    TransferHookManager::write(
        program_id,
        &extra_account_metas_info,
        &mint_info,
        &authority_info,
        account_info_iter.as_slice(),
    )
}

/// Processes an [Instruction](enum.Instruction.html).
pub fn process(program_id: &Pubkey, accounts: &[AccountInfo], input: &[u8]) -> ProgramResult {
    let instruction = TransferHookInstruction::unpack(input)?;

    match instruction {
        TransferHookInstruction::Execute { amount } => {
            msg!("Instruction: Execute");
            process_execute(program_id, accounts, amount)
        }
        TransferHookInstruction::InitializeExtraAccountMetas => {
            msg!("Instruction: InitializeExtraAccountMetas");
            process_initialize_extra_account_metas(program_id, accounts)
        }
    }
}
