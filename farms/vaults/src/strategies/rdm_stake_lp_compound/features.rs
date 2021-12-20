//! Feature toggling instructions handlers

use {
    crate::{traits::Features, vault_info::VaultInfo},
    solana_farm_sdk::{instruction::vault::VaultInstruction, vault::Vault},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

impl Features for VaultInstruction {
    fn set_min_crank_interval(
        _vault: &Vault,
        vault_info: &mut VaultInfo,
        _accounts: &[AccountInfo],
        min_crank_interval_sec: u64,
    ) -> ProgramResult {
        msg!("set_min_crank_interval: {}", min_crank_interval_sec);
        vault_info.set_min_crank_interval(min_crank_interval_sec)
    }

    fn set_fee(
        _vault: &Vault,
        vault_info: &mut VaultInfo,
        _accounts: &[AccountInfo],
        fee: f64,
    ) -> ProgramResult {
        msg!("set_fee: {}", fee);
        if !(0.0..=1.0).contains(&fee) {
            msg!("Error: Invalid new value for fee");
            return Err(ProgramError::InvalidArgument);
        }
        vault_info.set_fee(fee)
    }

    fn set_external_fee(
        _vault: &Vault,
        vault_info: &mut VaultInfo,
        _accounts: &[AccountInfo],
        external_fee: f64,
    ) -> ProgramResult {
        msg!("external_fee: {}", external_fee);
        if !(0.0..=1.0).contains(&external_fee) {
            msg!("Error: Invalid new value for external_fee");
            return Err(ProgramError::InvalidArgument);
        }
        vault_info.set_external_fee(external_fee)
    }

    fn enable_deposit(
        _vault: &Vault,
        vault_info: &mut VaultInfo,
        _accounts: &[AccountInfo],
    ) -> ProgramResult {
        msg!("enable_deposit");
        vault_info.enable_deposit()
    }

    fn disable_deposit(
        _vault: &Vault,
        vault_info: &mut VaultInfo,
        _accounts: &[AccountInfo],
    ) -> ProgramResult {
        msg!("disable_deposit");
        vault_info.disable_deposit()
    }

    fn enable_withdrawal(
        _vault: &Vault,
        vault_info: &mut VaultInfo,
        _accounts: &[AccountInfo],
    ) -> ProgramResult {
        msg!("enable_withdrawal");
        vault_info.enable_withdrawal()
    }

    fn disable_withdrawal(
        _vault: &Vault,
        vault_info: &mut VaultInfo,
        _accounts: &[AccountInfo],
    ) -> ProgramResult {
        msg!("disable_withdrawal");
        vault_info.disable_withdrawal()
    }
}
