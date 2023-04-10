//! RemovePool instruction handler

use {
    crate::{
        error::PerpetualsError,
        state::{
            multisig::{AdminInstruction, Multisig},
            perpetuals::Perpetuals,
            pool::Pool,
        },
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct RemovePool<'info> {
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
        mut,
        realloc = Perpetuals::LEN + (perpetuals.pools.len() - 1) * 32,
        realloc::payer = admin,
        realloc::zero = false,
        seeds = [b"perpetuals"],
        bump = perpetuals.perpetuals_bump
    )]
    pub perpetuals: Box<Account<'info, Perpetuals>>,

    #[account(
        mut,
        seeds = [b"pool",
                 pool.name.as_bytes()],
        bump = pool.bump,
        close = transfer_authority
    )]
    pub pool: Box<Account<'info, Pool>>,

    system_program: Program<'info, System>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct RemovePoolParams {}

pub fn remove_pool<'info>(
    ctx: Context<'_, '_, '_, 'info, RemovePool<'info>>,
    params: &RemovePoolParams,
) -> Result<u8> {
    // validate signatures
    let mut multisig = ctx.accounts.multisig.load_mut()?;

    let signatures_left = multisig.sign_multisig(
        &ctx.accounts.admin,
        &Multisig::get_account_infos(&ctx)[1..],
        &Multisig::get_instruction_data(AdminInstruction::RemovePool, params)?,
    )?;
    if signatures_left > 0 {
        msg!(
            "Instruction has been signed but more signatures are required: {}",
            signatures_left
        );
        return Ok(signatures_left);
    }

    require!(
        ctx.accounts.pool.custodies.is_empty(),
        PerpetualsError::InvalidPoolState
    );

    // remove pool from the list
    let perpetuals = ctx.accounts.perpetuals.as_mut();
    let pool_idx = perpetuals
        .pools
        .iter()
        .position(|x| *x == ctx.accounts.pool.key())
        .ok_or(PerpetualsError::InvalidPoolState)?;
    perpetuals.pools.remove(pool_idx);

    Ok(0)
}
