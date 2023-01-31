use borsh::BorshSerialize;
use solana_program::account_info::{next_account_info, AccountInfo};
use solana_program::clock::Clock;
use solana_program::entrypoint::ProgramResult;
use solana_program::instruction::AccountMeta;
use solana_program::instruction::Instruction;
use solana_program::msg;
use solana_program::program_error::ProgramError;
use solana_program::pubkey::Pubkey;
use solana_program::sysvar::Sysvar;

use crate::account_checks::{assert_address, assert_mut, assert_signer, assert_with_msg};
use crate::common::close;
use crate::common::transfer;
use crate::state::{assert_buffer_seeds, assert_idl_seeds, Idl, SolanaAccount};
use crate::IdlProgramInstruction;

pub fn set_buffer(
    program_id: Pubkey,
    wallet: Pubkey,
    authority: Pubkey,
    idl_account: Pubkey,
    buffer_account: Pubkey,
    program: Pubkey,
) -> Result<Instruction, ProgramError> {
    Ok(Instruction {
        program_id,
        accounts: vec![
            AccountMeta::new(wallet, true),
            AccountMeta::new_readonly(authority, true),
            AccountMeta::new(idl_account, false),
            AccountMeta::new(buffer_account, false),
            AccountMeta::new_readonly(program, false),
        ],
        data: IdlProgramInstruction::SetBuffer.try_to_vec()?,
    })
}

pub struct SetBufferCtx<'a, 'info> {
    wallet: &'a AccountInfo<'info>,
    authority: &'a AccountInfo<'info>,
    idl_account: &'a AccountInfo<'info>,
    buffer_account: &'a AccountInfo<'info>,
    program: &'a AccountInfo<'info>,
}

impl<'a, 'info> SetBufferCtx<'a, 'info> {
    pub fn load(accounts: &'a [AccountInfo<'info>]) -> Result<Self, ProgramError> {
        let accounts_iter = &mut accounts.iter();
        let ctx = Self {
            wallet: next_account_info(accounts_iter)?,
            authority: next_account_info(accounts_iter)?,
            idl_account: next_account_info(accounts_iter)?,
            buffer_account: next_account_info(accounts_iter)?,
            program: next_account_info(accounts_iter)?,
        };

        // wallet
        assert_mut(ctx.wallet, "wallet")?;
        assert_signer(ctx.wallet, "wallet")?;

        // authority
        assert_signer(ctx.authority, "authority")?;

        // idl_account
        assert_mut(ctx.idl_account, "idl_account")?;

        // buffer_account
        assert_mut(ctx.buffer_account, "buffer_account")?;

        // program
        assert_with_msg(
            ctx.program.executable,
            ProgramError::InvalidAccountData,
            "Program account must be a valid executable",
        )?;

        Ok(ctx)
    }
}

pub fn handler(ctx: SetBufferCtx) -> ProgramResult {
    assert_idl_seeds(ctx.program.key, ctx.idl_account.key)?;
    assert_buffer_seeds(ctx.program.key, ctx.buffer_account.key)?;

    let mut idl: Idl = Idl::from_account_info(ctx.idl_account)?;
    let buffer: Idl = Idl::from_account_info(ctx.buffer_account)?;

    assert_address(
        &idl.authority,
        ctx.authority.key,
        "Expected authority to be the same as the one in the idl account",
    )?;

    // Copy buffer data into idl account
    idl.data = buffer.data.clone();
    idl.slot = Clock::get()?.slot;

    // Close the buffer account
    let necessary_idl_lamports = ctx.buffer_account.lamports();
    msg!(
        "Closing buffer account with {} lamports",
        necessary_idl_lamports
    );
    close(ctx.buffer_account, ctx.idl_account)?;

    // Realloc the idl account to the size of the buffer account
    ctx.idl_account
        .realloc(std::mem::size_of::<Idl>() + idl.data.len(), false)?;

    // Trying to save before reallocing causes an error (data corruption?)
    idl.save(ctx.idl_account)?;

    // Move lamports from idl account to wallet if necessary
    let current_idl_lamports = ctx.idl_account.try_lamports()?;
    if current_idl_lamports > necessary_idl_lamports {
        let lamports_to_transfer = current_idl_lamports
            .checked_sub(necessary_idl_lamports)
            .unwrap();
        msg!("Refunding {} lamports back to wallet", lamports_to_transfer);
        transfer(ctx.idl_account, ctx.wallet, lamports_to_transfer)?;
    }
    Ok(())
}
