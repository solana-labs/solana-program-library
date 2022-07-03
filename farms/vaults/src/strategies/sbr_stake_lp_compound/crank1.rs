//! Crank step 1 instruction handler

use {
    crate::{clock::check_min_crank_interval, vault_info::VaultInfo},
    solana_farm_sdk::{
        program::{account, protocol::saber},
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
        iou_token_reward_custody,
        farm_program,
        vault_stake_info,
        mint_wrapper,
        mint_wrapper_program,
        minter,
        iou_token_mint,
        iou_fees_account,
        quarry,
        rewarder,
        zero_id
        ] = accounts
    {
        // validate accounts
        if vault_authority.key != &vault.vault_authority {
            msg!("Error: Invalid Vault accounts");
            return Err(ProgramError::InvalidArgument);
        }
        if let VaultStrategy::StakeLpCompoundRewards {
            token_b_reward_custody: token_b_reward_custody_key,
            vault_stake_info: vault_stake_info_key,
            ..
        } = vault.strategy
        {
            if &vault_stake_info_key != vault_stake_info.key {
                msg!("Error: Invalid Vault Stake Info account");
                return Err(ProgramError::InvalidArgument);
            }
            if token_b_reward_custody_key != Some(*iou_token_reward_custody.key) {
                msg!("Error: Invalid custody accounts");
                return Err(ProgramError::InvalidArgument);
            }
        } else {
            msg!("Error: Vault strategy mismatch");
            return Err(ProgramError::InvalidArgument);
        }

        let mut vault_info = VaultInfo::new(vault_info_account);
        check_min_crank_interval(&vault_info)?;

        // harvest
        let seeds: &[&[&[u8]]] = &[&[
            b"vault_authority",
            vault.name.as_bytes(),
            &[vault.authority_bump],
        ]];

        let initial_iou_token_reward_balance =
            account::get_token_balance(iou_token_reward_custody)?;

        msg!("Claim rewards");
        saber::claim_rewards_with_seeds(
            &[
                vault_authority.clone(),
                iou_token_reward_custody.clone(),
                farm_program.clone(),
                spl_token_program.clone(),
                zero_id.clone(),
                vault_stake_info.clone(),
                rewarder.clone(),
                minter.clone(),
                mint_wrapper.clone(),
                mint_wrapper_program.clone(),
                iou_token_mint.clone(),
                iou_fees_account.clone(),
                quarry.clone(),
            ],
            seeds,
        )?;
        // calculate rewards
        let iou_token_rewards = account::get_balance_increase(
            iou_token_reward_custody,
            initial_iou_token_reward_balance,
        )?;

        msg!("Rewards received. iou_token_rewards: {}", iou_token_rewards);

        // update Vault stats
        msg!("Update Vault stats",);
        vault_info.add_rewards(0, iou_token_rewards)?;
        vault_info.update_crank_time()?;
        vault_info.set_crank_step(1)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
