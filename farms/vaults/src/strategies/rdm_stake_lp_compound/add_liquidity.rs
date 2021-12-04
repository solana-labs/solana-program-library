//! Add Liquidity to the Vault instruction handler

use {
    crate::{strategies::common, traits::AddLiquidity, user_info::UserInfo, vault_info::VaultInfo},
    solana_farm_sdk::{
        id::zero,
        instruction::vault::VaultInstruction,
        program::{account, pda, protocol::raydium},
        vault::Vault,
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
            vault_authority,
            spl_token_program,
            vault_token_mint,
            user_info_account,
            user_token_a_account,
            user_token_b_account,
            user_lp_token_account,
            user_vt_token_account,
            token_a_reward_custody,
            token_b_reward_custody,
            lp_token_custody,
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
            // validate accounts
            if vault_authority.key != &vault.vault_authority {
                msg!("Error: Invalid Vault accounts");
                return Err(ProgramError::InvalidArgument);
            }
            if &account::get_token_account_owner(user_vt_token_account)? != user_account.key {
                msg!("Error: Invalid VT token account owner");
                return Err(ProgramError::IllegalOwner);
            }
            common::check_custody_accounts(
                vault,
                lp_token_custody,
                vault_authority,
                vault_authority,
                token_a_reward_custody,
                token_b_reward_custody,
                vault_stake_info,
                false,
            )?;
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

            // calculate deposit amounts
            let (max_token_a_deposit_amount, max_token_b_deposit_amount) =
                raydium::get_pool_deposit_amounts(
                    pool_coin_token_account,
                    pool_pc_token_account,
                    amm_open_orders,
                    amm_id,
                    max_token_a_amount,
                    max_token_b_amount,
                )?;

            // Deposit tokens into the pool
            msg!("Deposit tokens into the pool. max_token_a_deposit_amount: {}, max_token_b_deposit_amount: {}", max_token_a_deposit_amount, max_token_b_deposit_amount);
            if max_token_a_deposit_amount == 0 || max_token_b_deposit_amount == 0 {
                msg!("Error: Zero deposit amount");
                return Err(ProgramError::InsufficientFunds);
            }
            raydium::add_liquidity(
                &[
                    user_account.clone(),
                    user_token_a_account.clone(),
                    user_token_b_account.clone(),
                    user_lp_token_account.clone(),
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
                max_token_a_deposit_amount,
                max_token_b_deposit_amount,
            )?;

            // check amounts spent and received
            let tokens_a_spent = account::check_tokens_spent(
                user_token_a_account,
                initial_token_a_user_balance,
                max_token_a_deposit_amount,
            )?;
            let tokens_b_spent = account::check_tokens_spent(
                user_token_b_account,
                initial_token_b_user_balance,
                max_token_b_deposit_amount,
            )?;
            let lp_tokens_received =
                account::check_tokens_received(user_lp_token_account, initial_lp_user_balance, 1)?;
            let initial_lp_token_custody_balance = account::get_token_balance(lp_token_custody)?;

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

            // Stake LP tokens
            let seeds: &[&[&[u8]]] = &[&[
                b"vault_authority",
                vault.name.as_bytes(),
                &[vault.authority_bump],
            ]];

            let dual_rewards = *farm_reward_token_b_account.key != zero::id();
            let initial_token_a_reward_balance =
                account::get_token_balance(token_a_reward_custody)?;
            let initial_token_b_reward_balance = if dual_rewards {
                account::get_token_balance(token_b_reward_custody)?
            } else {
                0
            };

            msg!("Stake LP tokens");
            let stake_balance = raydium::get_stake_account_balance(vault_stake_info)?;

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
            if initial_lp_token_custody_balance != account::get_token_balance(lp_token_custody)? {
                msg!(
                    "Error: Stake instruction didn't result in expected amount of LP tokens spent"
                );
                return Err(ProgramError::Custom(165));
            }

            // update user stats
            msg!("Update user stats");
            let mut user_info = UserInfo::new(user_info_account);
            user_info.add_liquidity(tokens_a_spent, tokens_b_spent)?;

            // update Vault stats
            let token_a_rewards = account::get_balance_increase(
                token_a_reward_custody,
                initial_token_a_reward_balance,
            )?;
            let token_b_rewards = if dual_rewards {
                account::get_balance_increase(
                    token_b_reward_custody,
                    initial_token_b_reward_balance,
                )?
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

            // compute Vault tokens to mint
            let vt_supply_amount = account::get_token_supply(vault_token_mint)?;
            let vt_to_mint = if vt_supply_amount == 0 || stake_balance == 0 {
                lp_tokens_received
            } else {
                account::to_token_amount(
                    lp_tokens_received as f64 / stake_balance as f64 * vt_supply_amount as f64,
                    0,
                )?
            };

            // mint vault tokens to user
            msg!(
                "Mint Vault tokens to the user. vt_to_mint: {}, vt_supply_amount: {}, stake_balance: {}",
                vt_to_mint, vt_supply_amount,
                stake_balance
            );
            if vt_to_mint == 0 {
                msg!("Error: Add liquidity instruction didn't result in Vault tokens mint");
                return Err(ProgramError::Custom(170));
            }
            pda::mint_to_with_seeds(
                user_vt_token_account,
                vault_token_mint,
                vault_authority,
                seeds,
                vt_to_mint,
            )?;

            Ok(())
        } else {
            Err(ProgramError::NotEnoughAccountKeys)
        }
    }
}
