//! Vault WithdrawFees instruction handler

use {
    crate::traits::WithdrawFees,
    solana_farm_sdk::{
        instruction::vault::VaultInstruction, program::account, program::pda, vault::Vault,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

impl WithdrawFees for VaultInstruction {
    fn withdraw_fees(vault: &Vault, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        #[allow(clippy::deprecated_cfg_attr)]
        #[cfg_attr(rustfmt, rustfmt_skip)]
        if let [
            _admin_account,
            _vault_metadata,
            _vault_info_account,
            vault_authority,
            _spl_token_program,
            fees_account,
            destination_account
            ] = accounts
        {
            // validate accounts
            if vault_authority.key != &vault.vault_authority {
                msg!("Error: Invalid Vault accounts");
                return Err(ProgramError::InvalidArgument);
            }
            if Some(*fees_account.key) != vault.fees_account_a
                && Some(*fees_account.key) != vault.fees_account_b
            {
                msg!("Error: Invalid fee accounts");
                return Err(ProgramError::InvalidArgument);
            }

            let withdraw_amount = if amount > 0 {
                amount
            } else {
                account::get_token_balance(fees_account)?
            };

            let seeds: &[&[&[u8]]] = &[&[
                b"vault_authority",
                vault.name.as_bytes(),
                &[vault.authority_bump],
            ]];
            pda::transfer_tokens_with_seeds(
                fees_account,
                destination_account,
                vault_authority,
                seeds,
                withdraw_amount,
            )?;

            Ok(())
        } else {
            Err(ProgramError::NotEnoughAccountKeys)
        }
    }
}
