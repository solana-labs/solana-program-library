//! SetAdminSigners instruction handler

use {
    crate::state::multisig::{AdminInstruction, Multisig},
    anchor_lang::prelude::*,
};

#[derive(Accounts)]
pub struct SetAdminSigners<'info> {
    #[account()]
    pub admin: Signer<'info>,

    #[account(
        mut,
        seeds = [b"multisig"], 
        bump = multisig.load()?.bump
    )]
    pub multisig: AccountLoader<'info, Multisig>,
    // remaining accounts: 1 to Multisig::MAX_SIGNERS admin signers (read-only, unsigned)
}

#[derive(AnchorSerialize, AnchorDeserialize)]
pub struct SetAdminSignersParams {
    pub min_signatures: u8,
}

pub fn set_admin_signers<'info>(
    ctx: Context<'_, '_, '_, 'info, SetAdminSigners<'info>>,
    params: &SetAdminSignersParams,
) -> Result<u8> {
    // validate signatures
    let mut multisig = ctx.accounts.multisig.load_mut()?;

    let signatures_left = multisig.sign_multisig(
        &ctx.accounts.admin,
        &Multisig::get_account_infos(&ctx)[1..],
        &Multisig::get_instruction_data(AdminInstruction::SetAdminSigners, params)?,
    )?;
    if signatures_left > 0 {
        msg!(
            "Instruction has been signed but more signatures are required: {}",
            signatures_left
        );
        return Ok(signatures_left);
    }

    // set new admin signers
    multisig.set_signers(ctx.remaining_accounts, params.min_signatures)?;

    Ok(0)
}
