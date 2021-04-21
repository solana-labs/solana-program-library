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
    SquareRootU64 {
        /// Number underneath the square root sign, whose square root will be
        /// calculated
        radicand: u64,
    },
    /// Calculate the integer square root of the given u128
    ///
    /// No accounts required for this instruction
    SquareRootU128 {
        /// Number underneath the square root sign, whose square root will be
        /// calculated
        radicand: u128,
    },
    /// Multiply two u64 values
    ///
    /// No accounts required for this instruction
    U64Multiply {
        /// The multiplicand
        multiplicand: u64,
        /// The multipier
        multiplier: u64,
    },
    /// Divide two u64 values
    ///
    /// No accounts required for this instruction
    U64Divide {
        /// The dividend
        dividend: u64,
        /// The divisor
        divisor: u64,
    },
    /// Multiply two float values
    ///
    /// No accounts required for this instruction
    F32Multiply {
        /// The multiplicand
        multiplicand: f32,
        /// The multipier
        multiplier: f32,
    },
    /// Divide two float values
    ///
    /// No accounts required for this instruction
    F32Divide {
        /// The dividend
        dividend: f32,
        /// The divisor
        divisor: f32,
    },
    /// Don't do anything for comparison
    ///
    /// No accounts required for this instruction
    Noop,
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
pub fn sqrt_u64(radicand: u64) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: MathInstruction::SquareRootU64 { radicand }
            .try_to_vec()
            .unwrap(),
    }
}

/// Create SquareRoot instruction
pub fn sqrt_u128(radicand: u128) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: MathInstruction::SquareRootU128 { radicand }
            .try_to_vec()
            .unwrap(),
    }
}

/// Create PreciseSquareRoot instruction
pub fn u64_multiply(multiplicand: u64, multiplier: u64) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: MathInstruction::U64Multiply {
            multiplicand,
            multiplier,
        }
        .try_to_vec()
        .unwrap(),
    }
}

/// Create PreciseSquareRoot instruction
pub fn u64_divide(dividend: u64, divisor: u64) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: MathInstruction::U64Divide { dividend, divisor }
            .try_to_vec()
            .unwrap(),
    }
}

/// Create PreciseSquareRoot instruction
pub fn f32_multiply(multiplicand: f32, multiplier: f32) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: MathInstruction::F32Multiply {
            multiplicand,
            multiplier,
        }
        .try_to_vec()
        .unwrap(),
    }
}

/// Create PreciseSquareRoot instruction
pub fn f32_divide(dividend: f32, divisor: f32) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: MathInstruction::F32Divide { dividend, divisor }
            .try_to_vec()
            .unwrap(),
    }
}

/// Create PreciseSquareRoot instruction
pub fn noop() -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: MathInstruction::Noop.try_to_vec().unwrap(),
    }
}
