use borsh::{BorshDeserialize, BorshSerialize};
use lazy_format::lazy_format;
use solana_program::account_info::next_account_info;
use solana_program::account_info::AccountInfo;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::AccountMeta;
use solana_program::instruction::Instruction;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;

use crate::account_checks::assert_executable;
use crate::account_checks::assert_mut;
use crate::account_checks::assert_signer;
use crate::account_checks::assert_with_msg;
use crate::state::assert_idl_seeds;
use crate::{
    state::{Idl, SolanaAccount},
    IdlProgramInstruction,
};

pub fn set_authority(
    program_id: Pubkey,
    idl_account: Pubkey,
    program: Pubkey,
    program_authority: Pubkey,
    new_authority: Pubkey,
) -> Result<Instruction, ProgramError> {
    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(idl_account, false),
            AccountMeta::new_readonly(program, false),
            AccountMeta::new_readonly(program_authority, true),
        ],
        data: IdlProgramInstruction::SetAuthority(SetAuthorityArgs { new_authority })
            .try_to_vec()?,
    })
}

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct SetAuthorityArgs {
    new_authority: Pubkey,
}

pub struct SetAuthorityCtx<'a, 'info> {
    pub idl_account: &'a AccountInfo<'info>,
    pub program: &'a AccountInfo<'info>,
    pub authority_account: &'a AccountInfo<'info>,
}

impl<'a, 'info> SetAuthorityCtx<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let ctx = Self {
            idl_account: next_account_info(accounts_iter)?,
            program: next_account_info(accounts_iter)?,
            authority_account: next_account_info(accounts_iter)?,
        };

        // idl_account
        assert_mut(ctx.idl_account, "idl_account")?;

        // program
        assert_executable(ctx.program, "program")?;

        // authority_account
        assert_signer(ctx.authority_account, "authority_account")?;

        let idl: Idl = Idl::from_account_info(ctx.idl_account)?;
        assert_with_msg(
            idl.authority == *ctx.authority_account.key,
            ProgramError::InvalidInstructionData,
            lazy_format!(
                "authority_account {} does not match idl.authority {}",
                ctx.authority_account.key,
                idl.authority,
            ),
        )?;

        Ok(ctx)
    }
}

pub fn handler(ctx: SetAuthorityCtx, args: SetAuthorityArgs) -> ProgramResult {
    assert_idl_seeds(ctx.program.key, ctx.idl_account.key)?;
    let mut idl: Idl = Idl::from_account_info(ctx.idl_account)?;
    idl.authority = args.new_authority;
    idl.save(ctx.idl_account)?;
    Ok(())
}
