//! AddCustody instruction handler

use {
    crate::{
        error::PerpetualsError,
        state::{
            custody::{BorrowRateParams, Custody, Fees, OracleParams, PricingParams},
            multisig::{AdminInstruction, Multisig},
            perpetuals::{Permissions, Perpetuals},
            pool::{Pool, TokenRatios},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Mint, Token, TokenAccount},
};

#[derive(Accounts)]
pub struct AddCustody<'info> {
    #[account(mut)]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"multisig"],
        bump = multisig.load()?.bump
    )]
    pub multisig: AccountLoader<'info, Multisig>,

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
        realloc = Pool::LEN + (pool.custodies.len() + 1) * std::mem::size_of::<Pubkey>() +
                              (pool.ratios.len() + 1) * std::mem::size_of::<TokenRatios>(),
        realloc::payer = admin,
        realloc::zero = false,
        seeds = [b"pool",
                 pool.name.as_bytes()],
        bump = pool.bump
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(
        init_if_needed,
        payer = admin,
        space = Custody::LEN,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody_token_mint.key().as_ref()],
        bump
    )]
    pub custody: Box<Account<'info, Custody>>,

    #[account(
        init_if_needed,
        payer = admin,
        token::mint = custody_token_mint,
        token::authority = transfer_authority,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 custody_token_mint.key().as_ref()],
        bump
    )]
    pub custody_token_account: Box<Account<'info, TokenAccount>>,

    #[account()]
    pub custody_token_mint: Box<Account<'info, Mint>>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    rent: Sysvar<'info, Rent>,
}

#[derive(AnchorSerialize, AnchorDeserialize, Clone)]
pub struct AddCustodyParams {
    pub is_stable: bool,
    pub oracle: OracleParams,
    pub pricing: PricingParams,
    pub permissions: Permissions,
    pub fees: Fees,
    pub borrow_rate: BorrowRateParams,
    pub ratios: Vec<TokenRatios>,
}

pub fn add_custody<'info>(
    ctx: Context<'_, '_, '_, 'info, AddCustody<'info>>,
    params: &AddCustodyParams,
) -> Result<u8> {
    // validate inputs
    if params.ratios.len() != ctx.accounts.pool.ratios.len() + 1 {
        return Err(ProgramError::InvalidArgument.into());
    }

    // validate signatures
    let mut multisig = ctx.accounts.multisig.load_mut()?;

    let signatures_left = multisig.sign_multisig(
        &ctx.accounts.admin,
        &Multisig::get_account_infos(&ctx)[1..],
        &Multisig::get_instruction_data(AdminInstruction::AddCustody, params)?,
    )?;
    if signatures_left > 0 {
        msg!(
            "Instruction has been signed but more signatures are required: {}",
            signatures_left
        );
        return Ok(signatures_left);
    }

    let pool = ctx.accounts.pool.as_mut();
    if pool.get_token_id(&ctx.accounts.custody.key()).is_ok() {
        // return error if custody is already initialized
        return Err(ProgramError::AccountAlreadyInitialized.into());
    }

    // update pool data
    pool.custodies.push(ctx.accounts.custody.key());
    pool.ratios = params.ratios.clone();
    if !pool.validate() {
        return err!(PerpetualsError::InvalidPoolConfig);
    }

    // record custody data
    let custody = ctx.accounts.custody.as_mut();
    custody.pool = pool.key();
    custody.mint = ctx.accounts.custody_token_mint.key();
    custody.token_account = ctx.accounts.custody_token_account.key();
    custody.decimals = ctx.accounts.custody_token_mint.decimals;
    custody.is_stable = params.is_stable;
    custody.oracle = params.oracle;
    custody.pricing = params.pricing;
    custody.permissions = params.permissions;
    custody.fees = params.fees;
    custody.borrow_rate = params.borrow_rate;
    custody.borrow_rate_state.current_rate = params.borrow_rate.base_rate;
    custody.borrow_rate_state.last_update = ctx.accounts.perpetuals.get_time()?;
    custody.bump = *ctx.bumps.get("custody").ok_or(ProgramError::InvalidSeeds)?;
    custody.token_account_bump = *ctx
        .bumps
        .get("custody_token_account")
        .ok_or(ProgramError::InvalidSeeds)?;

    if !custody.validate() {
        err!(PerpetualsError::InvalidCustodyConfig)
    } else {
        Ok(0)
    }
}
