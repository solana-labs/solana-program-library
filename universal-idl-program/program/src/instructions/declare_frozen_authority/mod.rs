use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    rent::Rent,
    system_instruction::create_account,
    system_program,
    sysvar::{clock::Clock, Sysvar},
};

use crate::{
    account_checks::{
        assert_mut, assert_owned_by_frozen_loader, assert_program_authority_in_allowlist,
        assert_signer, assert_system_program,
    },
    state::{
        assert_frozen_authority_seeds, frozen_authority_seeds, FrozenProgramAuthority,
        SolanaAccount,
    },
    IdlProgramInstruction,
};

pub fn declare_frozen_authority(
    program_id: Pubkey,
    program: Pubkey,
    meta_authority: Pubkey,
    payer: Pubkey,
    new_frozen_authority: Pubkey,
) -> Result<Instruction, ProgramError> {
    let frozen_authority = frozen_authority_seeds(&program).0;
    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new_readonly(program, false),
            AccountMeta::new(frozen_authority, false),
            AccountMeta::new_readonly(meta_authority, true),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: IdlProgramInstruction::DeclareFrozenAuthority(DeclareFrozenAuthorityArgs {
            authority: new_frozen_authority,
        })
        .try_to_vec()?,
    })
}

#[derive(Debug, Clone, BorshDeserialize, BorshSerialize)]
pub struct DeclareFrozenAuthorityArgs {
    authority: Pubkey,
}

pub struct DeclareFrozenAuthorityCtx<'a, 'info> {
    pub program: &'a AccountInfo<'info>,
    pub frozen_authority: &'a AccountInfo<'info>,
    pub meta_authority: &'a AccountInfo<'info>,
    pub payer: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> DeclareFrozenAuthorityCtx<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let ctx = Self {
            program: next_account_info(accounts_iter)?,
            frozen_authority: next_account_info(accounts_iter)?,
            meta_authority: next_account_info(accounts_iter)?,
            payer: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
        };

        // program
        assert_owned_by_frozen_loader(ctx.program, "program")?;

        // frozen_authority
        assert_mut(ctx.frozen_authority, "frozen_authority")?;

        // meta_authority
        assert_signer(ctx.meta_authority, "meta_authority")?;
        assert_program_authority_in_allowlist(ctx.meta_authority, "meta_authority")?;

        // payer
        assert_signer(ctx.payer, "payer")?;
        assert_mut(ctx.payer, "payer")?;

        // system_program
        assert_system_program(ctx.system_program)?;
        Ok(ctx)
    }
}

pub fn handler(ctx: DeclareFrozenAuthorityCtx, args: DeclareFrozenAuthorityArgs) -> ProgramResult {
    let (frozen_authority_key, seeds) =
        assert_frozen_authority_seeds(&ctx.program.key, ctx.frozen_authority.key)?;

    let data_len = std::mem::size_of::<FrozenProgramAuthority>();
    let lamports = Rent::get()?.minimum_balance(data_len);

    invoke_signed(
        &create_account(
            ctx.payer.key,
            &frozen_authority_key,
            lamports,
            data_len as u64,
            ctx.program.key,
        ),
        &[ctx.payer.clone(), ctx.frozen_authority.clone()],
        &[&seeds.iter().map(|s| s.as_slice()).collect::<Vec<&[u8]>>()],
    )?;

    let mut frozen_authority: FrozenProgramAuthority = FrozenProgramAuthority::new();
    frozen_authority.authority = args.authority;
    frozen_authority.meta_authority = *ctx.meta_authority.key;
    frozen_authority.slot = Clock::get()?.slot;
    frozen_authority.save(ctx.frozen_authority)?;

    Ok(())
}
