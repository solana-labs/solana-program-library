//! Program state processor

use {
    crate::{instruction::MathInstruction, precise_number::PreciseNumber},
    borsh::BorshDeserialize,
    solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey},
};

/// Instruction processor
pub fn process_instruction(
    _program_id: &Pubkey,
    _accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = MathInstruction::try_from_slice(input).unwrap();
    match instruction {
        MathInstruction::PreciseSquareRoot { radicand } => {
            msg!("Calculating square root using PreciseNumber");
            let radicand = PreciseNumber::new(radicand as u128).unwrap();
            let result = radicand.sqrt().unwrap().to_imprecise().unwrap() as u64;
            msg!("{}", result);
            Ok(())
        }
    }
}
