//! Crank step 2 instruction handler

use {
    crate::{clock::check_min_crank_interval, vault_info::VaultInfo},
    solana_farm_sdk::{
        id::zero,
        program::{account, pda, protocol::raydium},
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
        token_a_reward_custody,
        token_b_reward_custody,
        token_a_custody,
        token_b_custody,
        pool_program_id,
        pool_coin_token_account,
        pool_pc_token_account,
        amm_id,
        amm_authority,
        amm_open_orders,
        amm_target,
        serum_market,
        serum_program_id,
        serum_coin_vault_account,
        serum_pc_vault_account,
        serum_vault_signer,
        serum_bids,
        serum_asks,
        serum_event_queue
        ] = accounts
    {
        // validate accounts
        if vault_authority.key != &vault.vault_authority {
            msg!("Error: Invalid Vault accounts");
            return Err(ProgramError::InvalidArgument);
        }
        if let VaultStrategy::StakeLpCompoundRewards {
            token_a_custody: token_a_custody_key,
            token_b_custody: token_b_custody_key,
            token_a_reward_custody: token_a_reward_custody_key,
            token_b_reward_custody: token_b_reward_custody_key,
            ..
        } = vault.strategy
        {
            if &token_a_reward_custody_key != token_a_reward_custody.key
                || &token_b_reward_custody_key.or_else(||Some(zero::id())).unwrap()
                    != token_b_reward_custody.key
                || &token_a_custody_key != token_a_custody.key
                || &token_b_custody_key.or_else(||Some(zero::id())).unwrap() != token_b_custody.key
            {
                msg!("Error: Invalid custody accounts");
                return Err(ProgramError::InvalidArgument);
            }
        } else {
            msg!("Error: Vault strategy mismatch");
            return Err(ProgramError::InvalidArgument);
        }

        let mut vault_info = VaultInfo::new(vault_info_account);
        check_min_crank_interval(&vault_info)?;
        vault_info.update_crank_time()?;
        vault_info.set_crank_step(2)?;

        // read reward balances
        let dual_rewards = *token_b_reward_custody.key != zero::id();
        let token_a_reward_balance = account::get_token_balance(token_a_reward_custody)?;
        let token_b_reward_balance = if dual_rewards {
            account::get_token_balance(token_b_reward_custody)?
        } else {
            0
        };
        msg!(
            "Read reward balances. token_a_reward_balance: {}, token_b_reward_balance: {}",
            token_a_reward_balance,
            token_b_reward_balance
        );

        // move rewards to token custodies
        let seeds: &[&[&[u8]]] = &[&[
            b"vault_authority",
            vault.name.as_bytes(),
            &[vault.authority_bump],
        ]];

        let token_a_reward_mint = account::get_token_account_mint(token_a_reward_custody)?;
        let token_a_custody_mint = account::get_token_account_mint(token_a_custody)?;
        let token_b_custody_mint = account::get_token_account_mint(token_b_custody)?;

        if token_a_reward_mint == token_a_custody_mint {
            pda::transfer_tokens_with_seeds(
                token_a_reward_custody,
                token_a_custody,
                vault_authority,
                seeds,
                token_a_reward_balance,
            )?;
        } else if token_a_reward_mint == token_b_custody_mint {
            pda::transfer_tokens_with_seeds(
                token_a_reward_custody,
                token_b_custody,
                vault_authority,
                seeds,
                token_a_reward_balance,
            )?;
        }
        if dual_rewards {
            let token_b_reward_mint = account::get_token_account_mint(token_b_reward_custody)?;
            if token_b_reward_mint == token_b_custody_mint {
                pda::transfer_tokens_with_seeds(
                    token_b_reward_custody,
                    token_b_custody,
                    vault_authority,
                    seeds,
                    token_b_reward_balance,
                )?;
            } else if token_b_reward_mint == token_a_custody_mint {
                pda::transfer_tokens_with_seeds(
                    token_b_reward_custody,
                    token_a_custody,
                    vault_authority,
                    seeds,
                    token_b_reward_balance,
                )?;
            }
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
        let (pool_coin_balance, pool_pc_balance) = raydium::get_pool_token_balances(
            pool_coin_token_account,
            pool_pc_token_account,
            amm_open_orders,
            amm_id,
        )?;
        let pool_ratio = if pool_coin_balance != 0 {
            pool_pc_balance as f64 / pool_coin_balance as f64
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

        let token_a_swap_custody = if reverse {
            token_b_custody
        } else {
            token_a_custody
        };
        let token_b_swap_custody = if reverse {
            token_a_custody
        } else {
            token_b_custody
        };
        let coint_extra_amount_in = if !reverse {
            account::to_token_amount(extra_a_tokens.abs(), 0)?
        } else {
            0
        };
        let pc_extra_amount_in = if !reverse {
            0
        } else {
            account::to_token_amount(extra_b_tokens.abs(), 0)?
        };
        if coint_extra_amount_in < 2 && pc_extra_amount_in < 2 {
            msg!("Nothing to do: Not enough tokens to balance");
            return Ok(());
        }

        // get exact swap amounts
        let (amount_in, min_amount_out) = raydium::get_pool_swap_amounts(
            pool_coin_token_account,
            pool_pc_token_account,
            amm_open_orders,
            amm_id,
            coint_extra_amount_in,
            pc_extra_amount_in,
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

        raydium::swap_with_seeds(
            &[
                vault_authority.clone(),
                token_a_swap_custody.clone(),
                token_b_swap_custody.clone(),
                pool_program_id.clone(),
                pool_coin_token_account.clone(),
                pool_pc_token_account.clone(),
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
