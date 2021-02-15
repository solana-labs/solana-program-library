//! Program state processor

use crate::instruction::Instruction;
//use num_traits::FromPrimitive;
use solana_program::{
    account_info::AccountInfo,
    clock::Slot,
    msg,
    //    decode_error::DecodeError,
    entrypoint::ProgramResult,
    //    program::{invoke, invoke_signed},
    //    program_error::{PrintProgramError, ProgramError},
    //    program_option::COption,
    program_pack::Pack,
    pubkey::Pubkey,
    //    sysvar::{clock::Clock, rent::Rent, Sysvar},
};
//use spl_token::state::Account as Token;

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
    let instruction = Instruction::unpack(input)?;
    match instruction {
        Instruction::InitPool {
            mint_end_slot,
            decide_end_slot,
            nonce,
        } => {
            msg!("Instruction: InitPool");
            process_init_pool(program_id, accounts, mint_end_slot, decide_end_slot, nonce)
        }
        _ => unimplemented!(),
    }
}
