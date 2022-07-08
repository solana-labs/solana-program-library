//! Fund SetDepositSchedule instruction handler

use {
    crate::fund_info::FundInfo,
    solana_farm_sdk::fund::{Fund, FundAssetsTrackingConfig},
    solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg},
};

pub fn set_assets_tracking_config(
    _fund: &Fund,
    fund_info: &mut FundInfo,
    _accounts: &[AccountInfo],
    config: &FundAssetsTrackingConfig,
) -> ProgramResult {
    msg!("Update Fund assets tracking parameters");
    fund_info.set_assets_limit_usd(config.assets_limit_usd)?;
    fund_info.set_assets_max_update_age_sec(config.max_update_age_sec)?;
    fund_info.set_assets_max_price_error(config.max_price_error)?;
    fund_info.set_assets_max_price_age_sec(config.max_price_age_sec)?;
    fund_info.set_issue_virtual_tokens(config.issue_virtual_tokens)?;

    msg!("Update Fund stats");
    fund_info.update_admin_action_time()
}
