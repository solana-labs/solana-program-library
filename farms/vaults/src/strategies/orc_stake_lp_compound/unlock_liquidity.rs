//! Unlock Liquidity in the Vault instruction handler

use {
    crate::{traits::UnlockLiquidity, user_info::UserInfo, vault_info::VaultInfo},
    solana_farm_sdk::{
        instruction::vault::VaultInstruction,
        math,
        program::{account, protocol::orca},
        vault::{Vault, VaultStrategy},
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
            reward_token_custody,
            lp_token_custody,
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
            // validate accounts
            if vault_authority.key != &vault.vault_authority {
                msg!("Error: Invalid Vault accounts");
                return Err(ProgramError::InvalidArgument);
            }
            if !user_account.is_signer {
                return Err(ProgramError::MissingRequiredSignature);
            }
            if !account::check_token_account_owner(user_vt_token_account, user_account.key)? {
                msg!("Error: Invalid VT token account owner");
                return Err(ProgramError::IllegalOwner);
            }
            if let VaultStrategy::StakeLpCompoundRewards {
                farm_id: farm_id_key,
                lp_token_custody: lp_token_custody_key,
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
                if farm_id.key != &farm_id_key {
                    msg!("Error: Invalid farm id");
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

            // calculate amounts to unstake
            let vt_remove_amount = if amount > 0 {
                amount
            } else {
                account::get_token_balance(user_vt_token_account)?
            };
            let vt_supply_amount = account::get_token_supply(vault_token_mint)?;
            let stake_balance = account::get_token_balance(vault_stake_custody)?;

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
            let lp_remove_amount = math::checked_as_u64(math::checked_div(
                math::checked_mul(stake_balance as u128, vt_remove_amount as u128)?,
                vt_supply_amount as u128,
            )?)?;

            // unstake
            let seeds: &[&[&[u8]]] = &[&[
                b"vault_authority",
                vault.name.as_bytes(),
                &[vault.authority_bump],
            ]];

            let initial_reward_token_balance = account::get_token_balance(reward_token_custody)?;
            let initial_lp_tokens_balance = account::get_token_balance(lp_token_custody)?;

            msg!(
                "Unstake user's lp tokens. amount: {}, lp_remove_amount: {}",
                amount,
                lp_remove_amount
            );
            orca::unstake_with_seeds(
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
            let token_rewards =
                account::get_balance_increase(reward_token_custody, initial_reward_token_balance)?;
            msg!("Update Vault stats. token_rewards: {}", token_rewards,);
            vault_info.add_rewards(token_rewards, 0)?;

            // burn vault tokens
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
