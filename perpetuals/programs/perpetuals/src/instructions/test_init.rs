//! TestInit instruction handler

use {
    crate::{
        error::PerpetualsError,
        state::{multisig::Multisig, perpetuals::Perpetuals},
    },
    anchor_lang::prelude::*,
    anchor_spl::token::Token,
    solana_program::program_error::ProgramError,
};

#[derive(Accounts)]
pub struct TestInit<'info> {
    #[account(mut)]
    pub upgrade_authority: Signer<'info>,

    #[account(
        init,
        payer = upgrade_authority,
        space = Multisig::LEN,
        seeds = [b"multisig"],
        bump
    )]
    pub multisig: AccountLoader<'info, Multisig>,

    /// CHECK: empty PDA, will be set as authority for token accounts
    #[account(
        init,
        payer = upgrade_authority,
        space = 0,
        seeds = [b"transfer_authority"],
        bump
    )]
    pub transfer_authority: AccountInfo<'info>,

    #[account(
        init,
        payer = upgrade_authority,
        space = Perpetuals::LEN,
        seeds = [b"perpetuals"],
        bump
    )]
    pub perpetuals: Box<Account<'info, Perpetuals>>,

    system_program: Program<'info, System>,
    token_program: Program<'info, Token>,
    // remaining accounts: 1 to Multisig::MAX_SIGNERS admin signers (read-only, unsigned)
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct TestInitParams {
    pub min_signatures: u8,
    pub allow_swap: bool,
    pub allow_add_liquidity: bool,
    pub allow_remove_liquidity: bool,
    pub allow_open_position: bool,
    pub allow_close_position: bool,
    pub allow_pnl_withdrawal: bool,
    pub allow_collateral_withdrawal: bool,
    pub allow_size_change: bool,
}

pub fn test_init(ctx: Context<TestInit>, params: &TestInitParams) -> Result<()> {
    if !cfg!(feature = "test") {
        return err!(PerpetualsError::InvalidEnvironment);
    }

    // initialize multisig, this will fail if account is already initialized
    let mut multisig = ctx.accounts.multisig.load_init()?;

    multisig.set_signers(ctx.remaining_accounts, params.min_signatures)?;

    // record multisig PDA bump
    multisig.bump = *ctx
        .bumps
        .get("multisig")
        .ok_or(ProgramError::InvalidSeeds)?;

    // record perpetuals
    let perpetuals = ctx.accounts.perpetuals.as_mut();
    perpetuals.permissions.allow_swap = params.allow_swap;
    perpetuals.permissions.allow_add_liquidity = params.allow_add_liquidity;
    perpetuals.permissions.allow_remove_liquidity = params.allow_remove_liquidity;
    perpetuals.permissions.allow_open_position = params.allow_open_position;
    perpetuals.permissions.allow_close_position = params.allow_close_position;
    perpetuals.permissions.allow_pnl_withdrawal = params.allow_pnl_withdrawal;
    perpetuals.permissions.allow_collateral_withdrawal = params.allow_collateral_withdrawal;
    perpetuals.permissions.allow_size_change = params.allow_size_change;
    perpetuals.transfer_authority_bump = *ctx
        .bumps
        .get("transfer_authority")
        .ok_or(ProgramError::InvalidSeeds)?;
    perpetuals.perpetuals_bump = *ctx
        .bumps
        .get("perpetuals")
        .ok_or(ProgramError::InvalidSeeds)?;
    perpetuals.inception_time = if cfg!(feature = "test") {
        0
    } else {
        perpetuals.get_time()?
    };

    if !perpetuals.validate() {
        return err!(PerpetualsError::InvalidPerpetualsConfig);
    }

    Ok(())
}
