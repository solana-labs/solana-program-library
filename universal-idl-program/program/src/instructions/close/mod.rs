use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
};

use crate::{
    account_checks::{assert_address, assert_mut, assert_signer},
    common,
    state::{Idl, SolanaAccount},
    IdlProgramInstruction,
};

pub fn close(
    program_id: Pubkey,
    idl_account: Pubkey,
    recipient: Pubkey,
    authority: Pubkey,
) -> Result<Instruction, ProgramError> {
    Ok(Instruction {
        program_id: program_id,
        accounts: vec![
            AccountMeta::new(idl_account, false),
            AccountMeta::new(recipient, false),
            AccountMeta::new_readonly(authority, true),
        ],
        data: IdlProgramInstruction::Close.try_to_vec()?,
    })
}

pub struct CloseCtx<'a, 'info> {
    pub idl_account: &'a AccountInfo<'info>,
    pub recipient: &'a AccountInfo<'info>,
    pub authority: &'a AccountInfo<'info>,
}

impl<'a, 'info> CloseCtx<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let ctx = Self {
            idl_account: next_account_info(accounts_iter)?,
            recipient: next_account_info(accounts_iter)?,
            authority: next_account_info(accounts_iter)?,
        };

        // idl_account
        assert_mut(ctx.idl_account, "idl_account")?;

        // recipient
        assert_mut(ctx.recipient, "recipient")?;

        // authority
        assert_signer(ctx.authority, "authority")?;

        Ok(ctx)
    }
}

pub fn handler(ctx: CloseCtx) -> ProgramResult {
    let idl: Idl = Idl::from_account_info(ctx.idl_account)?;
    assert_address(
        &idl.authority,
        ctx.authority.key,
        "Authority of IDL does not match authority",
    )?;
    common::close(ctx.idl_account, ctx.recipient)?;
    Ok(())
}
