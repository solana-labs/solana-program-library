//! RemoveCustody instruction handler

use {
    crate::{
        error::PerpetualsError,
        state::{
            custody::Custody,
            multisig::{AdminInstruction, Multisig},
            perpetuals::Perpetuals,
            pool::{Pool, TokenRatios},
        },
    },
    anchor_lang::prelude::*,
    anchor_spl::token::{Token, TokenAccount},
};

#[derive(Accounts)]
pub struct RemoveCustody<'info> {
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
        mut,
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
        realloc = Pool::LEN + (pool.custodies.len() - 1) * std::mem::size_of::<Pubkey>() +
                              (pool.ratios.len() - 1) * std::mem::size_of::<TokenRatios>(),
        realloc::payer = admin,
        realloc::zero = false,
        seeds = [b"pool",
                 pool.name.as_bytes()],
        bump = pool.bump
    )]
    pub pool: Box<Account<'info, Pool>>,

    #[account(
        mut,
        seeds = [b"custody",
                 pool.key().as_ref(),
                 custody.mint.as_ref()],
        bump = custody.bump,
        close = transfer_authority
    )]
    pub custody: Box<Account<'info, Custody>>,

    #[account(
        mut,
        seeds = [b"custody_token_account",
                 pool.key().as_ref(),
                 custody.mint.as_ref()],
        bump = custody.token_account_bump,
    )]
    pub custody_token_account: Box<Account<'info, TokenAccount>>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RemoveCustodyParams {
    pub ratios: Vec<TokenRatios>,
}

pub fn remove_custody<'info>(
    ctx: Context<'_, '_, '_, 'info, RemoveCustody<'info>>,
    params: &RemoveCustodyParams,
) -> Result<u8> {
    // validate inputs
    if ctx.accounts.pool.ratios.is_empty()
        || params.ratios.len() != ctx.accounts.pool.ratios.len() - 1
    {
        return Err(ProgramError::InvalidArgument.into());
    }

    // validate signatures
    let mut multisig = ctx.accounts.multisig.load_mut()?;

    let signatures_left = multisig.sign_multisig(
        &ctx.accounts.admin,
        &Multisig::get_account_infos(&ctx)[1..],
        &Multisig::get_instruction_data(AdminInstruction::RemoveCustody, params)?,
    )?;
    if signatures_left > 0 {
        msg!(
            "Instruction has been signed but more signatures are required: {}",
            signatures_left
        );
        return Ok(signatures_left);
    }

    require!(
        ctx.accounts.custody_token_account.amount == 0,
        PerpetualsError::InvalidCustodyState
    );

    // remove token from the list
    let pool = ctx.accounts.pool.as_mut();
    let token_id = pool.get_token_id(&ctx.accounts.custody.key())?;
    pool.custodies.remove(token_id);
    pool.ratios = params.ratios.clone();
    if !pool.validate() {
        return err!(PerpetualsError::InvalidPoolConfig);
    }

    Perpetuals::close_token_account(
        ctx.accounts.transfer_authority.to_account_info(),
        ctx.accounts.custody_token_account.to_account_info(),
        ctx.accounts.token_program.to_account_info(),
        ctx.accounts.transfer_authority.to_account_info(),
        &[&[
            b"transfer_authority",
            &[ctx.accounts.perpetuals.transfer_authority_bump],
        ]],
    )?;

    Ok(0)
}
