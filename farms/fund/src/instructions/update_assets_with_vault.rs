//! Update Fund assets with Vault balance instruction handler

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        fund::{Fund, FundAssetType, FundVault, FundVaultType, DISCRIMINATOR_FUND_VAULT},
        id::zero,
        math,
        pool::{Pool, PoolRoute},
        program,
        program::{
            account, clock,
            protocol::{orca, raydium},
        },
        token::Token,
        traits::Packed,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn update_assets_with_vault(fund: &Fund, accounts: &[AccountInfo]) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _user_account,
        fund_metadata,
        fund_info_account,
        custodies_assets_info,
        vaults_assets_info,
        vault_metadata_account,
        vault_info_account,
        underlying_pool_ref,
        pool_token_a_ref,
        pool_token_b_ref,
        underlying_lp_token_mint,
        pool_token_a_account,
        pool_token_b_account,
        amm_id,
        amm_open_orders,
        oracle_account_token_a,
        oracle_account_token_b,
        sysvar_account
        ] = accounts
    {
        // validate params and accounts
        msg!("Validate state and accounts");
        let mut fund_info = FundInfo::new(fund_info_account);
        if fund_info.get_liquidation_start_time()? > 0 {
            msg!("Error: Fund is in liquidation state");
            return Err(ProgramError::Custom(516));
        }

        // unpack and validate Vault metadata
        if vault_metadata_account.owner != &fund.fund_program_id {
            msg!("Error: Invalid custody owner");
            return Err(ProgramError::IllegalOwner);
        }
        let vault = account::unpack::<FundVault>(vault_metadata_account, "Vault")?;
        if &vault.fund_ref != fund_metadata.key {
            msg!("Error: Specified Vault doesn't belong to this Fund");
            return Err(ProgramError::Custom(507));
        }
        if vault.discriminator != DISCRIMINATOR_FUND_VAULT
            || &vault.underlying_pool_ref != underlying_pool_ref.key
            || &vault.underlying_lp_token_mint != underlying_lp_token_mint.key
        {
            msg!("Error: Invalid Vault metadata account");
            return Err(ProgramError::Custom(506));
        }
        match vault.vault_type {
            FundVaultType::Vault => {
                if vault_info_account.key != &vault.vault_ref {
                    msg!("Error: Invalid vault info account");
                    return Err(ProgramError::Custom(532));
                }
            }
            FundVaultType::Pool => {
                if vault_info_account.key != &vault.underlying_pool_ref {
                    msg!("Error: Invalid vault info account");
                    return Err(ProgramError::Custom(532));
                }
            }
            FundVaultType::Farm => {
                msg!("Nothing to do: Farms are not processed to avoid double counting");
                return Ok(());
            }
        }

        if !program::is_single_instruction(sysvar_account)? {
            msg!("Error: UpdateAssetsWithVault must be single instruction in the transaction");
            return Err(ProgramError::InvalidArgument);
        }

        // unpack and validate underlying pool
        let pool = account::unpack::<Pool>(underlying_pool_ref, "underlying Pool")?;
        if pool.token_a_ref.is_none()
            || pool.token_b_ref.is_none()
            || pool.token_a_account.is_none()
            || pool.token_b_account.is_none()
            || &pool.token_a_ref.unwrap() != pool_token_a_ref.key
            || &pool.token_b_ref.unwrap() != pool_token_b_ref.key
            || &pool.token_a_account.unwrap() != pool_token_a_account.key
            || &pool.token_b_account.unwrap() != pool_token_b_account.key
        {
            msg!("Error: Invalid Pool metadata account");
            return Err(ProgramError::Custom(533));
        }

        match pool.route {
            PoolRoute::Raydium {
                amm_id: amm_id_key,
                amm_open_orders: amm_open_orders_key,
                ..
            } => {
                if &amm_open_orders_key != amm_open_orders.key || &amm_id_key != amm_id.key {
                    msg!("Error: Invalid Pool route metadata");
                    return Err(ProgramError::Custom(534));
                }
            }
            PoolRoute::Orca {
                amm_id: amm_id_key, ..
            } => {
                if &zero::id() != amm_open_orders.key || &amm_id_key != amm_id.key {
                    msg!("Error: Invalid Pool route metadata");
                    return Err(ProgramError::Custom(534));
                }
            }
            _ => {
                msg!("Error: Unsupported Pool route");
                return Err(ProgramError::Custom(522));
            }
        }

        // unpack pool tokens
        let token_a = account::unpack::<Token>(pool_token_a_ref, "token_a")?;
        let token_b = account::unpack::<Token>(pool_token_b_ref, "token_b")?;
        if &token_a.oracle_account.unwrap_or_else(zero::id) != oracle_account_token_a.key
            || &token_b.oracle_account.unwrap_or_else(zero::id) != oracle_account_token_b.key
        {
            msg!("Error: Invalid oracle accounts");
            return Err(ProgramError::Custom(531));
        }

        // update assets tracking account
        msg!("Update Fund assets account");
        let mut fund_vaults_assets = common::check_and_get_fund_assets_account(
            fund,
            vaults_assets_info,
            FundAssetType::Vault,
        )?;

        if fund_vaults_assets.target_hash == 0 {
            msg!("Error: target_hash is 0. Vaults must be added before updating assets.");
            return Err(ProgramError::Custom(535));
        } else if vault.vault_id == 0 {
            fund_vaults_assets.current_hash = 0;
            fund_vaults_assets.current_assets_usd = 0.0;
            fund_vaults_assets.current_cycle =
                math::checked_add(fund_vaults_assets.current_cycle, 1)?;
            fund_vaults_assets.cycle_start_time = clock::get_time()?;
            fund_vaults_assets.cycle_end_time = 0;
        } else if fund_vaults_assets.cycle_end_time != 0 {
            msg!("Error: Cycle has already ended. To reset start with vault_id 0.");
            return Err(ProgramError::Custom(536));
        }

        // update running hash of processed vaults
        // this mechanism is used to verify that all vaults have been processed
        // before final number is recorded
        fund_vaults_assets.current_hash =
            math::hash_address(fund_vaults_assets.current_hash, &vault.vault_ref);

        if vault.lp_balance > 0 {
            // compute vault balances
            let (potential_token_a_balance, potential_token_b_balance) = match pool.route {
                PoolRoute::Raydium { .. } => raydium::get_pool_withdrawal_amounts(
                    pool_token_a_account,
                    pool_token_b_account,
                    amm_open_orders,
                    amm_id,
                    underlying_lp_token_mint,
                    vault.lp_balance,
                )?,
                PoolRoute::Orca { .. } => orca::get_pool_withdrawal_amounts(
                    pool_token_a_account,
                    pool_token_b_account,
                    underlying_lp_token_mint,
                    vault.lp_balance,
                )?,
                _ => {
                    msg!("Error: Invalid Pool route");
                    return Err(ProgramError::Custom(522));
                }
            };

            // update current assets value in usd
            fund_vaults_assets.current_assets_usd += account::get_asset_value_usd(
                potential_token_a_balance,
                token_a.decimals,
                token_a.oracle_type,
                oracle_account_token_a,
                fund_info.get_assets_max_price_error()?,
                fund_info.get_assets_max_price_age_sec()?,
            )?;

            fund_vaults_assets.current_assets_usd += account::get_asset_value_usd(
                potential_token_b_balance,
                token_b.decimals,
                token_b.oracle_type,
                oracle_account_token_b,
                fund_info.get_assets_max_price_error()?,
                fund_info.get_assets_max_price_age_sec()?,
            )?;
        }

        // check if all vaults have been processed
        if fund_vaults_assets.current_hash == fund_vaults_assets.target_hash {
            fund_vaults_assets.cycle_end_time = clock::get_time()?;

            // if all custodies have been processed as well the cycle is complete
            let fund_custodies_assets = common::check_and_get_fund_assets_account(
                fund,
                custodies_assets_info,
                FundAssetType::Custody,
            )?;

            if fund_custodies_assets.cycle_end_time != 0 || fund_custodies_assets.target_hash == 0 {
                // update fund stats
                msg!("Update Fund stats");
                fund_info.set_current_assets_usd(
                    fund_custodies_assets.current_assets_usd
                        + fund_vaults_assets.current_assets_usd,
                )?;
                fund_info.set_assets_update_time(clock::get_time()?)?;
            }
        }

        fund_vaults_assets.pack(*vaults_assets_info.try_borrow_mut_data()?)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
