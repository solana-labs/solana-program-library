//! Common math routines.

use {
    solana_program::{msg, program_error::ProgramError},
    std::fmt::Display,
};

pub fn checked_add<T>(arg1: T, arg2: T) -> Result<T, ProgramError>
where
    T: num_traits::PrimInt + Display,
{
    if let Some(res) = arg1.checked_add(&arg2) {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} + {}", arg1, arg2);
        Err(ProgramError::Custom(999))
    }
}

pub fn checked_sub<T>(arg1: T, arg2: T) -> Result<T, ProgramError>
where
    T: num_traits::PrimInt + Display,
{
    if let Some(res) = arg1.checked_sub(&arg2) {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} - {}", arg1, arg2);
        Err(ProgramError::Custom(999))
    }
}

pub fn checked_div<T>(arg1: T, arg2: T) -> Result<T, ProgramError>
where
    T: num_traits::PrimInt + Display,
{
    if let Some(res) = arg1.checked_div(&arg2) {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} / {}", arg1, arg2);
        Err(ProgramError::Custom(999))
    }
}

pub fn checked_mul<T>(arg1: T, arg2: T) -> Result<T, ProgramError>
where
    T: num_traits::PrimInt + Display,
{
    if let Some(res) = arg1.checked_mul(&arg2) {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} * {}", arg1, arg2);
        Err(ProgramError::Custom(999))
    }
}

pub fn checked_as_u64<T>(arg: T) -> Result<u64, ProgramError>
where
    T: Display + num_traits::ToPrimitive + Clone,
{
    let option: Option<u64> = num_traits::NumCast::from(arg.clone());
    if let Some(res) = option {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} as u64", arg);
        Err(ProgramError::Custom(999))
    }
}
