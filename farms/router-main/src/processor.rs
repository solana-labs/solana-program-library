//! Main router implementation.

use {
    crate::{
        add_farm::add_farm, add_fund::add_fund, add_pool::add_pool, add_token::add_token,
        add_vault::add_vault, refdb_instruction::process_refdb_instruction,
        remove_farm::remove_farm, remove_fund::remove_fund, remove_pool::remove_pool,
        remove_token::remove_token, remove_vault::remove_vault,
        set_admin_signers::set_admin_signers, set_program_admin_signers::set_program_admin_signers,
        set_program_single_authority::set_program_single_authority,
        upgrade_program::upgrade_program,
    },
    solana_farm_sdk::{
        error::FarmError,
        id::{main_router, main_router_admin, main_router_multisig},
        instruction::main_router::MainInstruction,
        log::sol_log_params_short,
        program::multisig,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        //hash::Hasher,
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
    let multisig_account = next_account_info(accounts_iter)?;

    // Read and unpack instruction data
    let instruction = MainInstruction::unpack(instruction_data)?;

    // multisig account and default admin are different for program specific instructions vs others
    let ((expected_multisig_account, multisig_bump), fallback_admin_account) = match instruction {
        MainInstruction::SetProgramAdminSigners { .. }
        | MainInstruction::SetProgramSingleAuthority
        | MainInstruction::UpgradeProgram => (
            Pubkey::find_program_address(
                &[b"multisig", next_account_info(accounts_iter)?.key.as_ref()],
                &main_router::id(),
            ),
            *signer_account.key,
        ),
        _ => ((main_router_multisig::id(), 0), main_router_admin::id()),
    };

    // validate signature and accounts
    if multisig_account.key != &expected_multisig_account {
        msg!("Error: Invalid multisig account");
        return Err(FarmError::IncorrectAccountAddress.into());
    }

    let signatures_left = multisig::sign_multisig(
        multisig_account,
        signer_account,
        &fallback_admin_account,
        &accounts[1..],
        instruction_data,
    )?;
    if signatures_left > 0 {
        msg!(
            "Instruction has been signed but more signatures are required: {}",
            signatures_left
        );
        return Ok(());
    }

    // process instruction
    match instruction {
        MainInstruction::AddVault { vault } => add_vault(program_id, accounts, &vault)?,
        MainInstruction::RemoveVault { name, refdb_index } => {
            remove_vault(program_id, accounts, &name, refdb_index)?
        }
        MainInstruction::AddPool { pool } => add_pool(program_id, accounts, &pool)?,
        MainInstruction::RemovePool { name, refdb_index } => {
            remove_pool(program_id, accounts, &name, refdb_index)?
        }
        MainInstruction::AddFarm { farm } => add_farm(program_id, accounts, &farm)?,
        MainInstruction::RemoveFarm { name, refdb_index } => {
            remove_farm(program_id, accounts, &name, refdb_index)?
        }
        MainInstruction::AddFund { fund } => add_fund(program_id, accounts, &fund)?,
        MainInstruction::RemoveFund { name, refdb_index } => {
            remove_fund(program_id, accounts, &name, refdb_index)?
        }
        MainInstruction::AddToken { token } => add_token(program_id, accounts, &token)?,
        MainInstruction::RemoveToken { name, refdb_index } => {
            remove_token(program_id, accounts, &name, refdb_index)?
        }
        MainInstruction::RefDbInstruction { instruction } => {
            process_refdb_instruction(program_id, accounts, instruction)?
        }
        MainInstruction::SetAdminSigners { min_signatures } => {
            set_admin_signers(program_id, accounts, min_signatures)?
        }
        MainInstruction::SetProgramAdminSigners { min_signatures } => {
            set_program_admin_signers(program_id, accounts, min_signatures)?
        }
        MainInstruction::SetProgramSingleAuthority => {
            set_program_single_authority(program_id, accounts, multisig_bump)?
        }
        MainInstruction::UpgradeProgram => upgrade_program(program_id, accounts, multisig_bump)?,
    }

    sol_log_compute_units();
    msg!("Main router end of instruction");
    Ok(())
}
