//! Vaults entrypoint.

#![cfg(not(feature = "no-entrypoint"))]

solana_security_txt::security_txt! {
    name: "Solana Farms",
    project_url: "https://github.com/solana-labs/solana-program-library/tree/master/farms",
    contacts: "email:solana.farms@protonmail.com",
    policy: "",
    preferred_languages: "en",
    auditors: "Halborn"
}

use {
    crate::{traits::*, vault_info::VaultInfo},
    solana_farm_sdk::{
        error::FarmError,
        id::{main_router, main_router_admin, main_router_multisig},
        instruction::vault::VaultInstruction,
        log::sol_log_params_short,
        program::multisig,
        refdb,
        string::ArrayString64,
        traits::Packed,
        vault::Vault,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint,
        entrypoint::ProgramResult,
        log::sol_log_compute_units,
        msg,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
};

fn log_start(instruction: &str, vault_name: &ArrayString64) {
    msg!(
        "Processing VaultInstruction::{} for {}",
        instruction,
        vault_name.as_str()
    );
    sol_log_compute_units();
}

fn log_end(vault_name: &ArrayString64) {
    sol_log_compute_units();
    msg!("Vault {} end of instruction", vault_name.as_str());
}

fn check_authority(
    accounts: &[AccountInfo],
    instruction_data: &[u8],
    vault: &Vault,
) -> Result<bool, ProgramError> {
    let account_info_iter = &mut accounts.iter();
    let admin_account = next_account_info(account_info_iter)?;
    let _vault_metadata = next_account_info(account_info_iter)?;
    let _vault_info_account = next_account_info(account_info_iter)?;
    let multisig_account = next_account_info(account_info_iter)?;

    if multisig_account.key != &vault.multisig_account
        && multisig_account.key != &main_router_multisig::id()
    {
        msg!("Error: Invalid multisig account");
        return Err(FarmError::IncorrectAccountAddress.into());
    }

    let signatures_left = multisig::sign_multisig(
        multisig_account,
        admin_account,
        &main_router_admin::id(),
        &accounts[1..],
        instruction_data,
    )?;
    if signatures_left > 0 {
        msg!(
            "Instruction has been signed but more signatures are required: {}",
            signatures_left
        );
        return Ok(false);
    }

    Ok(true)
}

entrypoint!(process_instruction);
/// Program's entrypoint.
///
/// # Arguments
/// * `program_id` - Public key of the vault.
/// * `accounts` - Accounts, see handlers in particular strategy for the list.
/// * `instructions_data` - Packed VaultInstruction.
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    msg!("Vault entrypoint");
    if cfg!(feature = "debug") {
        sol_log_params_short(accounts, instruction_data);
    }

    let account_info_iter = &mut accounts.iter();
    let _user_account = next_account_info(account_info_iter)?;
    let vault_metadata = next_account_info(account_info_iter)?;
    let vault_info_account = next_account_info(account_info_iter)?;

    // unpack Vault's metadata and validate Vault accounts
    let vault = Vault::unpack(&vault_metadata.try_borrow_data()?)?;
    let derived_vault_metadata = refdb::find_target_pda_with_bump(
        refdb::StorageType::Vault,
        &vault.name,
        vault.metadata_bump,
    )?;
    if &vault.info_account != vault_info_account.key
        || &derived_vault_metadata != vault_metadata.key
        || vault_metadata.owner != &main_router::id()
    {
        msg!("Error: Invalid Vault accounts");
        return Err(ProgramError::InvalidArgument);
    }
    if &vault.vault_program_id != program_id {
        msg!("Error: Invalid Vault program id");
        return Err(ProgramError::IncorrectProgramId);
    }

    // Read and unpack instruction data
    let instruction = VaultInstruction::unpack(instruction_data)?;

    match instruction {
        VaultInstruction::UserInit => {
            log_start("UserInit", &vault.name);
            VaultInstruction::user_init(&vault, accounts)?;
        }
        VaultInstruction::AddLiquidity {
            max_token_a_amount,
            max_token_b_amount,
        } => {
            log_start("AddLiquidity", &vault.name);
            VaultInstruction::add_liquidity(
                &vault,
                accounts,
                max_token_a_amount,
                max_token_b_amount,
            )?;
        }
        VaultInstruction::LockLiquidity { amount } => {
            log_start("LockLiquidity", &vault.name);
            VaultInstruction::lock_liquidity(&vault, accounts, amount)?;
        }
        VaultInstruction::UnlockLiquidity { amount } => {
            log_start("UnlockLiquidity", &vault.name);
            VaultInstruction::unlock_liquidity(&vault, accounts, amount)?;
        }
        VaultInstruction::RemoveLiquidity { amount } => {
            log_start("RemoveLiquidity", &vault.name);
            VaultInstruction::remove_liquidity(&vault, accounts, amount)?;
        }
        VaultInstruction::SetMinCrankInterval { min_crank_interval } => {
            log_start("SetMinCrankInterval", &vault.name);
            if check_authority(accounts, instruction_data, &vault)? {
                VaultInstruction::set_min_crank_interval(
                    &vault,
                    &mut VaultInfo::new(vault_info_account),
                    accounts,
                    min_crank_interval as u64,
                )?;
            }
        }
        VaultInstruction::SetFee { fee } => {
            log_start("SetFee", &vault.name);
            if check_authority(accounts, instruction_data, &vault)? {
                VaultInstruction::set_fee(
                    &vault,
                    &mut VaultInfo::new(vault_info_account),
                    accounts,
                    fee as f64,
                )?;
            }
        }
        VaultInstruction::SetExternalFee { external_fee } => {
            log_start("SetExternalFee", &vault.name);
            if check_authority(accounts, instruction_data, &vault)? {
                VaultInstruction::set_external_fee(
                    &vault,
                    &mut VaultInfo::new(vault_info_account),
                    accounts,
                    external_fee as f64,
                )?;
            }
        }
        VaultInstruction::EnableDeposits => {
            log_start("EnableDeposits", &vault.name);
            if check_authority(accounts, instruction_data, &vault)? {
                VaultInstruction::enable_deposits(
                    &vault,
                    &mut VaultInfo::new(vault_info_account),
                    accounts,
                )?;
            }
        }
        VaultInstruction::DisableDeposits => {
            log_start("DisableDeposits", &vault.name);
            if check_authority(accounts, instruction_data, &vault)? {
                VaultInstruction::disable_deposits(
                    &vault,
                    &mut VaultInfo::new(vault_info_account),
                    accounts,
                )?;
            }
        }
        VaultInstruction::EnableWithdrawals => {
            log_start("EnableWithdrawals", &vault.name);
            if check_authority(accounts, instruction_data, &vault)? {
                VaultInstruction::enable_withdrawals(
                    &vault,
                    &mut VaultInfo::new(vault_info_account),
                    accounts,
                )?;
            }
        }
        VaultInstruction::DisableWithdrawals => {
            log_start("DisableWithdrawals", &vault.name);
            if check_authority(accounts, instruction_data, &vault)? {
                VaultInstruction::disable_withdrawals(
                    &vault,
                    &mut VaultInfo::new(vault_info_account),
                    accounts,
                )?;
            }
        }
        VaultInstruction::Crank { step } => {
            log_start("Crank", &vault.name);
            VaultInstruction::crank(&vault, accounts, step)?;
        }
        VaultInstruction::Init { step } => {
            log_start("Init", &vault.name);
            if check_authority(accounts, instruction_data, &vault)? {
                VaultInstruction::init(&vault, accounts, step)?;
            }
        }
        VaultInstruction::Shutdown => {
            log_start("Shutdown", &vault.name);
            if check_authority(accounts, instruction_data, &vault)? {
                VaultInstruction::shutdown(&vault, accounts)?;
            }
        }
        VaultInstruction::WithdrawFees { amount } => {
            log_start("WithdrawFees", &vault.name);
            if check_authority(accounts, instruction_data, &vault)? {
                VaultInstruction::withdraw_fees(&vault, accounts, amount)?;
            }
        }
        VaultInstruction::SetAdminSigners { min_signatures } => {
            log_start("SetAdminSigners", &vault.name);
            if check_authority(accounts, instruction_data, &vault)? {
                VaultInstruction::set_admin_signers(&vault, accounts, min_signatures)?;
            }
        }
        VaultInstruction::RemoveMultisig => {
            log_start("RemoveMultisig", &vault.name);
            if check_authority(accounts, instruction_data, &vault)? {
                VaultInstruction::remove_multisig(&vault, accounts)?;
            }
        }
    }

    log_end(&vault.name);
    Ok(())
}
