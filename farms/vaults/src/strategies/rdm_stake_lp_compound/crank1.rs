//! Crank step 1 instruction handler

use {
    crate::{clock::check_min_crank_interval, strategies::common, vault_info::VaultInfo},
    solana_farm_sdk::{
        id::zero,
        program::{account, pda, protocol::raydium},
        vault::Vault,
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
        token_a_reward_custody,
        token_b_reward_custody,
        lp_token_custody,
        fees_account_a,
        fees_account_b,
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

        let dual_rewards = *farm_reward_token_b_account.key != zero::id();

        if Some(*fees_account_a.key) != vault.fees_account_a
            || (dual_rewards && Some(*fees_account_b.key) != vault.fees_account_b)
        {
            msg!("Error: Invalid fee accounts");
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

        let initial_token_a_reward_balance = account::get_token_balance(token_a_reward_custody)?;
        let initial_token_b_reward_balance = if dual_rewards {
            account::get_token_balance(token_b_reward_custody)?
        } else {
            0
        };
        let initial_lp_tokens_balance = account::get_token_balance(lp_token_custody)?;

        msg!("Harvest rewards");
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
            0,
        )?;
        let _ = account::check_tokens_spent(lp_token_custody, initial_lp_tokens_balance, 0)?;

        // calculate rewards
        let token_a_rewards =
            account::get_balance_increase(token_a_reward_custody, initial_token_a_reward_balance)?;
        let token_b_rewards = if dual_rewards {
            account::get_balance_increase(token_b_reward_custody, initial_token_b_reward_balance)?
        } else {
            0
        };
        msg!(
            "Rewards received. token_a_rewards: {}, token_b_rewards: {}",
            token_a_rewards,
            token_b_rewards
        );
        // take fees
        let fee = vault_info.get_fee()?;
        if !(0.0..=1.0).contains(&fee) {
            msg!("Error: Invalid fee. fee: {}", fee);
            return Err(ProgramError::Custom(260));
        }
        let fees_a = account::to_token_amount(token_a_rewards as f64 * fee, 0)?;
        let fees_b = account::to_token_amount(token_b_rewards as f64 * fee, 0)?;
        msg!(
            "Apply fees. fee: {}, fees_a: {}, fees_b: {}",
            fee,
            fees_a,
            fees_b
        );
        pda::transfer_tokens_with_seeds(
            token_a_reward_custody,
            fees_account_a,
            vault_authority,
            seeds,
            fees_a,
        )?;
        if dual_rewards {
            pda::transfer_tokens_with_seeds(
                token_b_reward_custody,
                fees_account_b,
                vault_authority,
                seeds,
                fees_b,
            )?;
        }

        // update Vault stats
        msg!("Update Vault stats",);
        vault_info.add_rewards(token_a_rewards, token_b_rewards)?;
        vault_info.update_crank_time()?;
        vault_info.set_crank_step(1)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
