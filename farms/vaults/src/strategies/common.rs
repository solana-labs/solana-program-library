//! Common functions

use {
    solana_farm_sdk::{
        id::zero,
        vault::{Vault, VaultStrategy},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
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
    check_non_reward_custody: bool,
) -> ProgramResult {
    if let VaultStrategy::StakeLpCompoundRewards {
        pool_id_ref: _,
        farm_id_ref: _,
        lp_token_custody: lp_token_custody_key,
        token_a_custody: token_a_custody_key,
        token_b_custody: token_b_custody_key,
        token_a_reward_custody: token_a_reward_custody_key,
        token_b_reward_custody: token_b_reward_custody_key,
        vault_stake_info: vault_stake_info_key,
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
    } else {
        msg!("Error: Vault strategy mismatch");
        return Err(ProgramError::InvalidArgument);
    }
    Ok(())
}
