//! WithdrawSolFees instruction handler

use {
    crate::{
        math,
        state::{
            multisig::{AdminInstruction, Multisig},
            perpetuals::Perpetuals,
        },
    },
    anchor_lang::prelude::*,
    solana_program::sysvar,
};

#[derive(Accounts)]
pub struct WithdrawSolFees<'info> {
    #[account()]
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

    /// CHECK: SOL fees receiving account
    #[account(
        mut,
        constraint = receiving_account.data_is_empty()
    )]
    pub receiving_account: AccountInfo<'info>,
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct WithdrawSolFeesParams {
    pub amount: u64,
}

pub fn withdraw_sol_fees<'info>(
    ctx: Context<'_, '_, '_, 'info, WithdrawSolFees<'info>>,
    params: &WithdrawSolFeesParams,
) -> Result<u8> {
    // validate inputs
    if params.amount == 0 {
        return Err(ProgramError::InvalidArgument.into());
    }

    // validate signatures
    let mut multisig = ctx.accounts.multisig.load_mut()?;

    let signatures_left = multisig.sign_multisig(
        &ctx.accounts.admin,
        &Multisig::get_account_infos(&ctx)[1..],
        &Multisig::get_instruction_data(AdminInstruction::WithdrawSolFees, params)?,
    )?;
    if signatures_left > 0 {
        msg!(
            "Instruction has been signed but more signatures are required: {}",
            signatures_left
        );
        return Ok(signatures_left);
    }

    // transfer sol fees from the custody to the receiver
    let balance = ctx.accounts.transfer_authority.try_lamports()?;
    let min_balance = sysvar::rent::Rent::get().unwrap().minimum_balance(0);
    let available_balance = if balance > min_balance {
        math::checked_sub(balance, min_balance)?
    } else {
        0
    };

    msg!(
        "Withdraw SOL fees: {} / {}",
        params.amount,
        available_balance
    );

    if available_balance < params.amount {
        return Err(ProgramError::InsufficientFunds.into());
    }

    Perpetuals::transfer_sol_from_owned(
        ctx.accounts.transfer_authority.to_account_info(),
        ctx.accounts.receiving_account.to_account_info(),
        params.amount,
    )?;

    Ok(0)
}
