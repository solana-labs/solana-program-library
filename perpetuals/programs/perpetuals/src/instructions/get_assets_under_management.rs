//! GetAssetsUnderManagement instruction handler

use {
    crate::state::{
        perpetuals::Perpetuals,
        pool::{AumCalcMode, Pool},
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct GetAssetsUnderManagement<'info> {
    #[account(
        seeds = [b"perpetuals"],
        bump = perpetuals.perpetuals_bump
    )]
    pub perpetuals: Box<Account<'info, Perpetuals>>,

    #[account(
        seeds = [b"pool",
                 pool.name.as_bytes()],
        bump = pool.bump
    )]
    pub pool: Box<Account<'info, Pool>>,
    // remaining accounts:
    //   pool.tokens.len() custody accounts (read-only, unsigned)
    //   pool.tokens.len() custody oracles (read-only, unsigned)
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct GetAssetsUnderManagementParams {}

pub fn get_assets_under_management(
    ctx: Context<GetAssetsUnderManagement>,
    _params: &GetAssetsUnderManagementParams,
) -> Result<u128> {
    ctx.accounts.pool.get_assets_under_management_usd(
        AumCalcMode::EMA,
        ctx.remaining_accounts,
        ctx.accounts.perpetuals.get_time()?,
    )
}
