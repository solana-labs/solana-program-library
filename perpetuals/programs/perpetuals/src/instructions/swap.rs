//! Swap instruction handler

use {
    crate::{
        error::PerpetualsError,
        math,
        state::{custody::Custody, oracle::OraclePrice, perpetuals::Perpetuals, pool::Pool},
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
#[instruction(params: SwapParams)]
pub struct Swap<'info> {
    #[account(mut)]
    pub owner: Signer<'info>,

    #[account(
        mut,
        constraint = funding_account.mint == receiving_custody.mint,
        has_one = owner
    )]
    pub funding_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        constraint = receiving_account.mint == dispensing_custody.mint,
        has_one = owner
    )]
    pub receiving_account: Box<Account<'info, TokenAccount>>,

    /// CHECK: empty PDA, authority for token accounts
    #[account(
        seeds = [b"transfer_authority"],
        bump = perpetuals.transfer_authority_bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    #[account(
        seeds = [b"perpetuals"],
        bump = perpetuals.perpetuals_bump
    )]
    pub perpetuals: Box<Account<'info, Perpetuals>>,

    #[account(
        mut,
        seeds = [b"pool",
                 pool.name.as_bytes()],
        bump = pool.bump
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 receiving_custody.mint.as_ref()],
        bump = receiving_custody.bump
    )]
    pub receiving_custody: Box<Account<'info, Custody>>,

    /// CHECK: oracle account for the received token
    #[account(
        constraint = receiving_custody_oracle_account.key() == receiving_custody.oracle.oracle_account
    )]
    pub receiving_custody_oracle_account: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 receiving_custody.mint.as_ref()],
        bump = receiving_custody.token_account_bump
    )]
    pub receiving_custody_token_account: Box<Account<'info, TokenAccount>>,

    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 dispensing_custody.mint.as_ref()],
        bump = dispensing_custody.bump
    )]
    pub dispensing_custody: Box<Account<'info, Custody>>,

    /// CHECK: oracle account for the returned token
    #[account(
        constraint = dispensing_custody_oracle_account.key() == dispensing_custody.oracle.oracle_account
    )]
    pub dispensing_custody_oracle_account: AccountInfo<'info>,

    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 dispensing_custody.mint.as_ref()],
        bump = dispensing_custody.token_account_bump
    )]
    pub dispensing_custody_token_account: Box<Account<'info, TokenAccount>>,

    token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy)]
pub struct SwapParams {
    pub amount_in: u64,
    pub min_amount_out: u64,
}

