//! Vault User Init instruction handler

use {
    crate::{traits::UserInit, user_info::UserInfo},
    solana_farm_sdk::{instruction::vault::VaultInstruction, program::pda, vault::Vault},
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
        pubkey::Pubkey,
    },
};

impl UserInit for VaultInstruction {
    fn user_init(vault: &Vault, accounts: &[AccountInfo]) -> ProgramResult {
        #[allow(clippy::deprecated_cfg_attr)]
        #[cfg_attr(rustfmt, rustfmt_skip)]
        if let [
            user_account,
            _vault_metadata,
            _vault_info_account,
            user_info_account,
            _system_program
            ] = accounts
        {
            if user_info_account.data_is_empty() {
                msg!("Create user info account");
                let seeds: &[&[u8]] = &[
                    b"user_info_account",
                    &user_account.key.to_bytes()[..],
                    vault.name.as_bytes(),
                ];
                let bump = Pubkey::find_program_address(seeds, &vault.vault_program_id).1;
                pda::init_system_account(
                    user_account,
                    user_info_account,
                    &vault.vault_program_id,
                    &vault.vault_program_id,
                    seeds,
                    UserInfo::LEN,
                )?;
                let mut user_info = UserInfo::new(user_info_account);
                user_info.init(&vault.name, bump)?;
            } else if !UserInfo::validate_account(vault, user_info_account, user_account.key) {
                msg!("Error: Invalid user info account");
                return Err(ProgramError::Custom(140));
            }

            Ok(())
        } else {
            Err(ProgramError::NotEnoughAccountKeys)
        }
    }
}
