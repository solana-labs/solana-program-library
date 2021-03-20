//! Program entrypoint definitions

#![cfg(all(target_arch = "bpf", not(feature = "no-entrypoint")))]

use solana_program::{
    account_info::AccountInfo,
    decode_error::DecodeError,
    entrypoint,
    entrypoint::ProgramResult,
    msg,
    program::invoke_signed,
    program_error::PrintProgramError,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
};
use std::{convert::TryInto, mem::size_of};

use crate::instruction::{release_escrow, seed_escrow, HelloWorldError, HelloWorldInstruction};

entrypoint!(process_instruction);
fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    instruction_data: &[u8],
) -> ProgramResult {
    let instruction = HelloWorldInstruction::unpack(instruction_data)?;

    if let Err(error) = match instruction {
        HelloWorldInstruction::ReleaseEscrow { amount } => {
            msg!("Instruction: Release escrow");
            release_escrow(program_id, accounts, amount)
        }
        HelloWorldInstruction::SeedEscrow { amount } => {
            msg!("Instruction: Seed escrow");
            seed_escrow(program_id, accounts, amount)
        }
    } {
        // catch the error so we can print it
        error.print::<HelloWorldError>();
        return Err(error);
    }
    Ok(())
}
