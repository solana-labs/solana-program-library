use crate::{
    account_checks::{
        assert_executable, assert_mut, assert_signer, assert_system_program, assert_with_msg,
    },
    instructions::common::transfer,
    state::{assert_buffer_seeds, Idl, SolanaAccount},
    IdlProgramInstruction,
};
use borsh::{BorshDeserialize, BorshSerialize};
use lazy_format::lazy_format;
use solana_program::{account_info::next_account_info, pubkey::Pubkey, system_program};
use solana_program::{account_info::AccountInfo, program_error::ProgramError};
use solana_program::{entrypoint::ProgramResult, msg};
use solana_program::{instruction::AccountMeta, rent::Rent};
use solana_program::{instruction::Instruction, sysvar::Sysvar};

pub fn extend(
    program_id: Pubkey,
    buffer_account: Pubkey,
    payer: Pubkey,
    program_authority: Pubkey,
    program: Pubkey,
    data: Vec<u8>,
) -> Result<Instruction, ProgramError> {
    println!("\tCreating extend instruction");
    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(buffer_account, false),
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(program_authority, true),
            AccountMeta::new_readonly(program, false),
            AccountMeta::new_readonly(system_program::id(), false),
        ],
        data: IdlProgramInstruction::Extend(ExtendArgs { data }).try_to_vec()?,
    })
}

pub struct ExtendCtx<'a, 'info> {
    pub buffer_account: &'a AccountInfo<'info>,
    pub payer_account: &'a AccountInfo<'info>,
    pub authority_account: &'a AccountInfo<'info>,
    pub program: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> ExtendCtx<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        msg!("Loading accounts");
        let ctx = Self {
            buffer_account: next_account_info(accounts_iter)?,
            payer_account: next_account_info(accounts_iter)?,
            authority_account: next_account_info(accounts_iter)?,
            program: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
        };

        msg!("Loaded all accounts");
        // buffer_account
        assert_mut(ctx.buffer_account, "buffer_account")?;

        // payer_account
        assert_mut(ctx.payer_account, "payer_account")?;
        assert_signer(ctx.payer_account, "payer_account")?;

        // authority_account
        assert_signer(ctx.authority_account, "authority_account")?;

        // program
        assert_executable(ctx.program, "program")?;

        // system_program
        // needed to transfer from payer to idl_account
        assert_system_program(ctx.system_program)?;

        let idl: Idl = Idl::from_account_info(ctx.buffer_account)?;
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

#[derive(Clone, Debug, BorshSerialize, BorshDeserialize)]
pub struct ExtendArgs {
    data: Vec<u8>,
}

pub fn handler(ctx: ExtendCtx, args: ExtendArgs) -> ProgramResult {
    assert_buffer_seeds(ctx.program.key, ctx.buffer_account.key)?;
    let mut idl: Idl = Idl::from_account_info(ctx.buffer_account)?;
    idl.data.extend(args.data.clone());

    let curr_lamports = ctx.buffer_account.lamports();
    ctx.buffer_account
        .realloc(std::mem::size_of::<Idl>() + idl.data.len(), false)?;
    let new_lamports = Rent::get()?.minimum_balance(ctx.buffer_account.lamports() as usize);
    transfer(
        ctx.payer_account,
        ctx.buffer_account,
        new_lamports.checked_sub(curr_lamports).unwrap(),
    )?;
    idl.save(ctx.buffer_account)?;

    Ok(())
}
