//! Vault Shutdown instruction handler

use {
    crate::{traits::Shutdown, vault_info::VaultInfo},
    solana_farm_sdk::{instruction::vault::VaultInstruction, vault::Vault},
    solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg},
};

impl Shutdown for VaultInstruction {
    fn shutdown(_vault: &Vault, accounts: &[AccountInfo]) -> ProgramResult {
        if let [_admin_account, _vault_metadata, vault_info_account] = accounts {
            // Don't do anything special on shutdown for this Vault, just disable deposits and withdrawals
            let mut vault_info = VaultInfo::new(vault_info_account);
            msg!("disable_deposit");
            vault_info.disable_deposit()?;
            msg!("disable_withdrawal");
            vault_info.disable_withdrawal()?;
            //pda::close_account(admin_account, vault_info_account)
        }
        Ok(())
    }
}
