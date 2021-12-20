//! Crank step 3 instruction handler

use {
    crate::{clock::check_min_crank_interval, strategies::common, vault_info::VaultInfo},
    solana_farm_sdk::{
        id::zero,
        program::{account, protocol::raydium},
        vault::Vault,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn crank3(vault: &Vault, accounts: &[AccountInfo]) -> ProgramResult {
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
        lp_token_custody,
        token_a_custody,
        token_b_custody,
        pool_program_id,
        pool_coin_token_account,
        pool_pc_token_account,
        lp_token_mint,
        amm_id,
        amm_authority,
        amm_open_orders,
        amm_target,
        serum_market,
        farm_program,
        vault_stake_info,
        farm_id,
        farm_authority,
        farm_lp_token_account,
        farm_reward_token_a_account,
        farm_reward_token_b_account,
        clock_program
        ] = accounts
    {
        if vault_authority.key != &vault.vault_authority {
            msg!("Error: Invalid Vault accounts");
            return Err(ProgramError::InvalidArgument);
        }
        common::check_custody_accounts(
            vault,
            lp_token_custody,
            token_a_custody,
            token_b_custody,
            token_a_reward_custody,
            token_b_reward_custody,
            vault_stake_info,
            true,
        )?;

        let mut vault_info = VaultInfo::new(vault_info_account);
        check_min_crank_interval(&vault_info)?;
        vault_info.update_crank_time()?;
        vault_info.set_crank_step(3)?;

        // read balances
        let token_a_balance = account::get_token_balance(token_a_custody)?;
        let token_b_balance = account::get_token_balance(token_b_custody)?;
        let lp_token_balance = account::get_token_balance(lp_token_custody)?;
        msg!(
            "Read balances. token_a_balance: {}, token_b_balance: {}",
            token_a_balance,
            token_b_balance
        );
        if token_a_balance < 10 || token_b_balance < 10 {
            msg!("Nothing to do: Not enough tokens to compound");
            return Ok(());
        }

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
        if custody_ratio == 0.0 || pool_ratio == 0.0 {
            msg!("Pool ratio is zero");
            return Ok(());
        }
        if (custody_ratio - pool_ratio).abs() * 100.0 / pool_ratio > 10.0 {
            msg!("Unbalanced tokens, run Crank2 first");
            return Ok(());
        }

        // Deposit tokens into the pool
        let seeds: &[&[&[u8]]] = &[&[
            b"vault_authority",
            vault.name.as_bytes(),
            &[vault.authority_bump],
        ]];

        // calculate deposit amounts
        let (max_token_a_deposit_amount, max_token_b_deposit_amount) =
            if custody_ratio >= pool_ratio {
                raydium::get_pool_deposit_amounts(
                    pool_coin_token_account,
                    pool_pc_token_account,
                    amm_open_orders,
                    amm_id,
                    token_a_balance,
                    0,
                )?
            } else {
                raydium::get_pool_deposit_amounts(
                    pool_coin_token_account,
                    pool_pc_token_account,
                    amm_open_orders,
                    amm_id,
                    0,
                    token_b_balance,
                )?
            };
        // one of the amounts can come out over the balance because ratios didn't reflect
        // deposited volume, while get_pool_deposit_amounts does include it.
        // in this case we just flip the side.
        let (max_token_a_deposit_amount, max_token_b_deposit_amount) =
            if max_token_b_deposit_amount > token_b_balance {
                raydium::get_pool_deposit_amounts(
                    pool_coin_token_account,
                    pool_pc_token_account,
                    amm_open_orders,
                    amm_id,
                    0,
                    token_b_balance,
                )?
            } else if max_token_a_deposit_amount > token_a_balance {
                raydium::get_pool_deposit_amounts(
                    pool_coin_token_account,
                    pool_pc_token_account,
                    amm_open_orders,
                    amm_id,
                    token_a_balance,
                    0,
                )?
            } else {
                (max_token_a_deposit_amount, max_token_b_deposit_amount)
            };

        msg!("Deposit tokens into the pool. max_token_a_deposit_amount: {}, max_token_b_deposit_amount: {}",
                        max_token_a_deposit_amount,
                        max_token_b_deposit_amount);
        if max_token_a_deposit_amount == 0
            || max_token_b_deposit_amount == 0
            || raydium::estimate_lp_tokens_amount(
                lp_token_mint,
                max_token_a_deposit_amount,
                max_token_b_deposit_amount,
                pool_coin_balance,
                pool_pc_balance,
            )? < 2
        {
            msg!("Nothing to do: Tokens balance is not large enough");
            return Ok(());
        }

        raydium::add_liquidity_with_seeds(
            &[
                vault_authority.clone(),
                token_a_custody.clone(),
                token_b_custody.clone(),
                lp_token_custody.clone(),
                pool_program_id.clone(),
                pool_coin_token_account.clone(),
                pool_pc_token_account.clone(),
                lp_token_mint.clone(),
                spl_token_program.clone(),
                amm_id.clone(),
                amm_authority.clone(),
                amm_open_orders.clone(),
                amm_target.clone(),
                serum_market.clone(),
            ],
            seeds,
            max_token_a_deposit_amount,
            max_token_b_deposit_amount,
        )?;

        // Check tokens spent and return change back to user
        let tokens_a_spent = account::check_tokens_spent(
            token_a_custody,
            token_a_balance,
            max_token_a_deposit_amount,
        )?;
        let tokens_b_spent = account::check_tokens_spent(
            token_b_custody,
            token_b_balance,
            max_token_b_deposit_amount,
        )?;

        // Stake LP tokens
        let dual_rewards = *farm_reward_token_b_account.key != zero::id();
        let lp_tokens_received =
            account::check_tokens_received(lp_token_custody, lp_token_balance, 1)?;
        msg!(
            "Stake LP tokens. tokens_a_spent: {}, tokens_b_spent: {}, lp_tokens_received: {}",
            tokens_a_spent,
            tokens_b_spent,
            lp_tokens_received
        );
        let token_a_reward_balance = account::get_token_balance(token_a_reward_custody)?;
        let token_b_reward_balance = if dual_rewards {
            account::get_token_balance(token_b_reward_custody)?
        } else {
            0
        };

        raydium::stake_with_seeds(
            &[
                vault_authority.clone(),
                vault_stake_info.clone(),
                lp_token_custody.clone(),
                token_a_reward_custody.clone(),
                token_b_reward_custody.clone(),
                farm_program.clone(),
                farm_lp_token_account.clone(),
                farm_reward_token_a_account.clone(),
                farm_reward_token_b_account.clone(),
                clock_program.clone(),
                spl_token_program.clone(),
                farm_id.clone(),
                farm_authority.clone(),
            ],
            seeds,
            lp_tokens_received,
        )?;
        if lp_token_balance != account::get_token_balance(lp_token_custody)? {
            msg!("Error: Stake instruction didn't result in expected amount of LP tokens spent");
            return Err(ProgramError::Custom(165));
        }

        // update Vault stats
        let token_a_rewards =
            account::get_balance_increase(token_a_reward_custody, token_a_reward_balance)?;
        let token_b_rewards = if dual_rewards {
            account::get_balance_increase(token_b_reward_custody, token_b_reward_balance)?
        } else {
            0
        };
        msg!(
            "Update Vault stats. token_a_rewards: {}, token_b_rewards: {}",
            token_a_rewards,
            token_b_rewards
        );
        vault_info.add_rewards(token_a_rewards, token_b_rewards)?;
        vault_info.add_liquidity(tokens_a_spent, tokens_b_spent)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
