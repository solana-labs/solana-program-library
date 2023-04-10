//! GetEntryPriceAndFee instruction handler

use {
    crate::state::{
        custody::Custody,
        oracle::OraclePrice,
        perpetuals::{NewPositionPricesAndFee, Perpetuals},
        pool::Pool,
        position::{Position, Side},
    },
    anchor_lang::prelude::*,
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct GetEntryPriceAndFee<'info> {
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

    #[account(
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody.mint.as_ref()],
        bump = custody.bump
    )]
    pub custody: Box<Account<'info, Custody>>,

    /// CHECK: oracle account for the collateral token
    #[account(
        constraint = custody_oracle_account.key() == custody.oracle.oracle_account
    )]
    pub custody_oracle_account: AccountInfo<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct GetEntryPriceAndFeeParams {
    collateral: u64,
    size: u64,
    side: Side,
}

pub fn get_entry_price_and_fee(
    ctx: Context<GetEntryPriceAndFee>,
    params: &GetEntryPriceAndFeeParams,
) -> Result<NewPositionPricesAndFee> {
    // validate inputs
    if params.collateral == 0 || params.size == 0 || params.side == Side::None {
        return Err(ProgramError::InvalidArgument.into());
    }
    let pool = &ctx.accounts.pool;
    let custody = ctx.accounts.custody.as_mut();

    // compute position price
    let curtime = ctx.accounts.perpetuals.get_time()?;

    let token_price = OraclePrice::new_from_oracle(
        custody.oracle.oracle_type,
        &ctx.accounts.custody_oracle_account.to_account_info(),
        custody.oracle.max_price_error,
        custody.oracle.max_price_age_sec,
        curtime,
        false,
    )?;

    let token_ema_price = OraclePrice::new_from_oracle(
        custody.oracle.oracle_type,
        &ctx.accounts.custody_oracle_account.to_account_info(),
        custody.oracle.max_price_error,
        custody.oracle.max_price_age_sec,
        curtime,
        custody.pricing.use_ema,
    )?;

    let min_price = if token_price < token_ema_price {
        token_price
    } else {
        token_ema_price
    };

    let entry_price = pool.get_entry_price(&token_price, &token_ema_price, params.side, custody)?;

    let size_usd = min_price.get_asset_amount_usd(params.size, custody.decimals)?;
    let collateral_usd = min_price.get_asset_amount_usd(params.collateral, custody.decimals)?;

    let position = Position {
        side: params.side,
        price: entry_price,
        size_usd,
        collateral_usd,
        cumulative_interest_snapshot: custody.get_cumulative_interest(curtime)?,
        ..Position::default()
    };

    let liquidation_price =
        pool.get_liquidation_price(&position, &token_ema_price, custody, curtime)?;

    let fee = pool.get_entry_fee(params.size, custody)?;

    Ok(NewPositionPricesAndFee {
        entry_price,
        liquidation_price,
        fee,
    })
}
