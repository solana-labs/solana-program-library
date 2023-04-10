//! Common math routines.

#![allow(dead_code)]

use {crate::error::PerpetualsError, anchor_lang::prelude::*, std::fmt::Display};

pub fn checked_add<T>(arg1: T, arg2: T) -> Result<T>
where
    T: num_traits::PrimInt + Display,
{
    if let Some(res) = arg1.checked_add(&arg2) {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} + {}", arg1, arg2);
        err!(PerpetualsError::MathOverflow)
    }
}

pub fn checked_sub<T>(arg1: T, arg2: T) -> Result<T>
where
    T: num_traits::PrimInt + Display,
{
    if let Some(res) = arg1.checked_sub(&arg2) {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} - {}", arg1, arg2);
        err!(PerpetualsError::MathOverflow)
    }
}

pub fn checked_div<T>(arg1: T, arg2: T) -> Result<T>
where
    T: num_traits::PrimInt + Display,
{
    if let Some(res) = arg1.checked_div(&arg2) {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} / {}", arg1, arg2);
        err!(PerpetualsError::MathOverflow)
    }
}

pub fn checked_float_div<T>(arg1: T, arg2: T) -> Result<T>
where
    T: num_traits::Float + Display,
{
    if arg2 == T::zero() {
        msg!("Error: Overflow in {} / {}", arg1, arg2);
        return err!(PerpetualsError::MathOverflow);
    }
    let res = arg1 / arg2;
    if !res.is_finite() {
        msg!("Error: Overflow in {} / {}", arg1, arg2);
        err!(PerpetualsError::MathOverflow)
    } else {
        Ok(res)
    }
}

pub fn checked_ceil_div<T>(arg1: T, arg2: T) -> Result<T>
where
    T: num_traits::PrimInt + Display,
{
    if arg1 > T::zero() {
        if arg1 == arg2 && arg2 != T::zero() {
            return Ok(T::one());
        }
        if let Some(res) = (arg1 - T::one()).checked_div(&arg2) {
            Ok(res + T::one())
        } else {
            msg!("Error: Overflow in {} / {}", arg1, arg2);
            err!(PerpetualsError::MathOverflow)
        }
    } else if let Some(res) = arg1.checked_div(&arg2) {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} / {}", arg1, arg2);
        err!(PerpetualsError::MathOverflow)
    }
}

pub fn checked_decimal_div(
    coefficient1: u64,
    exponent1: i32,
    coefficient2: u64,
    exponent2: i32,
    target_exponent: i32,
) -> Result<u64> {
    if coefficient2 == 0 {
        msg!("Error: Overflow in {} / {}", coefficient1, coefficient2);
        return err!(PerpetualsError::MathOverflow);
    }
    if coefficient1 == 0 {
        return Ok(0);
    }
    // compute scale factor for the dividend
    let mut scale_factor = 0;
    let mut target_power = checked_sub(checked_sub(exponent1, exponent2)?, target_exponent)?;
    if exponent1 > 0 {
        scale_factor = checked_add(scale_factor, exponent1)?;
    }
    if exponent2 < 0 {
        scale_factor = checked_sub(scale_factor, exponent2)?;
        target_power = checked_add(target_power, exponent2)?;
    }
    if target_exponent < 0 {
        scale_factor = checked_sub(scale_factor, target_exponent)?;
        target_power = checked_add(target_power, target_exponent)?;
    }
    let scaled_coeff1 = if scale_factor > 0 {
        checked_mul(
            coefficient1 as u128,
            checked_pow(10u128, scale_factor as usize)?,
        )?
    } else {
        coefficient1 as u128
    };

    if target_power >= 0 {
        checked_as_u64(checked_mul(
            checked_div(scaled_coeff1, coefficient2 as u128)?,
            checked_pow(10u128, target_power as usize)?,
        )?)
    } else {
        checked_as_u64(checked_div(
            checked_div(scaled_coeff1, coefficient2 as u128)?,
            checked_pow(10u128, (-target_power) as usize)?,
        )?)
    }
}

pub fn checked_decimal_ceil_div(
    coefficient1: u64,
    exponent1: i32,
    coefficient2: u64,
    exponent2: i32,
    target_exponent: i32,
) -> Result<u64> {
    if coefficient2 == 0 {
        msg!("Error: Overflow in {} / {}", coefficient1, coefficient2);
        return err!(PerpetualsError::MathOverflow);
    }
    if coefficient1 == 0 {
        return Ok(0);
    }
    // compute scale factor for the dividend
    let mut scale_factor = 0;
    let mut target_power = checked_sub(checked_sub(exponent1, exponent2)?, target_exponent)?;
    if exponent1 > 0 {
        scale_factor = checked_add(scale_factor, exponent1)?;
    }
    if exponent2 < 0 {
        scale_factor = checked_sub(scale_factor, exponent2)?;
        target_power = checked_add(target_power, exponent2)?;
    }
    if target_exponent < 0 {
        scale_factor = checked_sub(scale_factor, target_exponent)?;
        target_power = checked_add(target_power, target_exponent)?;
    }
    let scaled_coeff1 = if scale_factor > 0 {
        checked_mul(
            coefficient1 as u128,
            checked_pow(10u128, scale_factor as usize)?,
        )?
    } else {
        coefficient1 as u128
    };

    if target_power >= 0 {
        checked_as_u64(checked_mul(
            checked_ceil_div(scaled_coeff1, coefficient2 as u128)?,
            checked_pow(10u128, target_power as usize)?,
        )?)
    } else {
        checked_as_u64(checked_div(
            checked_ceil_div(scaled_coeff1, coefficient2 as u128)?,
            checked_pow(10u128, (-target_power) as usize)?,
        )?)
    }
}

