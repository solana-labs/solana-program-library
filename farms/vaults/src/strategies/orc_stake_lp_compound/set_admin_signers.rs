//! Vault SetAdminSigners instruction handler

use {
    crate::traits::SetAdminSigners,
    solana_farm_sdk::{
        error::FarmError,
        instruction::vault::VaultInstruction,
        program::{account, multisig, multisig::Multisig, pda},
        vault::Vault,
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
    },
};

impl SetAdminSigners for VaultInstruction {
    fn set_admin_signers(
        vault: &Vault,
        accounts: &[AccountInfo],
        min_signatures: u8,
    ) -> ProgramResult {
        let accounts_iter = &mut accounts.iter();

        let signer_account = next_account_info(accounts_iter)?;
        let _vault_metadata = next_account_info(accounts_iter)?;
        let _vault_info_account = next_account_info(accounts_iter)?;
        let _active_multisig_account = next_account_info(accounts_iter)?;
        let vault_multisig_account = next_account_info(accounts_iter)?;
        let _system_program = next_account_info(accounts_iter)?;

        if vault_multisig_account.key != &vault.multisig_account {
            msg!("Error: Invalid vault multisig account");
            return Err(FarmError::IncorrectAccountAddress.into());
        }

        if account::is_empty(vault_multisig_account)? {
            msg!("Init multisig account");
            let seeds: &[&[u8]] = &[b"multisig", vault.name.as_bytes()];
            let _bump = pda::init_system_account(
                signer_account,
                vault_multisig_account,
                &vault.vault_program_id,
                &vault.vault_program_id,
                seeds,
                Multisig::LEN,
            )?;
        } else {
            msg!("Update multisig account");
        }
        multisig::set_signers(
            vault_multisig_account,
            accounts_iter.as_slice(),
            min_signatures,
        )?;

        Ok(())
    }
}
