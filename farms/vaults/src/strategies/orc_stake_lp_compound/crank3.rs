//! Crank step 3 instruction handler

use {
    crate::{strategies::common, vault_info::VaultInfo},
    solana_farm_sdk::{
        id::zero,
        program::{account, protocol::orca},
        vault::{Vault, VaultStrategy},
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
        reward_token_custody,
        lp_token_custody,
        token_a_custody,
        token_b_custody,
        pool_program_id,
        pool_token_a_account,
        pool_token_b_account,
        lp_token_mint,
        amm_id,
        amm_authority,
        farm_program,
        vault_stake_info,
        vault_stake_custody,
        farm_id,
        farm_authority,
        farm_lp_token_mint,
        base_token_vault,
        reward_token_vault
        ] = accounts
    {
        if vault_authority.key != &vault.vault_authority {
            msg!("Error: Invalid Vault accounts");
            return Err(ProgramError::InvalidArgument);
        }
        if let VaultStrategy::StakeLpCompoundRewards {
            pool_id: pool_id_key,
            farm_id: farm_id_key,
            lp_token_custody: lp_token_custody_key,
            token_a_custody: token_a_custody_key,
            token_b_custody: token_b_custody_key,
            token_a_reward_custody: token_a_reward_custody_key,
            vault_stake_info: vault_stake_info_key,
            vault_stake_custody: vault_stake_custody_key,
            ..
        } = vault.strategy
        {
            if &vault_stake_info_key != vault_stake_info.key {
                msg!("Error: Invalid Vault Stake Info account");
                return Err(ProgramError::InvalidArgument);
            }
            if vault_stake_custody_key.is_none()
                || &vault_stake_custody_key.unwrap() != vault_stake_custody.key
            {
                msg!("Error: Invalid Vault Stake Custody account");
                return Err(ProgramError::InvalidArgument);
            }
            if &token_a_reward_custody_key != reward_token_custody.key
                || &lp_token_custody_key != lp_token_custody.key
            {
                msg!("Error: Invalid custody accounts");
                return Err(ProgramError::InvalidArgument);
            }
            if &token_a_custody_key != token_a_custody.key
                || &token_b_custody_key.or_else(|| Some(zero::id())).unwrap() != token_b_custody.key
            {
                msg!("Error: Invalid custody accounts");
                return Err(ProgramError::InvalidArgument);
            }
            if amm_id.key != &pool_id_key {
                msg!("Error: Invalid pool id");
                return Err(ProgramError::InvalidArgument);
            }
            if farm_id.key != &farm_id_key {
                msg!("Error: Invalid farm id");
                return Err(ProgramError::InvalidArgument);
            }
        } else {
            msg!("Error: Vault strategy mismatch");
            return Err(ProgramError::InvalidArgument);
        }

        let mut vault_info = VaultInfo::new(vault_info_account);
        common::check_min_crank_interval(&vault_info)?;
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
        let (_, max_token_a_deposit_amount, max_token_b_deposit_amount) =
            if custody_ratio >= pool_ratio {
                orca::get_pool_deposit_amounts(
                    pool_token_a_account,
                    pool_token_b_account,
                    lp_token_mint,
                    token_a_balance,
                    0,
                )?
            } else {
                orca::get_pool_deposit_amounts(
                    pool_token_a_account,
                    pool_token_b_account,
                    lp_token_mint,
                    0,
                    token_b_balance,
                )?
            };
        // one of the amounts can come out over the balance because ratios didn't reflect
        // deposited volume, while get_pool_deposit_amounts does include it.
        // in this case we just flip the side.
        let (min_lp_token_amount, max_token_a_deposit_amount, max_token_b_deposit_amount) =
            if max_token_b_deposit_amount > token_b_balance {
                orca::get_pool_deposit_amounts(
                    pool_token_a_account,
                    pool_token_b_account,
                    lp_token_mint,
                    0,
                    token_b_balance,
                )?
            } else if max_token_a_deposit_amount > token_a_balance {
                orca::get_pool_deposit_amounts(
                    pool_token_a_account,
                    pool_token_b_account,
                    lp_token_mint,
                    token_a_balance,
                    0,
                )?
            } else {
                (
                    orca::estimate_lp_tokens_amount(
                        lp_token_mint,
                        max_token_a_deposit_amount,
                        max_token_b_deposit_amount,
                        pool_token_a_balance,
                        pool_token_b_balance,
                    )?,
                    max_token_a_deposit_amount,
                    max_token_b_deposit_amount,
                )
            };

        msg!("Deposit tokens into the pool. min_lp_token_amount: {}, max_token_a_deposit_amount: {}, max_token_b_deposit_amount: {}",
                        min_lp_token_amount,
                        max_token_a_deposit_amount,
                        max_token_b_deposit_amount);
        if max_token_a_deposit_amount == 0
            || max_token_b_deposit_amount == 0
            || min_lp_token_amount < 2
        {
            msg!("Nothing to do: Tokens balance is not large enough");
            return Ok(());
        }

        orca::add_liquidity_with_seeds(
            &[
                vault_authority.clone(),
                token_a_custody.clone(),
                token_b_custody.clone(),
                lp_token_custody.clone(),
                pool_program_id.clone(),
                pool_token_a_account.clone(),
                pool_token_b_account.clone(),
                lp_token_mint.clone(),
                spl_token_program.clone(),
                amm_id.clone(),
                amm_authority.clone(),
            ],
            seeds,
            max_token_a_deposit_amount,
            max_token_b_deposit_amount,
            min_lp_token_amount,
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
        let lp_tokens_received = account::check_tokens_received(
            lp_token_custody,
            lp_token_balance,
            min_lp_token_amount,
        )?;
        msg!(
            "Stake LP tokens. tokens_a_spent: {}, tokens_b_spent: {}, lp_tokens_received: {}",
            tokens_a_spent,
            tokens_b_spent,
            lp_tokens_received
        );
        let reward_token_balance = account::get_token_balance(reward_token_custody)?;

        orca::stake_with_seeds(
            &[
                vault_authority.clone(),
                vault_stake_info.clone(),
                lp_token_custody.clone(),
                reward_token_custody.clone(),
                vault_stake_custody.clone(),
                farm_lp_token_mint.clone(),
                farm_program.clone(),
                base_token_vault.clone(),
                reward_token_vault.clone(),
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
        let token_rewards =
            account::get_balance_increase(reward_token_custody, reward_token_balance)?;
        msg!("Update Vault stats. token_rewards: {}", token_rewards,);
        vault_info.add_rewards(token_rewards, 0)?;
        vault_info.add_liquidity(tokens_a_spent, tokens_b_spent)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
