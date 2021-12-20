//! Unlock Liquidity in the Vault instruction handler

use {
    crate::{
        strategies::common, traits::UnlockLiquidity, user_info::UserInfo, vault_info::VaultInfo,
    },
    solana_farm_sdk::{
        id::zero,
        instruction::vault::VaultInstruction,
        program::{account, protocol::raydium},
        vault::Vault,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
        pubkey::Pubkey,
    },
};

impl UnlockLiquidity for VaultInstruction {
    fn unlock_liquidity(vault: &Vault, accounts: &[AccountInfo], amount: u64) -> ProgramResult {
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
            if !user_account.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
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
            if !vault_info.is_withdrawal_allowed()? {
                msg!("Error: Withdrawals are not allowed for this Vault");
                return Err(ProgramError::Custom(230));
            }

            // calculate amounts to unstake
            let vt_remove_amount = if amount > 0 {
                amount
            } else {
                account::get_token_balance(user_vt_token_account)?
            };
            let vt_supply_amount = account::get_token_supply(vault_token_mint)?;
            let stake_balance = raydium::get_stake_account_balance(vault_stake_info)?;

            msg!(
                "Read balances. vt_remove_amount: {}, vt_supply_amount: {}, stake_balance: {}",
                vt_remove_amount,
                vt_supply_amount,
                stake_balance
            );
            if vt_remove_amount == 0 || vt_supply_amount == 0 || stake_balance == 0 {
                msg!("Error: Zero balance");
                return Err(ProgramError::InsufficientFunds);
            }
            let lp_remove_amount = account::to_token_amount(
                stake_balance as f64 * (vt_remove_amount as f64 / vt_supply_amount as f64),
                0,
            )?;

            // unstake
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
            let initial_lp_tokens_balance = account::get_token_balance(lp_token_custody)?;

            msg!(
                "Unstake user's lp tokens. amount: {}, lp_remove_amount: {}",
                amount,
                lp_remove_amount
            );
            raydium::unstake_with_seeds(
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
                lp_remove_amount,
            )?;
            let _ = account::check_tokens_received(
                lp_token_custody,
                initial_lp_tokens_balance,
                lp_remove_amount,
            )?;

            // update user stats
            msg!("Update user stats");
            let mut user_info = UserInfo::new(user_info_account);
            user_info.add_lp_tokens_debt(lp_remove_amount)?;

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

            // brun vault tokens
            msg!(
                "Burn Vault tokens from the user. vt_remove_amount: {}",
                vt_remove_amount
            );
            let key = Pubkey::create_program_address(
                &[
                    b"vault_token_mint",
                    vault.name.as_bytes(),
                    &[vault.vault_token_bump],
                ],
                &vault.vault_program_id,
            )?;
            if vault_token_mint.key != &key {
                msg!("Error: Invalid Vault token mint");
                return Err(ProgramError::InvalidSeeds);
            }
            account::burn_tokens(
                user_vt_token_account,
                vault_token_mint,
                user_account,
                vt_remove_amount,
            )?;

            Ok(())
        } else {
            Err(ProgramError::NotEnoughAccountKeys)
        }
    }
}
