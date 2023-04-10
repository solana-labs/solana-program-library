//! SetTestTime instruction handler

use {
    crate::{
        error::PerpetualsError,
        state::{
            multisig::{AdminInstruction, Multisig},
            perpetuals::Perpetuals,
        },
    },
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct SetTestTime<'info> {
    #[account()]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"multisig"],
        bump = multisig.load()?.bump
    )]
    pub multisig: AccountLoader<'info, Multisig>,

    #[account(
        mut,
        seeds = [b"perpetuals"],
        bump = perpetuals.perpetuals_bump
    )]
    pub perpetuals: Box<Account<'info, Perpetuals>>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SetTestTimeParams {
    pub time: i64,
}

pub fn set_test_time<'info>(
    ctx: Context<'_, '_, '_, 'info, SetTestTime<'info>>,
    params: &SetTestTimeParams,
) -> Result<u8> {
    if !cfg!(feature = "test") {
        return err!(PerpetualsError::InvalidEnvironment);
    }

    // validate signatures
    let mut multisig = ctx.accounts.multisig.load_mut()?;

    let signatures_left = multisig.sign_multisig(
        &ctx.accounts.admin,
        &Multisig::get_account_infos(&ctx)[1..],
        &Multisig::get_instruction_data(AdminInstruction::SetTestTime, params)?,
    )?;
    if signatures_left > 0 {
        msg!(
            "Instruction has been signed but more signatures are required: {}",
            signatures_left
        );
        return Ok(signatures_left);
    }

    // update time data
    if cfg!(feature = "test") {
        ctx.accounts.perpetuals.inception_time = params.time;
    }

    Ok(0)
}
