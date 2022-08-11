//! Crank step 1 instruction handler

use {
    crate::{strategies::common, vault_info::VaultInfo},
    solana_farm_sdk::{
        math,
        program::{account, pda, protocol::orca},
        vault::{Vault, VaultStrategy},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn crank1(vault: &Vault, accounts: &[AccountInfo]) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _funding_account,
        _vault_metadata,
        vault_info_account,
        vault_authority,
        spl_token_program,
        reward_token_custody,
        fees_account,
        farm_program,
        vault_stake_info,
        farm_id,
        farm_authority,
        base_token_vault,
        reward_token_vault
        ] = accounts
    {
        // validate accounts
        if vault_authority.key != &vault.vault_authority {
            msg!("Error: Invalid Vault accounts");
            return Err(ProgramError::InvalidArgument);
        }
        if let VaultStrategy::StakeLpCompoundRewards {
            farm_id: farm_id_key,
            token_a_reward_custody: token_a_reward_custody_key,
            vault_stake_info: vault_stake_info_key,
            ..
        } = vault.strategy
        {
            if &vault_stake_info_key != vault_stake_info.key {
                msg!("Error: Invalid Vault Stake Info account");
                return Err(ProgramError::InvalidArgument);
            }
            if &token_a_reward_custody_key != reward_token_custody.key {
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

        if Some(*fees_account.key) != vault.fees_account_a {
            msg!("Error: Invalid fee accounts");
            return Err(ProgramError::InvalidArgument);
        }

        let mut vault_info = VaultInfo::new(vault_info_account);
        common::check_min_crank_interval(&vault_info)?;

        // harvest
        let seeds: &[&[&[u8]]] = &[&[
            b"vault_authority",
            vault.name.as_bytes(),
            &[vault.authority_bump],
        ]];

        let initial_reward_token_reward_balance = account::get_token_balance(reward_token_custody)?;

        msg!("Harvest rewards");
        orca::harvest_with_seeds(
            &[
                vault_authority.clone(),
                vault_stake_info.clone(),
                reward_token_custody.clone(),
                farm_program.clone(),
                base_token_vault.clone(),
                reward_token_vault.clone(),
                spl_token_program.clone(),
                farm_id.clone(),
                farm_authority.clone(),
            ],
            seeds,
        )?;

        // calculate rewards
        let token_rewards = account::get_balance_increase(
            reward_token_custody,
            initial_reward_token_reward_balance,
        )?;
        msg!("Rewards received. token_rewards: {}", token_rewards);

        // take fees
        let fee = vault_info.get_fee()?;
        if !(0.0..=1.0).contains(&fee) {
            msg!("Error: Invalid fee. fee: {}", fee);
            return Err(ProgramError::Custom(260));
        }
        let mut fees = math::checked_as_u64(token_rewards as f64 * fee)?;
        if fees == 0 && token_rewards > 0 {
            fees = 1;
        }

        msg!("Apply fees. fee: {}, fees: {}", fee, fees);
        pda::transfer_tokens_with_seeds(
            reward_token_custody,
            fees_account,
            vault_authority,
            seeds,
            fees,
        )?;

        // update Vault stats
        msg!("Update Vault stats",);
        vault_info.add_rewards(token_rewards, 0)?;
        vault_info.update_crank_time()?;
        vault_info.set_crank_step(1)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
