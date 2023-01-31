use crate::account_checks::assert_address;
use crate::account_checks::assert_executable;
use crate::account_checks::assert_mut;
use crate::account_checks::assert_signer;
use crate::account_checks::assert_system_program;
use crate::id;
use crate::state::assert_buffer_seeds;
use crate::state::Idl;
use crate::state::SolanaAccount;
use crate::IdlProgramInstruction;

use borsh::BorshSerialize;
use solana_program::program::invoke_signed;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction::create_account,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
};

pub fn create_buffer(
    program_id: Pubkey,
    payer: Pubkey,
    idl_account: Pubkey,
    authority: Pubkey,
    buffer_account: Pubkey,
    program: Pubkey,
) -> Result<Instruction, ProgramError> {
    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(payer, true),
            AccountMeta::new_readonly(idl_account, false),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(buffer_account, false),
            AccountMeta::new_readonly(program, false),
            AccountMeta::new_readonly(solana_program::system_program::id(), false),
        ],
        data: IdlProgramInstruction::CreateBuffer.try_to_vec()?,
    })
}

// You have to use same authority as IDL to create buffer
pub struct CreateBufferCtx<'a, 'info> {
    pub payer: &'a AccountInfo<'info>,
    pub idl_account: &'a AccountInfo<'info>,
    pub authority: &'a AccountInfo<'info>,
    pub buffer_account: &'a AccountInfo<'info>,
    pub program: &'a AccountInfo<'info>,
    pub system_program: &'a AccountInfo<'info>,
}

impl<'a, 'info> CreateBufferCtx<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let ctx = Self {
            payer: next_account_info(accounts_iter)?,
            idl_account: next_account_info(accounts_iter)?,
            authority: next_account_info(accounts_iter)?,
            buffer_account: next_account_info(accounts_iter)?,
            program: next_account_info(accounts_iter)?,
            system_program: next_account_info(accounts_iter)?,
        };

        // payer
        assert_signer(ctx.payer, "payer")?;
        assert_mut(ctx.payer, "payer")?;

        // authority
        assert_signer(ctx.authority, "authority")?;
        let idl: Idl = Idl::from_account_info(ctx.idl_account)?;
        assert_address(ctx.authority.key, &idl.authority, "authority")?;

        // buffer_account
        assert_mut(ctx.idl_account, "buffer_account")?;

        // program
        assert_executable(ctx.program, "program")?;

        // system_program
        assert_system_program(ctx.system_program)?;

        Ok(ctx)
    }
}

pub fn handler(ctx: CreateBufferCtx) -> ProgramResult {
    let buffer_seeds = assert_buffer_seeds(ctx.program.key, ctx.buffer_account.key)?;
    let data_len = std::mem::size_of::<Idl>();
    invoke_signed(
        &create_account(
            ctx.payer.key,
            ctx.buffer_account.key,
            Rent::get()?.minimum_balance(data_len as usize),
            data_len as u64,
            &id(),
        ),
        &[ctx.payer.clone(), ctx.buffer_account.clone()],
        &[&buffer_seeds
            .iter()
            .map(|s| s.as_slice())
            .collect::<Vec<&[u8]>>()],
    )?;

    let mut buffer: Idl = Idl::new();
    buffer.authority = *ctx.authority.key;
    buffer.slot = Clock::get()?.slot;
    buffer.save(ctx.buffer_account)?;

    Ok(())
}
