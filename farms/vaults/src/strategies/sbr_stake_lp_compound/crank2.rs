//! Crank step 2 instruction handler

use {
    crate::{clock::check_min_crank_interval, vault_info::VaultInfo},
    solana_farm_sdk::{
        id::zero,
        program::{account, pda, protocol::saber},
        vault::{Vault, VaultStrategy},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn crank2(vault: &Vault, accounts: &[AccountInfo]) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _funding_account,
        _vault_metadata,
        vault_info_account,
        vault_authority,
        spl_token_program,
        sbr_token_reward_custody,
        iou_token_reward_custody,
        fees_account_sbr,
        redeemer,
        redeemer_program,
        sbr_token_mint,
        iou_token_mint,
        sbr_vault,
        mint_proxy_program,
        mint_proxy_authority,
        mint_proxy_state,
        minter_info
        ] = accounts
    {
        // validate accounts
        if vault_authority.key != &vault.vault_authority {
            msg!("Error: Invalid Vault accounts");
            return Err(ProgramError::InvalidArgument);
        }
        if let VaultStrategy::StakeLpCompoundRewards {
            token_a_reward_custody: sbr_token_reward_custody_key,
            token_b_reward_custody: iou_token_reward_custody_key,
            ..
        } = vault.strategy
        {
            if &sbr_token_reward_custody_key != sbr_token_reward_custody.key
                || &iou_token_reward_custody_key.or(Some(zero::id())).unwrap()
                    != iou_token_reward_custody.key
            {
                msg!("Error: Invalid custody accounts");
                return Err(ProgramError::InvalidArgument);
            }
        } else {
            msg!("Error: Vault strategy mismatch");
            return Err(ProgramError::InvalidArgument);
        }

        if Some(*fees_account_sbr.key) != vault.fees_account_a {
            msg!("Error: Invalid fee account");
            return Err(ProgramError::InvalidArgument);
        }

        let mut vault_info = VaultInfo::new(vault_info_account);
        check_min_crank_interval(&vault_info)?;

        // redeem rewards
        let seeds: &[&[&[u8]]] = &[&[
            b"vault_authority",
            vault.name.as_bytes(),
            &[vault.authority_bump],
        ]];

        let initial_sbr_tokens_balance = account::get_token_balance(sbr_token_reward_custody)?;
        let iou_tokens_balance = account::get_token_balance(iou_token_reward_custody)?;

        msg!("Redeem rewards: {}", iou_tokens_balance);
        if iou_tokens_balance < 10 {
            msg!("Nothing to do: Not enough tokens to redeem");
            return Ok(());
        }
        saber::redeem_rewards_with_seeds(
            &[
                vault_authority.clone(),
                iou_token_reward_custody.clone(),
                sbr_token_reward_custody.clone(),
                spl_token_program.clone(),
                redeemer.clone(),
                redeemer_program.clone(),
                sbr_token_mint.clone(),
                iou_token_mint.clone(),
                sbr_vault.clone(),
                mint_proxy_program.clone(),
                mint_proxy_authority.clone(),
                mint_proxy_state.clone(),
                minter_info.clone(),
            ],
            seeds,
        )?;
        let _ = account::check_tokens_received(
            sbr_token_reward_custody,
            initial_sbr_tokens_balance,
            iou_tokens_balance,
        )?;

        // take fees
        let fee = vault_info.get_fee()?;
        if fee < 0.0 || fee > 1.0 {
            msg!("Error: Invalid fee. fee: {}", fee);
            return Err(ProgramError::Custom(260));
        }
        let sbr_fees = account::to_token_amount(iou_tokens_balance as f64 * fee, 0)?;

        msg!("Apply fees. fee: {}, sbr_fees: {}", fee, sbr_fees);
        pda::transfer_tokens_with_seeds(
            sbr_token_reward_custody,
            fees_account_sbr,
            vault_authority,
            seeds,
            sbr_fees,
        )?;

        // update Vault stats
        msg!("Update Vault stats",);
        vault_info.add_rewards(iou_tokens_balance, 0)?;
        vault_info.update_crank_time()?;
        vault_info.set_crank_step(2)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
