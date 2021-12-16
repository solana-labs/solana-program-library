//! Main router implementation.

use {
    crate::{
        add_farm::add_farm, add_pool::add_pool, add_token::add_token, add_vault::add_vault,
        refdb_instruction::process_refdb_instruction, remove_farm::remove_farm,
        remove_pool::remove_pool, remove_token::remove_token, remove_vault::remove_vault,
    },
    solana_farm_sdk::{
        id::{main_router, main_router_admin},
        instruction::main_router::MainInstruction,
        log::sol_log_params_short,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        log::sol_log_compute_units,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};

/// Program's entrypoint.
///
/// # Arguments
/// * `program_id` - Public key of the router.
/// * `accounts` - Accounts, see particular instruction handler for the list.
/// * `instructions_data` - Packed MainInstruction.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Main router entrypoint");
    if cfg!(feature = "debug") {
        sol_log_params_short(accounts, instruction_data);
    }

    if *program_id != main_router::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let accounts_iter = &mut accounts.iter();
    let signer_account = next_account_info(accounts_iter)?;
    if signer_account.key != &main_router_admin::id() {
        msg!(
            "Error: Main router must be called with the admin account {}",
            main_router_admin::id()
        );
        return Err(ProgramError::IllegalOwner);
    }
    if !signer_account.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }

    // Read and unpack instruction data
    let instruction = MainInstruction::unpack(instruction_data)?;

    match instruction {
        MainInstruction::AddVault { vault } => add_vault(program_id, accounts, &vault)?,
        MainInstruction::RemoveVault { name } => remove_vault(program_id, accounts, &name)?,
        MainInstruction::AddPool { pool } => add_pool(program_id, accounts, &pool)?,
        MainInstruction::RemovePool { name } => remove_pool(program_id, accounts, &name)?,
        MainInstruction::AddFarm { farm } => add_farm(program_id, accounts, &farm)?,
        MainInstruction::RemoveFarm { name } => remove_farm(program_id, accounts, &name)?,
        MainInstruction::AddToken { token } => add_token(program_id, accounts, &token)?,
        MainInstruction::RemoveToken { name } => remove_token(program_id, accounts, &name)?,
        MainInstruction::RefDbInstruction { instruction } => {
            process_refdb_instruction(program_id, accounts, instruction)?
        }
    }

    sol_log_compute_units();
    msg!("Main router end of instruction");
    Ok(())
}