pub fn checked_token_div(
    amount1: u64,
    decimals1: u8,
    amount2: u64,
    decimals2: u8,
) -> Result<(u64, u8)> {
    let target_decimals = std::cmp::max(decimals1, decimals2);
    Ok((
        checked_decimal_div(
            amount1,
            -(decimals1 as i32),
            amount2,
            -(decimals2 as i32),
            -(target_decimals as i32),
        )?,
        target_decimals,
    ))
}

pub fn checked_mul<T>(arg1: T, arg2: T) -> Result<T>
where
    T: num_traits::PrimInt + Display,
{
    if let Some(res) = arg1.checked_mul(&arg2) {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} * {}", arg1, arg2);
        err!(PerpetualsError::MathOverflow)
    }
}

pub fn checked_float_mul<T>(arg1: T, arg2: T) -> Result<T>
where
    T: num_traits::Float + Display,
{
    let res = arg1 * arg2;
    if !res.is_finite() {
        msg!("Error: Overflow in {} * {}", arg1, arg2);
        err!(PerpetualsError::MathOverflow)
    } else {
        Ok(res)
    }
}

pub fn checked_decimal_mul(
    coefficient1: u64,
    exponent1: i32,
    coefficient2: u64,
    exponent2: i32,
    target_exponent: i32,
) -> Result<u64> {
    if coefficient1 == 0 || coefficient2 == 0 {
        return Ok(0);
    }
    let target_power = checked_sub(checked_add(exponent1, exponent2)?, target_exponent)?;
    if target_power >= 0 {
        checked_as_u64(checked_mul(
            checked_mul(coefficient1 as u128, coefficient2 as u128)?,
            checked_pow(10u128, target_power as usize)?,
        )?)
    } else {
        checked_as_u64(checked_div(
            checked_mul(coefficient1 as u128, coefficient2 as u128)?,
            checked_pow(10u128, (-target_power) as usize)?,
        )?)
    }
}

pub fn checked_decimal_ceil_mul(
    coefficient1: u64,
    exponent1: i32,
    coefficient2: u64,
    exponent2: i32,
    target_exponent: i32,
) -> Result<u64> {
    if coefficient1 == 0 || coefficient2 == 0 {
        return Ok(0);
    }
    let target_power = checked_sub(checked_add(exponent1, exponent2)?, target_exponent)?;
    if target_power >= 0 {
        checked_as_u64(checked_mul(
            checked_mul(coefficient1 as u128, coefficient2 as u128)?,
            checked_pow(10u128, target_power as usize)?,
        )?)
    } else {
        checked_as_u64(checked_ceil_div(
            checked_mul(coefficient1 as u128, coefficient2 as u128)?,
            checked_pow(10u128, (-target_power) as usize)?,
        )?)
    }
}

pub fn checked_token_mul(
    amount1: u64,
    decimals1: u8,
    amount2: u64,
    decimals2: u8,
) -> Result<(u64, u8)> {
    let target_decimals = std::cmp::max(decimals1, decimals2);
    Ok((
        checked_decimal_mul(
            amount1,
            -(decimals1 as i32),
            amount2,
            -(decimals2 as i32),
            -(target_decimals as i32),
        )?,
        target_decimals,
    ))
}

pub fn checked_pow<T>(arg: T, exp: usize) -> Result<T>
where
    T: num_traits::PrimInt + Display,
{
    if let Some(res) = num_traits::checked_pow(arg, exp) {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} ^ {}", arg, exp);
        err!(PerpetualsError::MathOverflow)
    }
}

pub fn checked_powf(arg: f64, exp: f64) -> Result<f64> {
    let res = f64::powf(arg, exp);
    if res.is_finite() {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} ^ {}", arg, exp);
        err!(PerpetualsError::MathOverflow)
    }
}

pub fn checked_powi(arg: f64, exp: i32) -> Result<f64> {
    let res = if exp > 0 {
        f64::powi(arg, exp)
    } else {
        // wrokaround due to f64::powi() not working properly on-chain with negative exponent
        checked_float_div(1.0, f64::powi(arg, -exp))?
    };
    if res.is_finite() {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} ^ {}", arg, exp);
        err!(PerpetualsError::MathOverflow)
    }
}

pub fn checked_as_u64<T>(arg: T) -> Result<u64>
where
    T: Display + num_traits::ToPrimitive + Clone,
{
    let option: Option<u64> = num_traits::NumCast::from(arg.clone());
    if let Some(res) = option {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} as u64", arg);
        err!(PerpetualsError::MathOverflow)
    }
}

pub fn checked_as_u128<T>(arg: T) -> Result<u128>
where
    T: Display + num_traits::ToPrimitive + Clone,
{
    let option: Option<u128> = num_traits::NumCast::from(arg.clone());
    if let Some(res) = option {
        Ok(res)
    } else {
        msg!("Error: Overflow in {} as u128", arg);
        err!(PerpetualsError::MathOverflow)
    }
}

pub fn scale_to_exponent(arg: u64, exponent: i32, target_exponent: i32) -> Result<u64> {
    if target_exponent == exponent {
        return Ok(arg);
    }
    let delta = checked_sub(target_exponent, exponent)?;
    if delta > 0 {
        checked_div(arg, checked_pow(10, delta as usize)?)
    } else {
        checked_mul(arg, checked_pow(10, (-delta) as usize)?)
    }
}

pub fn to_ui_amount(amount: u64, decimals: u8) -> Result<f64> {
    checked_float_div(amount as f64, checked_powi(10.0, decimals as i32)?)
}

pub fn to_token_amount(ui_amount: f64, decimals: u8) -> Result<u64> {
    checked_as_u64(checked_float_mul(
        ui_amount,
        checked_powi(10.0, decimals as i32)?,
    )?)
}
