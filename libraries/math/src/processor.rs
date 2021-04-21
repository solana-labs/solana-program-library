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
