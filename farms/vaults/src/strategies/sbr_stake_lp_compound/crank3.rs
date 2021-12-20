//! Crank step 3 instruction handler

use {
    crate::{clock::check_min_crank_interval, vault_info::VaultInfo},
    solana_farm_sdk::{
        id::zero,
        program::{
            account,
            protocol::{raydium, saber},
        },
        vault::{Vault, VaultStrategy},
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn crank3(vault: &Vault, accounts: &[AccountInfo]) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _funding_account,
        _vault_metadata,
        vault_info_account,
        vault_authority,
        spl_token_program,
        sbr_token_custody,
        usdc_token_custody,
        wrapped_token_custody,
        usdc_token_mint,
        wrapped_token_mint,
        wrapped_token_vault,
        decimal_wrapper,
        decimal_wrapper_program,
        pool_program_id,
        pool_coin_token_account,
        pool_pc_token_account,
        amm_id,
        amm_authority,
        amm_open_orders,
        amm_target,
        serum_market,
        serum_program_id,
        serum_coin_vault_account,
        serum_pc_vault_account,
        serum_vault_signer,
        serum_bids,
        serum_asks,
        serum_event_queue
        ] = accounts
    {
        // validate accounts
        if vault_authority.key != &vault.vault_authority {
            msg!("Error: Invalid Vault accounts");
            return Err(ProgramError::InvalidArgument);
        }
        if let VaultStrategy::StakeLpCompoundRewards {
            token_a_custody: usdc_token_custody_key,
            token_b_custody: wrapped_token_custody_key,
            token_a_reward_custody: sbr_token_custody_key,
            ..
        } = vault.strategy
        {
            if &usdc_token_custody_key != usdc_token_custody.key
                || &wrapped_token_custody_key.or(Some(zero::id())).unwrap()
                    != wrapped_token_custody.key
                || &sbr_token_custody_key != sbr_token_custody.key
            {
                msg!("Error: Invalid custody accounts");
                return Err(ProgramError::InvalidArgument);
            }
        } else {
            msg!("Error: Vault strategy mismatch");
            return Err(ProgramError::InvalidArgument);
        }

        let mut vault_info = VaultInfo::new(vault_info_account);
        check_min_crank_interval(&vault_info)?;
        vault_info.update_crank_time()?;
        vault_info.set_crank_step(3)?;

        // read balances
        let sbr_token_balance = account::get_token_balance(sbr_token_custody)?;
        let usdc_token_balance = account::get_token_balance(usdc_token_custody)?;
        msg!("SBR rewards balance: {}", sbr_token_balance);
        if sbr_token_balance < 10 {
            msg!("Nothing to do: Not enough SBR tokens to swap");
            return Ok(());
        }

        // move rewards to token custodies
        let seeds: &[&[&[u8]]] = &[&[
            b"vault_authority",
            vault.name.as_bytes(),
            &[vault.authority_bump],
        ]];

        msg!("Swap SBR to USDC");
        raydium::swap_with_seeds(
            &[
                vault_authority.clone(),
                sbr_token_custody.clone(),
                usdc_token_custody.clone(),
                pool_program_id.clone(),
                pool_coin_token_account.clone(),
                pool_pc_token_account.clone(),
                spl_token_program.clone(),
                amm_id.clone(),
                amm_authority.clone(),
                amm_open_orders.clone(),
                amm_target.clone(),
                serum_market.clone(),
                serum_program_id.clone(),
                serum_bids.clone(),
                serum_asks.clone(),
                serum_event_queue.clone(),
                serum_coin_vault_account.clone(),
                serum_pc_vault_account.clone(),
                serum_vault_signer.clone(),
            ],
            seeds,
            sbr_token_balance,
            1,
        )?;
        let _ =
            account::check_tokens_spent(sbr_token_custody, sbr_token_balance, sbr_token_balance)?;
        let usdc_tokens_received =
            account::check_tokens_received(usdc_token_custody, usdc_token_balance, 1)?;

        msg!("USDC tokens received: {}", usdc_tokens_received);

        if wrapped_token_mint.key != &zero::id() {
            msg!("Wrap USDC tokens");
            let initial_usdc_token_balance = account::get_token_balance(usdc_token_custody)?;
            let initial_wrapped_token_balance = account::get_token_balance(wrapped_token_custody)?;

            let usdc_decimals = account::get_token_decimals(&usdc_token_mint)?;
            let wrapped_decimals = account::get_token_decimals(&wrapped_token_mint)?;

            saber::wrap_token_with_seeds(
                decimal_wrapper,
                wrapped_token_mint,
                wrapped_token_vault,
                vault_authority,
                usdc_token_custody,
                wrapped_token_custody,
                decimal_wrapper_program.key,
                seeds,
                initial_usdc_token_balance,
            )?;

            account::check_tokens_received(
                wrapped_token_custody,
                initial_wrapped_token_balance,
                account::to_amount_with_new_decimals(
                    initial_usdc_token_balance,
                    usdc_decimals,
                    wrapped_decimals,
                )?,
            )?;
        }

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
