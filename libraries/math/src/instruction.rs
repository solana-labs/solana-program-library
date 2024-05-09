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

    /// Exponentiate a float base by a power
    ///
    /// No accounts required for this instruction
    F32Exponentiate {
        /// The base
        base: f32,
        /// The exponent
        exponent: f32,
    },

    /// Natural Log of a float
    ///
    /// No accounts required for this instruction
    F32NaturalLog {
        /// The argument
        argument: f32,
    },

    /// The Normal CDF of a float
    ///
    /// No accounts required for this instruction
    F32NormalCDF {
        /// The argument
        argument: f32,
    },

    /// Pow two float values
    ///
    /// No accounts required for this instruction
    F64Pow {
        /// The base
        base: f64,
        /// The exponent
        exponent: f64,
    },

    /// Multiply two u128 values
    ///
    /// No accounts required for this instruction
    U128Multiply {
        /// The multiplicand
        multiplicand: u128,
        /// The multipier
        multiplier: u128,
    },
    /// Divide two u128 values
    ///
    /// No accounts required for this instruction
    U128Divide {
        /// The dividend
        dividend: u128,
        /// The divisor
        divisor: u128,
    },
    /// Multiply two f64 values
    ///
    /// No accounts required for this instruction
    F64Multiply {
        /// The multiplicand
        multiplicand: f64,
        /// The multipier
        multiplier: f64,
    },
    /// Divide two f64 values
    ///
    /// No accounts required for this instruction
    F64Divide {
        /// The dividend
        dividend: f64,
        /// The divisor
        divisor: f64,
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
        data: borsh::to_vec(&MathInstruction::PreciseSquareRoot { radicand }).unwrap(),
    }
}

/// Create U64 SquareRoot instruction
pub fn sqrt_u64(radicand: u64) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: borsh::to_vec(&MathInstruction::SquareRootU64 { radicand }).unwrap(),
    }
}

/// Create U128 SquareRoot instruction
pub fn sqrt_u128(radicand: u128) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: borsh::to_vec(&MathInstruction::SquareRootU128 { radicand }).unwrap(),
    }
}

/// Create U64 Multiplication instruction
pub fn u64_multiply(multiplicand: u64, multiplier: u64) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: borsh::to_vec(&MathInstruction::U64Multiply {
            multiplicand,
            multiplier,
        })
        .unwrap(),
    }
}

/// Create U64 Division instruction
pub fn u64_divide(dividend: u64, divisor: u64) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: borsh::to_vec(&MathInstruction::U64Divide { dividend, divisor }).unwrap(),
    }
}

/// Create F32 Multiplication instruction
pub fn f32_multiply(multiplicand: f32, multiplier: f32) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: borsh::to_vec(&MathInstruction::F32Multiply {
            multiplicand,
            multiplier,
        })
        .unwrap(),
    }
}

/// Create F32 Division instruction
pub fn f32_divide(dividend: f32, divisor: f32) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: borsh::to_vec(&MathInstruction::F32Divide { dividend, divisor }).unwrap(),
    }
}

/// Create F32 Exponentiate instruction
pub fn f32_exponentiate(base: f32, exponent: f32) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: borsh::to_vec(&MathInstruction::F32Exponentiate { base, exponent }).unwrap(),
    }
}

/// Create F32 Natural Log instruction
pub fn f32_natural_log(argument: f32) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: borsh::to_vec(&MathInstruction::F32NaturalLog { argument }).unwrap(),
    }
}

/// Create F32 Normal CDF instruction
pub fn f32_normal_cdf(argument: f32) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: borsh::to_vec(&MathInstruction::F32NormalCDF { argument }).unwrap(),
    }
}

/// Create F64Pow instruction
pub fn f64_pow(base: f64, exponent: f64) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: borsh::to_vec(&MathInstruction::F64Pow { base, exponent }).unwrap(),
    }
}

/// Create U128 Multiplication instruction
pub fn u128_multiply(multiplicand: u128, multiplier: u128) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: borsh::to_vec(&MathInstruction::U128Multiply {
            multiplicand,
            multiplier,
        })
        .unwrap(),
    }
}

/// Create U128 Division instruction
pub fn u128_divide(dividend: u128, divisor: u128) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: borsh::to_vec(&MathInstruction::U128Divide { dividend, divisor }).unwrap(),
    }
}

/// Create F64 Multiplication instruction
pub fn f64_multiply(multiplicand: f64, multiplier: f64) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: borsh::to_vec(&MathInstruction::F64Multiply {
            multiplicand,
            multiplier,
        })
        .unwrap(),
    }
}

/// Create F64 Division instruction
pub fn f64_divide(dividend: f64, divisor: f64) -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: borsh::to_vec(&MathInstruction::F64Divide { dividend, divisor }).unwrap(),
    }
}

