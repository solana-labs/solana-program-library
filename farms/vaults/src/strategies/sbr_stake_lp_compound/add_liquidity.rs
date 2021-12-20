//! Add Liquidity to the Vault instruction handler

use {
    crate::{traits::AddLiquidity, user_info::UserInfo, vault_info::VaultInfo},
    solana_farm_sdk::{
        instruction::vault::VaultInstruction,
        program::{account, protocol::saber},
        vault::{Vault, VaultStrategy},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

impl AddLiquidity for VaultInstruction {
    fn add_liquidity(
        vault: &Vault,
        accounts: &[AccountInfo],
        max_token_a_amount: u64,
        max_token_b_amount: u64,
    ) -> ProgramResult {
        #[allow(clippy::deprecated_cfg_attr)]
        #[cfg_attr(rustfmt, rustfmt_skip)]
        if let [
            user_account,
            _vault_metadata,
            vault_info_account,
            spl_token_program,
            user_info_account,
            user_token_a_account,
            user_token_b_account,
            user_lp_token_account,
            lp_token_custody,
            pool_program_id,
            pool_token_a_account,
            pool_token_b_account,
            lp_token_mint,
            clock_program,
            swap_account,
            swap_authority
            ] = accounts
        {
            // validate accounts
            if let VaultStrategy::StakeLpCompoundRewards {
                lp_token_custody: lp_token_custody_key,
                ..
            } = vault.strategy
            {
                if &lp_token_custody_key != lp_token_custody.key {
                    msg!("Error: Invalid custody accounts");
                    return Err(ProgramError::InvalidArgument);
                }
            } else {
                msg!("Error: Vault strategy mismatch");
                return Err(ProgramError::InvalidArgument);
            }

            if !UserInfo::validate_account(vault, user_info_account, user_account.key) {
                msg!("Error: Invalid user info account");
                return Err(ProgramError::Custom(140));
            }

            let mut vault_info = VaultInfo::new(vault_info_account);
            if !vault_info.is_deposit_allowed()? {
                msg!("Error: Deposits are not allowed for this Vault");
                return Err(ProgramError::Custom(220));
            }

            // read user balances
            let initial_token_a_user_balance = account::get_token_balance(user_token_a_account)?;
            let initial_token_b_user_balance = account::get_token_balance(user_token_b_account)?;
            let initial_lp_user_balance = account::get_token_balance(user_lp_token_account)?;

            saber::add_liquidity(
                &[
                    user_account.clone(),
                    user_token_a_account.clone(),
                    user_token_b_account.clone(),
                    user_lp_token_account.clone(),
                    pool_program_id.clone(),
                    pool_token_a_account.clone(),
                    pool_token_b_account.clone(),
                    lp_token_mint.clone(),
                    spl_token_program.clone(),
                    clock_program.clone(),
                    swap_account.clone(),
                    swap_authority.clone(),
                ],
                max_token_a_amount,
                max_token_b_amount,
            )?;

            // check amounts spent and received
            let tokens_a_spent = account::check_tokens_spent(
                user_token_a_account,
                initial_token_a_user_balance,
                max_token_a_amount,
            )?;
            let tokens_b_spent = account::check_tokens_spent(
                user_token_b_account,
                initial_token_b_user_balance,
                max_token_b_amount,
            )?;
            let lp_tokens_received =
                account::check_tokens_received(user_lp_token_account, initial_lp_user_balance, 1)?;

            // transfer LP tokens to the custody
            msg!(
                "Transfer LP tokens from user. tokens_a_spent: {}, tokens_b_spent: {}, lp_tokens_received: {}",
                tokens_a_spent,
                tokens_b_spent,
                lp_tokens_received
            );
            account::transfer_tokens(
                user_lp_token_account,
                lp_token_custody,
                user_account,
                lp_tokens_received,
            )?;

            // update user stats
            msg!("Update user stats");
            let mut user_info = UserInfo::new(user_info_account);
            user_info.add_liquidity(tokens_a_spent, tokens_b_spent)?;
            user_info.add_lp_tokens_debt(lp_tokens_received)?;

            // update Vault stats
            msg!("Update Vault stats",);
            vault_info.add_liquidity(tokens_a_spent, tokens_b_spent)?;

            Ok(())
        } else {
            Err(ProgramError::NotEnoughAccountKeys)
        }
    }
}
