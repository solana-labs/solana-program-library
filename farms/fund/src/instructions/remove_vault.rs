//! Fund RemoveVault instruction handler

use {
    crate::{common, fund_info::FundInfo},
    solana_farm_sdk::{
        fund::{Fund, FundAssetType, FundVaultType},
        program::account,
        traits::Packed,
    },
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    },
};

pub fn remove_vault(
    fund: &Fund,
    accounts: &[AccountInfo],
    target_hash: u64,
    vault_type: FundVaultType,
) -> ProgramResult {
    #[allow(clippy::deprecated_cfg_attr)]
    #[cfg_attr(rustfmt, rustfmt_skip)]
    if let [
        admin_account,
        fund_metadata,
        fund_info_account,
        _multisig_account,
        fund_authority,
        _system_program,
        vaults_assets_info,
        vault_metadata_account
        ] = accounts
    {
        // validate params and accounts
        msg!("Validate state and accounts");
        let mut fund_info = FundInfo::new(fund_info_account);
        if fund_info.get_liquidation_start_time()? > 0 {
            msg!("Error: Fund is in liquidation state");
            return Err(ProgramError::Custom(516));
        }
        if fund_authority.key != &fund.fund_authority {
            msg!("Error: Invalid Fund authority account");
            return Err(ProgramError::Custom(517));
        }

        common::check_vault_account(
            &fund.fund_program_id,
            fund_metadata,
            vault_metadata_account,
            vault_type,
        )?;

        // close accounts
        msg!("Close vault accounts");
        account::close_system_account(
            admin_account,
            vault_metadata_account,
            &fund.fund_program_id,
        )?;

        // update assets tracking account if vault is not a farm
        if vault_type != FundVaultType::Farm {
            msg!("Update Fund assets account");
            let mut fund_assets = common::check_and_get_fund_assets_account(
                fund,
                vaults_assets_info,
                FundAssetType::Vault,
            )?;
            fund_assets.current_hash = 0;
            fund_assets.target_hash = target_hash;
            fund_assets.cycle_start_time = 0;
            fund_assets.cycle_end_time = 0;
            fund_assets.pack(*vaults_assets_info.try_borrow_mut_data()?)?;
        }

        // update fund stats
        msg!("Update Fund stats");
        fund_info.update_admin_action_time()
    } else {
        Err(ProgramError::NotEnoughAccountKeys)
    }
}
