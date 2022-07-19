//! Vault RemoveMultisig instruction handler

use {
    crate::traits::RemoveMultisig,
    solana_farm_sdk::{instruction::vault::VaultInstruction, program::account, vault::Vault},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

impl RemoveMultisig for VaultInstruction {
    fn remove_multisig(vault: &Vault, accounts: &[AccountInfo]) -> ProgramResult {
        #[allow(clippy::deprecated_cfg_attr)]
        #[cfg_attr(rustfmt, rustfmt_skip)]
        if let [
            admin_account,
            _vault_metadata,
            _vault_info_account,
            _active_multisig_account,
            vault_multisig_account
            ] = accounts
        {
            msg!("Close multisig account");
            account::close_system_account(admin_account, vault_multisig_account, &vault.vault_program_id)?;

            Ok(())
        } else {
            Err(ProgramError::NotEnoughAccountKeys)
        }
    }
}
