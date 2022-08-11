//! Remove Liquidity from the Vault instruction handler

use {
    crate::{traits::RemoveLiquidity, user_info::UserInfo, vault_info::VaultInfo},
    solana_farm_sdk::{
        instruction::vault::VaultInstruction,
        program::{account, protocol::raydium},
        vault::{Vault, VaultStrategy},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

impl RemoveLiquidity for VaultInstruction {
    fn remove_liquidity(vault: &Vault, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
        #[allow(clippy::deprecated_cfg_attr)]
        #[cfg_attr(rustfmt, rustfmt_skip)]
        if let [
            user_account,
            _vault_metadata,
            vault_info_account,
            vault_authority,
            spl_token_program,
            user_info_account,
            user_token_a_account,
            user_token_b_account,
            lp_token_custody,
            pool_program_id,
            pool_withdraw_queue,
            pool_temp_lp_token_account,
            pool_coin_token_account,
            pool_pc_token_account,
            lp_token_mint,
            amm_id,
            amm_authority,
            amm_open_orders,
            amm_target,
            serum_market,
            serum_program_id,
            serum_bids,
            serum_asks,
            serum_event_queue,
            serum_coin_vault_account,
            serum_pc_vault_account,
            serum_vault_signer
            ] = accounts
        {
            // validate accounts
            if vault_authority.key != &vault.vault_authority {
                msg!("Error: Invalid Vault accounts");
                return Err(ProgramError::InvalidArgument);
            }
            if !user_account.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }
            if !account::check_token_account_owner(user_token_a_account, user_account.key)?
                || !account::check_token_account_owner(user_token_b_account, user_account.key)?
            {
                msg!("Error: Invalid token account owner");
                return Err(ProgramError::IllegalOwner);
            }
            if let VaultStrategy::StakeLpCompoundRewards {
                pool_id: pool_id_key,
                lp_token_custody: lp_token_custody_key,
                ..
            } = vault.strategy
            {
                if &lp_token_custody_key != lp_token_custody.key
                {
                    msg!("Error: Invalid LP custody account");
                    return Err(ProgramError::InvalidArgument);
                }
                if &pool_id_key != amm_id.key {
                    msg!("Error: Invalid pool id");
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
            if !vault_info.is_withdrawal_allowed()? {
                msg!("Error: Withdrawals are not allowed for this Vault");
                return Err(ProgramError::Custom(230));
            }

            // check lp balance
            let mut user_info = UserInfo::new(user_info_account);
            let lp_tokens_debt = user_info.get_lp_tokens_debt()?;
            msg!("Read balances. lp_tokens_debt: {}", lp_tokens_debt);

            let lp_remove_amount = if amount > 0 {
                if lp_tokens_debt < amount {
                    msg!("Error: Insufficient funds");
                    return Err(ProgramError::InsufficientFunds);
                }
                amount
            } else {
                lp_tokens_debt
            };
            if lp_remove_amount == 0 {
                msg!("Error: Zero balance. Forgot to unlock funds?");
                return Err(ProgramError::InsufficientFunds);
            }

            // remove liquidity from the pool
            let seeds: &[&[&[u8]]] = &[&[
                b"vault_authority",
                vault.name.as_bytes(),
                &[vault.authority_bump],
            ]];

            let initial_token_a_account_balance = account::get_token_balance(user_token_a_account)?;
            let initial_token_b_account_balance = account::get_token_balance(user_token_b_account)?;
            let initial_lp_tokens_balance = account::get_token_balance(lp_token_custody)?;

            msg!(
                "Remove liquidity from the pool. lp_remove_amount: {}",
                lp_remove_amount
            );
            raydium::remove_liquidity_with_seeds(
                &[
                    vault_authority.clone(),
                    user_token_a_account.clone(),
                    user_token_b_account.clone(),
                    lp_token_custody.clone(),
                    pool_program_id.clone(),
                    pool_withdraw_queue.clone(),
                    pool_temp_lp_token_account.clone(),
                    pool_coin_token_account.clone(),
                    pool_pc_token_account.clone(),
                    lp_token_mint.clone(),
                    spl_token_program.clone(),
                    amm_id.clone(),
                    amm_authority.clone(),
                    amm_open_orders.clone(),
                    amm_target.clone(),
                    serum_market.clone(),
                    serum_program_id.clone(),
                    serum_bids.clone(),
                    serum_asks.clone(),
                    serum_event_queue.clone(),
                    serum_coin_vault_account.clone(),
                    serum_pc_vault_account.clone(),
                    serum_vault_signer.clone(),
                ],
                seeds,
                lp_remove_amount,
            )?;

            // check tokens received
            let tokens_a_received =
                account::get_balance_increase(user_token_a_account, initial_token_a_account_balance)?;
            let tokens_b_received =
                account::get_balance_increase(user_token_b_account, initial_token_b_account_balance)?;
            if tokens_a_received == 0 && tokens_b_received == 0 {
                msg!("Error: Remove liquidity instruction didn't result in any of the tokens received");
                return Err(ProgramError::Custom(190));
            }
            let _ = account::check_tokens_spent(
                lp_token_custody,
                initial_lp_tokens_balance,
                lp_remove_amount,
            )?;

            // update user stats
            msg!(
                "Update user stats. tokens_a_received: {}, tokens_b_received: {}",
                tokens_a_received,
                tokens_b_received
            );
            user_info.remove_liquidity(tokens_a_received, tokens_b_received)?;
            user_info.remove_lp_tokens_debt(lp_remove_amount)?;

            // update vault stats
            msg!("Update Vault stats");
            vault_info.remove_liquidity(tokens_a_received, tokens_b_received)?;

            Ok(())
        } else {
            Err(ProgramError::NotEnoughAccountKeys)
        }
    }
}
