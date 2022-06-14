//! Update Fund assets with custody balance instruction handler

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        fund::{Fund, FundAssetType, FundCustody},
        id::zero,
        math,
        program::{account, clock},
        token::Token,
        traits::Packed,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn update_assets_with_custody(fund: &Fund, accounts: &[AccountInfo]) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        _user_account,
        fund_metadata,
        fund_info_account,
        custodies_assets_info,
        vaults_assets_info,
        custody_account,
        custody_metadata,
        custody_token_metadata,
        oracle_account
        ] = accounts
    {
        // validate params and accounts
        msg!("Validate state and accounts");
        let mut fund_info = FundInfo::new(fund_info_account);
        if fund_info.get_liquidation_start_time()? > 0 {
            msg!("Error: Fund is in liquidation state");
            return Err(ProgramError::Custom(516));
        }

        let custody_token = account::unpack::<Token>(custody_token_metadata, "custody token")?;
        let custody = account::unpack::<FundCustody>(custody_metadata, "custody")?;
        if custody.is_vault_token {
            msg!("Nothing to do: Custody is for vault tokens and must be processed with UpdateAssetsWithVault");
            return Ok(());
        }
        common::check_custody_account(
            &fund.fund_program_id,
            fund_metadata.key,
            &custody_token,
            custody_token_metadata,
            custody_metadata,
            custody.custody_type,
            custody_account,
            None,
        )?;

        if oracle_account.key != &custody_token.oracle_account.unwrap_or_else(zero::id) {
            msg!("Error: Invalid oracle account");
            return Err(ProgramError::Custom(531));
        }

        // update assets tracking account
        msg!("Update Fund assets account");
        let mut fund_custodies_assets = common::check_and_get_fund_assets_account(
            fund,
            custodies_assets_info,
            FundAssetType::Custody,
        )?;

        if fund_custodies_assets.target_hash == 0 {
            msg!("Error: target_hash is 0. Custodies must be added before updating assets.");
            return Err(ProgramError::InvalidAccountData);
        } else if custody.custody_id == 0 {
            fund_custodies_assets.current_hash = 0;
            fund_custodies_assets.current_assets_usd = 0.0;
            fund_custodies_assets.current_cycle =
                math::checked_add(fund_custodies_assets.current_cycle, 1)?;
            fund_custodies_assets.cycle_start_time = clock::get_time()?;
            fund_custodies_assets.cycle_end_time = 0;
        } else if fund_custodies_assets.cycle_end_time != 0 {
            msg!("Error: Cycle has already ended. To reset start with custody_id 0.");
            return Err(ProgramError::InvalidAccountData);
        }

        // update running hash of processed custodies
        // this mechanism is used to verify that all custodies have been processed
        // before final number is recorded
        fund_custodies_assets.current_hash =
            math::hash_address(fund_custodies_assets.current_hash, custody_account.key);

        // update current assets value in usd
        fund_custodies_assets.current_assets_usd += account::get_asset_value_usd(
            account::get_token_balance(custody_account)?,
            custody_token.decimals,
            custody_token.oracle_type,
            oracle_account,
            fund_info.get_assets_max_price_error()?,
            fund_info.get_assets_max_price_age_sec()?,
        )?;

        // check if all custodies have been processed
        if fund_custodies_assets.current_hash == fund_custodies_assets.target_hash {
            fund_custodies_assets.cycle_end_time = clock::get_time()?;

            // if all vaults have been processed as well the cycle is complete
            let fund_vaults_assets = common::check_and_get_fund_assets_account(
                fund,
                vaults_assets_info,
                FundAssetType::Vault,
            )?;

            if fund_vaults_assets.cycle_end_time != 0 || fund_vaults_assets.target_hash == 0 {
                // update fund stats
                msg!("Update Fund stats");
                fund_info.set_current_assets_usd(
                    fund_custodies_assets.current_assets_usd
                        + fund_vaults_assets.current_assets_usd,
                )?;
                fund_info.set_assets_update_time(clock::get_time()?)?;
            }
        }

        fund_custodies_assets.pack(*custodies_assets_info.try_borrow_mut_data()?)?;

        Ok(())
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
