//! Crank step 2 instruction handler

use {
    crate::{strategies::common, vault_info::VaultInfo},
    solana_farm_sdk::{
        id::zero,
        program,
        program::{account, pda, protocol::orca},
        vault::{Vault, VaultStrategy},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn crank2(vault: &Vault, accounts: &[AccountInfo]) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _funding_account,
        _vault_metadata,
        vault_info_account,
        vault_authority,
        spl_token_program,
        reward_token_custody,
        token_a_custody,
        token_b_custody,
        pool_program_id,
        pool_token_a_account,
        pool_token_b_account,
        lp_token_mint,
        amm_id,
        amm_authority,
        pool_fees_account,
        rdex_pool_token_a_account,
        rdex_pool_token_b_account,
        rdex_lp_token_mint,
        rdex_amm_id,
        rdex_amm_authority,
        rdex_pool_fees_account,
        sysvar_account
        ] = accounts
    {
        // validate accounts
        if vault_authority.key != &vault.vault_authority {
            msg!("Error: Invalid Vault accounts");
            return Err(ProgramError::InvalidArgument);
        }
        if let VaultStrategy::StakeLpCompoundRewards {
            pool_id: pool_id_key,
            token_a_custody: token_a_custody_key,
            token_b_custody: token_b_custody_key,
            token_a_reward_custody: token_a_reward_custody_key,
            reward_exchange_pool_id,
            ..
        } = vault.strategy
        {
            if &token_a_reward_custody_key != reward_token_custody.key
                || &token_a_custody_key != token_a_custody.key
                || &token_b_custody_key.or_else(|| Some(zero::id())).unwrap() != token_b_custody.key
            {
                msg!("Error: Invalid custody accounts");
                return Err(ProgramError::InvalidArgument);
            }
            if &pool_id_key != amm_id.key
                || &reward_exchange_pool_id
                    .or_else(|| Some(zero::id()))
                    .unwrap()
                    != rdex_amm_id.key
            {
                msg!("Error: Invalid pool id");
                return Err(ProgramError::InvalidArgument);
            }
        } else {
            msg!("Error: Vault strategy mismatch");
            return Err(ProgramError::InvalidArgument);
        }

        if !program::is_last_instruction(sysvar_account)? {
            msg!("Error: Crank2 must be the last instruction in the transaction");
            return Err(ProgramError::InvalidArgument);
        }

        let mut vault_info = VaultInfo::new(vault_info_account);
        common::check_min_crank_interval(&vault_info)?;
        vault_info.update_crank_time()?;
        vault_info.set_crank_step(2)?;

        // read reward balance
        let reward_token_balance = account::get_token_balance(reward_token_custody)?;
        msg!(
            "Read reward balance. reward_token_balance: {}",
            reward_token_balance
        );

        // move rewards to token custodies
        let seeds: &[&[&[u8]]] = &[&[
            b"vault_authority",
            vault.name.as_bytes(),
            &[vault.authority_bump],
        ]];

        let reward_token_mint = account::get_token_account_mint(reward_token_custody)?;
        let token_a_custody_mint = account::get_token_account_mint(token_a_custody)?;
        let token_b_custody_mint = account::get_token_account_mint(token_b_custody)?;

        if reward_token_mint == token_a_custody_mint {
            pda::transfer_tokens_with_seeds(
                reward_token_custody,
                token_a_custody,
                vault_authority,
                seeds,
                reward_token_balance,
            )?;
        } else if reward_token_mint == token_b_custody_mint {
            pda::transfer_tokens_with_seeds(
                reward_token_custody,
                token_b_custody,
                vault_authority,
                seeds,
                reward_token_balance,
            )?;
        } else if reward_token_balance > 0 {
            // if rewards are not in pool tokens we need to swap
            // determine swap direction
            let rdex_pool_token_b_mint =
                account::get_token_account_mint(rdex_pool_token_b_account)?;
            let destination_token_custody = if rdex_pool_token_b_mint == token_a_custody_mint {
                token_a_custody
            } else if rdex_pool_token_b_mint == token_b_custody_mint {
                token_b_custody
            } else {
                msg!("Error: Invalid reward exchange pool");
                return Err(ProgramError::InvalidArgument);
            };
            // calculate amounts
            let initial_tokens_spent_balance = account::get_token_balance(reward_token_custody)?;
            let initial_tokens_received_balance =
                account::get_token_balance(destination_token_custody)?;
            let (amount_in, min_amount_out) = orca::get_pool_swap_amounts(
                rdex_pool_token_a_account,
                rdex_pool_token_b_account,
                reward_token_balance,
                0,
            )?;
            // swap
            msg!(
                "Swap rewards. amount_in: {}, min_amount_out {}",
                amount_in,
                min_amount_out
            );
            orca::swap_with_seeds(
                &[
                    vault_authority.clone(),
                    reward_token_custody.clone(),
                    destination_token_custody.clone(),
                    pool_program_id.clone(),
                    rdex_pool_token_a_account.clone(),
                    rdex_pool_token_b_account.clone(),
                    rdex_lp_token_mint.clone(),
                    spl_token_program.clone(),
                    rdex_amm_id.clone(),
                    rdex_amm_authority.clone(),
                    rdex_pool_fees_account.clone(),
                ],
                seeds,
                amount_in,
                min_amount_out,
            )?;
            // check results
            let _ = account::check_tokens_spent(
                reward_token_custody,
                initial_tokens_spent_balance,
                amount_in,
            )?;
            let _ = account::check_tokens_received(
                destination_token_custody,
                initial_tokens_received_balance,
                min_amount_out,
            )?;
        }

        // read balances
        let token_a_balance = account::get_token_balance(token_a_custody)?;
        let token_b_balance = account::get_token_balance(token_b_custody)?;
        msg!(
            "Read balances. token_a_balance: {}, token_b_balance: {}",
            token_a_balance,
            token_b_balance
        );
        if token_a_balance < 10 && token_b_balance < 10 {
            msg!("Nothing to do: Not enough tokens to balance");
            return Ok(());
        }

        // rebalance
        // compute and check pool ratios
        let (pool_token_a_balance, pool_token_b_balance) =
            orca::get_pool_token_balances(pool_token_a_account, pool_token_b_account)?;
        let pool_ratio = if pool_token_a_balance != 0 {
            pool_token_b_balance as f64 / pool_token_a_balance as f64
        } else {
            0.0
        };
        let custody_ratio = account::get_token_pair_ratio(token_a_custody, token_b_custody)?;
        msg!(
            "Compute pool ratios. custody_ratio: {}, pool_ratio: {}",
            custody_ratio,
            pool_ratio
        );
        if pool_ratio == 0.0 {
            msg!("Can't balance: Pool ratio is zero");
            return Ok(());
        }
        if custody_ratio > 0.0 && (custody_ratio - pool_ratio).abs() * 100.0 / pool_ratio < 3.0 {
            msg!("Nothing to do: Already balanced");
            return Ok(());
        }

        // compute ui amount to exchange
        let extra_a_tokens =
            (token_a_balance as f64 * pool_ratio - token_b_balance as f64) / (2.0 * pool_ratio);
        let extra_b_tokens = extra_a_tokens * pool_ratio;
        let reverse = extra_a_tokens < 0.0;
        msg!(
            "Rebalance tokens. reverse: {}, extra_a_tokens: {}, extra_b_tokens: {}",
            reverse,
            extra_a_tokens,
            extra_b_tokens
        );

        let (
            token_a_swap_custody,
            token_b_swap_custody,
            pool_token_a_swap_account,
            pool_token_b_swap_account,
        ) = if reverse {
            (
                token_b_custody,
                token_a_custody,
                pool_token_b_account,
                pool_token_a_account,
            )
        } else {
            (
                token_a_custody,
                token_b_custody,
                pool_token_a_account,
                pool_token_b_account,
            )
        };
        let token_a_extra_amount_in = if !reverse {
            account::to_token_amount(extra_a_tokens.abs(), 0)?
        } else {
            0
        };
        let token_b_extra_amount_in = if !reverse {
            0
        } else {
            account::to_token_amount(extra_b_tokens.abs(), 0)?
        };
        if token_a_extra_amount_in < 2 && token_b_extra_amount_in < 2 {
            msg!("Nothing to do: Not enough tokens to balance");
            return Ok(());
        }

        // get exact swap amounts
        let (amount_in, min_amount_out) = orca::get_pool_swap_amounts(
            pool_token_a_account,
            pool_token_b_account,
            token_a_extra_amount_in,
            token_b_extra_amount_in,
        )?;
        msg!(
            "Swap. amount_in: {}, min_amount_out {}",
            amount_in,
            min_amount_out
        );
        if amount_in == 0 || min_amount_out == 0 {
            msg!("Nothing to do: Not enough tokens to balance");
            return Ok(());
        }

        let initial_tokens_spent_balance = account::get_token_balance(token_a_swap_custody)?;
        let initial_tokens_received_balance = account::get_token_balance(token_b_swap_custody)?;

        orca::swap_with_seeds(
            &[
                vault_authority.clone(),
                token_a_swap_custody.clone(),
                token_b_swap_custody.clone(),
                pool_program_id.clone(),
                pool_token_a_swap_account.clone(),
                pool_token_b_swap_account.clone(),
                lp_token_mint.clone(),
                spl_token_program.clone(),
                amm_id.clone(),
                amm_authority.clone(),
                pool_fees_account.clone(),
            ],
            seeds,
            amount_in,
            min_amount_out,
        )?;
        let _ = account::check_tokens_spent(
            token_a_swap_custody,
            initial_tokens_spent_balance,
            amount_in,
        )?;
        let tokens_received = account::check_tokens_received(
            token_b_swap_custody,
            initial_tokens_received_balance,
            min_amount_out,
        )?;

        msg!(
            "Done. tokens_received: {}, token_a_balance: {}, token_b_balance: {}",
            tokens_received,
            account::get_token_balance(token_a_custody)?,
            account::get_token_balance(token_b_custody)?
        );

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
