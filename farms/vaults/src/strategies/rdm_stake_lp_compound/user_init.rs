//! Vault User Init instruction handler

use {
    crate::{traits::UserInit, user_info::UserInfo},
    solana_farm_sdk::{
        instruction::vault::VaultInstruction,
        program::{account, pda},
        vault::Vault,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

impl UserInit for VaultInstruction {
    fn user_init(vault: &Vault, accounts: &[AccountInfo]) -> ProgramResult {
        #[allow(clippy::deprecated_cfg_attr)]
        #[cfg_attr(rustfmt, rustfmt_skip)]
        if let [
            funding_account,
            _vault_metadata,
            _vault_info_account,
            user_account,
            user_info_account,
            _system_program
            ] = accounts
        {
            if account::is_empty(user_info_account)? {
                msg!("Create user info account");
                let seeds: &[&[u8]] = &[
                    b"user_info_account",
                    user_account.key.as_ref(),
                    vault.name.as_bytes(),
                ];
                let bump = pda::init_system_account(
                    funding_account,
                    user_info_account,
                    &vault.vault_program_id,
                    &vault.vault_program_id,
                    seeds,
                    UserInfo::LEN,
                )?;
                let mut user_info = UserInfo::new(user_info_account);
                user_info.init(&vault.name, bump)?;
            } else if !UserInfo::validate_account(vault, user_info_account, user_account.key) {
                msg!("Error: User info account already initialized but not valid");
                return Err(ProgramError::AccountAlreadyInitialized);
            }

            Ok(())
        } else {
            Err(ProgramError::NotEnoughAccountKeys)
        }
    }
}
