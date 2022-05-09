//! Common functions

use {
    crate::vault_info::VaultInfo,
    solana_farm_sdk::{
        id::zero,
        program::clock,
        vault::{Vault, VaultStrategy},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
        pubkey::Pubkey,
    },
    std::cmp,
};

#[allow(clippy::too_many_arguments)]
pub fn check_custody_accounts<'a, 'b>(
    vault: &Vault,
    lp_token_custody: &'a AccountInfo<'b>,
    token_a_custody: &'a AccountInfo<'b>,
    token_b_custody: &'a AccountInfo<'b>,
    token_a_reward_custody: &'a AccountInfo<'b>,
    token_b_reward_custody: &'a AccountInfo<'b>,
    vault_stake_info: &'a AccountInfo<'b>,
    pool_id: Option<&Pubkey>,
    farm_id: Option<&Pubkey>,
    check_non_reward_custody: bool,
) -> ProgramResult {
    if let VaultStrategy::StakeLpCompoundRewards {
        pool_id: pool_id_key,
        farm_id: farm_id_key,
        lp_token_custody: lp_token_custody_key,
        token_a_custody: token_a_custody_key,
        token_b_custody: token_b_custody_key,
        token_a_reward_custody: token_a_reward_custody_key,
        token_b_reward_custody: token_b_reward_custody_key,
        vault_stake_info: vault_stake_info_key,
        ..
    } = vault.strategy
    {
        if &vault_stake_info_key != vault_stake_info.key {
            msg!("Error: Invalid Vault Stake Info account");
            return Err(ProgramError::InvalidArgument);
        }
        if &token_a_reward_custody_key != token_a_reward_custody.key
            || &token_b_reward_custody_key
                .or_else(|| Some(zero::id()))
                .unwrap()
                != token_b_reward_custody.key
            || &lp_token_custody_key != lp_token_custody.key
        {
            msg!("Error: Invalid custody accounts");
            return Err(ProgramError::InvalidArgument);
        }
        if check_non_reward_custody
            && (&token_a_custody_key != token_a_custody.key
                || &token_b_custody_key.or_else(|| Some(zero::id())).unwrap()
                    != token_b_custody.key)
        {
            msg!("Error: Invalid custody accounts");
            return Err(ProgramError::InvalidArgument);
        }
        if let Some(pool_id) = pool_id {
            if pool_id != &pool_id_key {
                msg!("Error: Invalid pool id");
                return Err(ProgramError::InvalidArgument);
            }
        }
        if let Some(farm_id) = farm_id {
            if farm_id != &farm_id_key {
                msg!("Error: Invalid farm id");
                return Err(ProgramError::InvalidArgument);
            }
        }
    } else {
        msg!("Error: Vault strategy mismatch");
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}

pub fn check_min_crank_interval(vault_info: &VaultInfo) -> ProgramResult {
    let min_crank_interval = vault_info.get_min_crank_interval()?;
    if min_crank_interval == 0 {
        return Ok(());
    }
    let last_crank_time = vault_info.get_crank_time()?;
    let cur_time = cmp::max(clock::get_time()?, last_crank_time);
    if cur_time < last_crank_time.wrapping_add(min_crank_interval) {
        msg!(
            "Error: Too early, please wait for the additional {} sec",
            last_crank_time
                .wrapping_add(min_crank_interval)
                .wrapping_sub(cur_time)
        );
        Err(ProgramError::Custom(309))
    } else {
        Ok(())
    }
}
