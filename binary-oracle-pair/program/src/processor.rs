//! Program state processor

use crate::{instruction::Instruction, error::PoolError};
use solana_program::{
    account_info::AccountInfo,
    clock::Slot,
    msg,
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};
use borsh::BorshDeserialize;

/// Initialize the pool
pub fn process_init_pool(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    _mint_end_slot: Slot,
    _decide_end_slot: Slot,
    _nonce: u8,
) -> ProgramResult {
    unimplemented!()
}
/// Processes an instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = Instruction::try_from_slice(input).or(Err(PoolError::InstructionUnpackError))?;
    match instruction {
        Instruction::InitPool(init_args) => {
            msg!("Instruction: InitPool");
            process_init_pool(program_id, accounts, init_args.mint_end_slot, init_args.decide_end_slot, init_args.bump_seed)
        }
        _ => unimplemented!(),
    }
}
