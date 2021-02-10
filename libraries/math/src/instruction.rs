//! Program instructions, used for end-to-end testing and instruction counts

use {
    crate::id,
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::instruction::Instruction,
};

/// Instructions supported by the math program, used for testing instruction
/// counts
#[derive(Clone, Debug, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum MathInstruction {
    /// Calculate the square root of the given u64 with decimals
    ///
    /// No accounts required for this instruction
    PreciseSquareRoot {
        /// Number underneath the square root sign, whose square root will be
        /// calculated
        radicand: u64,
    },
    /// Calculate the integer square root of the given u64
    ///
    /// No accounts required for this instruction
    SquareRoot {
        /// Number underneath the square root sign, whose square root will be
        /// calculated
        radicand: u64,
    },
}

/// Create PreciseSquareRoot instruction
pub fn precise_sqrt(radicand: u64) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: MathInstruction::PreciseSquareRoot { radicand }
            .try_to_vec()
            .unwrap(),
    }
}

/// Create SquareRoot instruction
pub fn sqrt(radicand: u64) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: MathInstruction::SquareRoot { radicand }
            .try_to_vec()
            .unwrap(),
    }
}
