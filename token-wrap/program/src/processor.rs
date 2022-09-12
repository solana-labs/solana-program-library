//! Program state processor

use {
    crate::{instruction::TokenWrapInstruction, state::Backpointer},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_token_2022::instruction::{decode_instruction_type, decode_instruction_data},
};

/// Instruction processor
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    match decode_instruction_type(input)? {
        TokenWrapInstruction::CreateMint => {
            unimplemented!();
        }
        TokenWrapInstruction::Wrap => {
            unimplemented!();
        }
        TokenWrapInstruction::Unwrap => {
            unimplemented!();
        }
    }
}