/// Create Noop instruction
pub fn noop() -> Instruction {
    Instruction {
        program_id: id(),
        accounts: vec![],
        data: borsh::to_vec(&MathInstruction::Noop).unwrap(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_precise_sqrt() {
        let instruction = precise_sqrt(u64::MAX);
        assert_eq!(0, instruction.accounts.len());
        assert_eq!(
            instruction.data,
            borsh::to_vec(&MathInstruction::PreciseSquareRoot { radicand: u64::MAX }).unwrap()
        );
        assert_eq!(instruction.program_id, crate::id());
    }

    #[test]
    fn test_sqrt_u64() {
        let instruction = sqrt_u64(u64::MAX);
        assert_eq!(0, instruction.accounts.len());
        assert_eq!(
            instruction.data,
            borsh::to_vec(&MathInstruction::SquareRootU64 { radicand: u64::MAX }).unwrap()
        );
        assert_eq!(instruction.program_id, crate::id());
    }

    #[test]
    fn test_sqrt_u128() {
        let instruction = sqrt_u128(u128::MAX);
        assert_eq!(0, instruction.accounts.len());
        assert_eq!(
            instruction.data,
            borsh::to_vec(&MathInstruction::SquareRootU128 {
                radicand: u128::MAX
            })
            .unwrap()
        );
        assert_eq!(instruction.program_id, crate::id());
    }

    #[test]
    fn test_u64_multiply() {
        let instruction = u64_multiply(u64::MAX, u64::MAX);
        assert_eq!(0, instruction.accounts.len());
        assert_eq!(
            instruction.data,
            borsh::to_vec(&MathInstruction::U64Multiply {
                multiplicand: u64::MAX,
                multiplier: u64::MAX
            })
            .unwrap()
        );
        assert_eq!(instruction.program_id, crate::id());
    }

    #[test]
    fn test_u64_divide() {
        let instruction = u64_divide(u64::MAX, u64::MAX);
        assert_eq!(0, instruction.accounts.len());
        assert_eq!(
            instruction.data,
            borsh::to_vec(&MathInstruction::U64Divide {
                dividend: u64::MAX,
                divisor: u64::MAX
            })
            .unwrap()
        );
        assert_eq!(instruction.program_id, crate::id());
    }

    #[test]
    fn test_f32_multiply() {
        let instruction = f32_multiply(f32::MAX, f32::MAX);
        assert_eq!(0, instruction.accounts.len());
        assert_eq!(
            instruction.data,
            borsh::to_vec(&MathInstruction::F32Multiply {
                multiplicand: f32::MAX,
                multiplier: f32::MAX
            })
            .unwrap()
        );
        assert_eq!(instruction.program_id, crate::id());
    }

    #[test]
    fn test_f32_divide() {
        let instruction = f32_divide(f32::MAX, f32::MAX);
        assert_eq!(0, instruction.accounts.len());
        assert_eq!(
            instruction.data,
            borsh::to_vec(&MathInstruction::F32Divide {
                dividend: f32::MAX,
                divisor: f32::MAX
            })
            .unwrap()
        );
        assert_eq!(instruction.program_id, crate::id());
    }

    #[test]
    fn test_f32_exponentiate() {
        let instruction = f32_exponentiate(f32::MAX, f32::MAX);
        assert_eq!(0, instruction.accounts.len());
        assert_eq!(
            instruction.data,
            borsh::to_vec(&MathInstruction::F32Exponentiate {
                base: f32::MAX,
                exponent: f32::MAX
            })
            .unwrap()
        );
        assert_eq!(instruction.program_id, crate::id())
    }

    #[test]
    fn test_f32_natural_log() {
        let instruction = f32_natural_log(f32::MAX);
        assert_eq!(0, instruction.accounts.len());
        assert_eq!(
            instruction.data,
            borsh::to_vec(&MathInstruction::F32NaturalLog { argument: f32::MAX }).unwrap()
        );
        assert_eq!(instruction.program_id, crate::id())
    }

    #[test]
    fn test_f32_normal_cdf() {
        let instruction = f32_normal_cdf(f32::MAX);
        assert_eq!(0, instruction.accounts.len());
        assert_eq!(
            instruction.data,
            borsh::to_vec(&MathInstruction::F32NormalCDF { argument: f32::MAX }).unwrap()
        );
        assert_eq!(instruction.program_id, crate::id())
    }

    #[test]
    fn test_noop() {
        let instruction = noop();
        assert_eq!(0, instruction.accounts.len());
        assert_eq!(
            instruction.data,
            borsh::to_vec(&MathInstruction::Noop).unwrap()
        );
        assert_eq!(instruction.program_id, crate::id());
    }
}
