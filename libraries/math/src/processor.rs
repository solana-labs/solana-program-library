//! Program state processor

use {
    crate::{approximations::sqrt, instruction::MathInstruction, precise_number::PreciseNumber},
    borsh::BorshDeserialize,
    solana_program::{
        account_info::AccountInfo, entrypoint::ProgramResult, log::sol_log_compute_units, msg,
        pubkey::Pubkey,
    },
};

/// u64_multiply
#[inline(never)]
fn u64_multiply(multiplicand: u64, multiplier: u64) -> u64 {
    multiplicand * multiplier
}

/// u64_divide
#[inline(never)]
fn u64_divide(dividend: u64, divisor: u64) -> u64 {
    dividend / divisor
}

/// f32_multiply
#[inline(never)]
fn f32_multiply(multiplicand: f32, multiplier: f32) -> f32 {
    multiplicand * multiplier
}

/// f32_divide
#[inline(never)]
fn f32_divide(dividend: f32, divisor: f32) -> f32 {
    dividend / divisor
}

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
            sol_log_compute_units();
            let result = radicand.sqrt().unwrap().to_imprecise().unwrap() as u64;
            sol_log_compute_units();
            msg!("{}", result);
            Ok(())
        }
        MathInstruction::SquareRootU64 { radicand } => {
            msg!("Calculating u64 square root");
            sol_log_compute_units();
            let result = sqrt(radicand).unwrap();
            sol_log_compute_units();
            msg!("{}", result);
            Ok(())
        }
        MathInstruction::SquareRootU128 { radicand } => {
            msg!("Calculating u128 square root");
            sol_log_compute_units();
            let result = sqrt(radicand).unwrap();
            sol_log_compute_units();
            msg!("{}", result);
            Ok(())
        }
        MathInstruction::U64Multiply {
            multiplicand,
            multiplier,
        } => {
            msg!("Calculating U64 Multiply");
            sol_log_compute_units();
            let result = u64_multiply(multiplicand, multiplier);
            sol_log_compute_units();
            msg!("{}", result);
            Ok(())
        }
        MathInstruction::U64Divide { dividend, divisor } => {
            msg!("Calculating U64 Divide");
            sol_log_compute_units();
            let result = u64_divide(dividend, divisor);
            sol_log_compute_units();
            msg!("{}", result);
            Ok(())
        }
        MathInstruction::F32Multiply {
            multiplicand,
            multiplier,
        } => {
            msg!("Calculating f32 Multiply");
            sol_log_compute_units();
            let result = f32_multiply(multiplicand, multiplier);
            sol_log_compute_units();
            msg!("{}", result as u64);
            Ok(())
        }
        MathInstruction::F32Divide { dividend, divisor } => {
            msg!("Calculating f32 Divide");
            sol_log_compute_units();
            let result = f32_divide(dividend, divisor);
            sol_log_compute_units();
            msg!("{}", result as u64);
            Ok(())
        }
        MathInstruction::Noop => {
            msg!("Do nothing");
            msg!("{}", 0_u64);
            Ok(())
        }
    }
}

#[cfg(test)]
mod tests {
    use borsh::BorshSerialize;
    use crate::instruction::MathInstruction;
    use super::*;
    use solana_program::account_info::AccountInfo;

    #[test]
    fn test_u64_multiply() {
        assert_eq!(4, u64_multiply(2, 2));
        assert_eq!(12, u64_multiply(4, 3));
    }

    #[test]
    fn test_u64_divide() {
        assert_eq!(1, u64_divide(2, 2));
        assert_eq!(2, u64_divide(2, 1));
    }

    #[test]
    fn test_f32_multiply() {
        assert_eq!(4.0, f32_multiply(2.0, 2.0));
        assert_eq!(12.0, f32_multiply(4.0, 3.0));
    }

    #[test]
    fn test_f32_divide() {
        assert_eq!(1.0, f32_divide(2.0, 2.0));
        assert_eq!(2.0, f32_divide(2.0, 1.0));
    }

    #[allow(clippy::unit_cmp)]
    #[test]
    fn test_process_instruction() {
        let program_id = Pubkey::new_unique();
        let mut data = vec![];
        let mut lamports = 1000;
        let rent_epoch = 1;
        let account_info = AccountInfo::new(
            &program_id,
            true,
            true,
            &mut lamports,
            &mut data,
            &program_id,
            true,
            rent_epoch
        );

        let math_instruction = MathInstruction::PreciseSquareRoot { radicand: 18446744073709551615 };
        let input = math_instruction.try_to_vec().unwrap();
        let instruction = process_instruction(&program_id, &[account_info.clone()], &input).unwrap();
        assert_eq!((), instruction);

        let math_instruction = MathInstruction::SquareRootU64 { radicand: 18446744073709551615 };
        let input = math_instruction.try_to_vec().unwrap();
        let instruction = process_instruction(&program_id, &[account_info.clone()], &input).unwrap();
        assert_eq!((), instruction);

        let math_instruction = MathInstruction::SquareRootU128 { radicand: 340282366920938463463374607431768211455 };
        let input = math_instruction.try_to_vec().unwrap();
        let instruction = process_instruction(&program_id, &[account_info.clone()], &input).unwrap();
        assert_eq!((), instruction);

        let math_instruction = MathInstruction::U64Multiply {
            multiplicand: 3,
            multiplier: 4,
        };
        let input = math_instruction.try_to_vec().unwrap();
        let instruction = process_instruction(&program_id, &[account_info.clone()], &input).unwrap();
        assert_eq!((), instruction);

        let math_instruction = MathInstruction::U64Divide {
            dividend: 2,
            divisor: 2,
        };
        let input = math_instruction.try_to_vec().unwrap();
        let instruction = process_instruction(&program_id, &[account_info.clone()], &input).unwrap();
        assert_eq!((), instruction);

        let math_instruction = MathInstruction::F32Multiply {
            multiplicand: 3.0,
            multiplier: 4.0,
        };
        let input = math_instruction.try_to_vec().unwrap();
        let instruction = process_instruction(&program_id, &[account_info.clone()], &input).unwrap();
        assert_eq!((), instruction);

        let math_instruction = MathInstruction::F32Divide {
            dividend: 2.0,
            divisor: 2.0,
        };
        let input = math_instruction.try_to_vec().unwrap();
        let instruction = process_instruction(&program_id, &[account_info.clone()], &input).unwrap();
        assert_eq!((), instruction);

        let math_instruction = MathInstruction::Noop;
        let input = math_instruction.try_to_vec().unwrap();
        let instruction = process_instruction(&program_id, &[account_info.clone()], &input).unwrap();
        assert_eq!((), instruction);
    }
}
