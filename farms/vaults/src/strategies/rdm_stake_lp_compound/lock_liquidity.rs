//! Lock Liquidity in the Vault instruction handler

use {
    crate::{
        strategies::common, traits::LockLiquidity, user_info::UserInfo, vault_info::VaultInfo,
    },
    solana_farm_sdk::{
        id::zero,
        instruction::vault::VaultInstruction,
        math,
        program::{account, pda, protocol::raydium},
        vault::Vault,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

impl LockLiquidity for VaultInstruction {
    fn lock_liquidity(vault: &Vault, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
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
            user_vt_token_account,
            token_a_reward_custody,
            token_b_reward_custody,
            lp_token_custody,
            farm_program,
            vault_stake_info,
            farm_id,
            farm_authority,
            farm_lp_token_account,
            farm_first_reward_token_account,
            farm_second_reward_token_account,
            clock_program
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
            if !account::check_token_account_owner(user_vt_token_account,user_account.key)? {
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
                None,
                Some(farm_id.key),
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

            // check lp balance
            let mut user_info = UserInfo::new(user_info_account);
            let lp_tokens_debt = user_info.get_lp_tokens_debt()?;
            msg!("Read balances. lp_tokens_debt: {}", lp_tokens_debt);

            let lp_stake_amount = if amount > 0 {
                if lp_tokens_debt < amount {
                    msg!("Error: Insufficient funds");
                    return Err(ProgramError::InsufficientFunds);
                }
                amount
            } else {
                lp_tokens_debt
            };
            if lp_stake_amount == 0 {
                msg!("Error: Zero balance. Forgot to deposit funds?");
                return Err(ProgramError::InsufficientFunds);
            }

            let initial_lp_custody_balance = account::get_token_balance(lp_token_custody)?;

            // Stake LP tokens
            let seeds: &[&[&[u8]]] = &[&[
                b"vault_authority",
                vault.name.as_bytes(),
                &[vault.authority_bump],
            ]];

            let dual_rewards = *farm_second_reward_token_account.key != zero::id();
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
                    farm_first_reward_token_account.clone(),
                    farm_second_reward_token_account.clone(),
                    clock_program.clone(),
                    spl_token_program.clone(),
                    farm_id.clone(),
                    farm_authority.clone(),
                ],
                seeds,
                lp_stake_amount,
            )?;
            let _ = account::check_tokens_spent(
                lp_token_custody,
                initial_lp_custody_balance,
                lp_stake_amount,
            )?;

            // update user stats
            msg!("Update user stats");
            user_info.remove_lp_tokens_debt(lp_stake_amount)?;

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

            // compute Vault tokens to mint
            let vt_supply_amount = account::get_token_supply(vault_token_mint)?;
            let vt_to_mint = if vt_supply_amount == 0 || stake_balance == 0 {
                lp_stake_amount
            } else {
                math::checked_as_u64(math::checked_div(
               math::checked_mul(lp_stake_amount as u128, vt_supply_amount as u128)?,
               stake_balance as u128
                )?)?
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
