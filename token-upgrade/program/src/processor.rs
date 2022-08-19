//! Program state processor

use {
    crate::instruction::*,
    solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, pubkey::Pubkey},
    spl_token_2022::instruction::decode_instruction_type,
};

/// Instruction processor
pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    match decode_instruction_type(input)? {
        TokenUpgradeInstruction::CreateFactory => {
            unimplemented!()
        }
        TokenUpgradeInstruction::UpgradeTokens => {
            unimplemented!()
        }
    }
}