pub fn swap(ctx: Context<Swap>, params: &SwapParams) -> Result<()> {
    // check permissions
    msg!("Check permissions");
    let perpetuals = ctx.accounts.perpetuals.as_mut();
    let receiving_custody = ctx.accounts.receiving_custody.as_mut();
    let dispensing_custody = ctx.accounts.dispensing_custody.as_mut();
    require!(
        perpetuals.permissions.allow_swap
            && receiving_custody.permissions.allow_swap
            && dispensing_custody.permissions.allow_swap,
        PerpetualsError::InstructionNotAllowed
    );

    // validate inputs
    msg!("Validate inputs");
    if params.amount_in == 0 {
        return Err(ProgramError::InvalidArgument.into());
    }
    require_keys_neq!(receiving_custody.key(), dispensing_custody.key());

    // compute token amount returned to the user
    let pool = ctx.accounts.pool.as_mut();
    let curtime = perpetuals.get_time()?;
    let token_id_in = pool.get_token_id(&receiving_custody.key())?;
    let token_id_out = pool.get_token_id(&dispensing_custody.key())?;

    let received_token_price = OraclePrice::new_from_oracle(
        receiving_custody.oracle.oracle_type,
        &ctx.accounts
            .receiving_custody_oracle_account
            .to_account_info(),
        receiving_custody.oracle.max_price_error,
        receiving_custody.oracle.max_price_age_sec,
        curtime,
        false,
    )?;

    let received_token_ema_price = OraclePrice::new_from_oracle(
        receiving_custody.oracle.oracle_type,
        &ctx.accounts
            .receiving_custody_oracle_account
            .to_account_info(),
        receiving_custody.oracle.max_price_error,
        receiving_custody.oracle.max_price_age_sec,
        curtime,
        receiving_custody.pricing.use_ema,
    )?;

    let dispensed_token_price = OraclePrice::new_from_oracle(
        dispensing_custody.oracle.oracle_type,
        &ctx.accounts
            .dispensing_custody_oracle_account
            .to_account_info(),
        dispensing_custody.oracle.max_price_error,
        dispensing_custody.oracle.max_price_age_sec,
        curtime,
        false,
    )?;

    let dispensed_token_ema_price = OraclePrice::new_from_oracle(
        dispensing_custody.oracle.oracle_type,
        &ctx.accounts
            .dispensing_custody_oracle_account
            .to_account_info(),
        dispensing_custody.oracle.max_price_error,
        dispensing_custody.oracle.max_price_age_sec,
        curtime,
        dispensing_custody.pricing.use_ema,
    )?;

    msg!("Compute swap amount");
    let amount_out = pool.get_swap_amount(
        &received_token_price,
        &received_token_ema_price,
        &dispensed_token_price,
        &dispensed_token_ema_price,
        receiving_custody,
        dispensing_custody,
        params.amount_in,
    )?;

    // calculate fee
    let fees = pool.get_swap_fees(
        token_id_in,
        token_id_out,
        params.amount_in,
        amount_out,
        receiving_custody,
        &received_token_price,
        dispensing_custody,
        &dispensed_token_price,
    )?;
    msg!("Collected fees: {} {}", fees.0, fees.1);

    // check returned amount
    let no_fee_amount = math::checked_sub(amount_out, fees.1)?;
    msg!("Amount out: {}", no_fee_amount);
    require_gte!(
        no_fee_amount,
        params.min_amount_out,
        PerpetualsError::InsufficientAmountReturned
    );

    // check pool constraints
    msg!("Check pool constraints");
    let protocol_fee_in = Pool::get_fee_amount(receiving_custody.fees.protocol_share, fees.0)?;
    let protocol_fee_out = Pool::get_fee_amount(dispensing_custody.fees.protocol_share, fees.1)?;
    let deposit_amount = math::checked_sub(params.amount_in, protocol_fee_in)?;
    let withdrawal_amount = math::checked_add(no_fee_amount, protocol_fee_out)?;

    require!(
        pool.check_token_ratio(
            token_id_in,
            deposit_amount,
            0,
            receiving_custody,
            &received_token_price
        )? && pool.check_token_ratio(
            token_id_out,
            0,
            withdrawal_amount,
            dispensing_custody,
            &dispensed_token_price
        )?,
        PerpetualsError::TokenRatioOutOfRange
    );
    require!(
        math::checked_sub(
            dispensing_custody.assets.owned,
            dispensing_custody.assets.locked
        )? >= withdrawal_amount,
        PerpetualsError::CustodyAmountLimit
    );

    // transfer tokens
    msg!("Transfer tokens");
    perpetuals.transfer_tokens_from_user(
        ctx.accounts.funding_account.to_account_info(),
        ctx.accounts
            .receiving_custody_token_account
            .to_account_info(),
        ctx.accounts.owner.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        params.amount_in,
    )?;

    perpetuals.transfer_tokens(
        ctx.accounts
            .dispensing_custody_token_account
            .to_account_info(),
        ctx.accounts.receiving_account.to_account_info(),
        ctx.accounts.transfer_authority.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        no_fee_amount,
    )?;

    // update custody stats
    msg!("Update custody stats");
    receiving_custody.volume_stats.swap_usd = receiving_custody.volume_stats.swap_usd.wrapping_add(
        received_token_price.get_asset_amount_usd(params.amount_in, receiving_custody.decimals)?,
    );

    receiving_custody.collected_fees.swap_usd =
        receiving_custody.collected_fees.swap_usd.wrapping_add(
            dispensed_token_price.get_asset_amount_usd(fees.0, dispensing_custody.decimals)?,
        );

    receiving_custody.assets.owned =
        math::checked_add(receiving_custody.assets.owned, deposit_amount)?;

    receiving_custody.assets.protocol_fees =
        math::checked_add(receiving_custody.assets.protocol_fees, protocol_fee_in)?;

    dispensing_custody.collected_fees.swap_usd =
        dispensing_custody.collected_fees.swap_usd.wrapping_add(
            dispensed_token_price.get_asset_amount_usd(fees.1, dispensing_custody.decimals)?,
        );

    dispensing_custody.volume_stats.swap_usd =
        dispensing_custody.volume_stats.swap_usd.wrapping_add(
            dispensed_token_price.get_asset_amount_usd(amount_out, dispensing_custody.decimals)?,
        );

    dispensing_custody.assets.protocol_fees =
        math::checked_add(dispensing_custody.assets.protocol_fees, protocol_fee_out)?;

    dispensing_custody.assets.owned =
        math::checked_sub(dispensing_custody.assets.owned, withdrawal_amount)?;

    receiving_custody.update_borrow_rate(curtime)?;
    dispensing_custody.update_borrow_rate(curtime)?;

    Ok(())
}
