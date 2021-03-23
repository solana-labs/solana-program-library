//! State types

mod last_update;
mod lending_market;
mod obligation;
mod reserve;

pub use last_update::*;
pub use lending_market::*;
pub use obligation::*;
pub use reserve::*;

use crate::math::{Decimal, WAD};
use arrayref::{array_refs, mut_array_refs};
use solana_program::{
    clock::{DEFAULT_TICKS_PER_SECOND, DEFAULT_TICKS_PER_SLOT, SECONDS_PER_DAY},
    program_error::ProgramError,
    program_option::COption,
    pubkey::{Pubkey, PUBKEY_BYTES},
};

/// Collateral tokens are initially valued at a ratio of 5:1 (collateral:liquidity)
pub const INITIAL_COLLATERAL_RATIO: u64 = 5;
const INITIAL_COLLATERAL_RATE: u64 = INITIAL_COLLATERAL_RATIO * WAD;

/// Current version of the program and all new accounts created
pub const PROGRAM_VERSION: u8 = 1;

/// Accounts are created with data zeroed out, so uninitialized state instances
/// will have the version set to 0.
pub const UNINITIALIZED_VERSION: u8 = 0;

/// Number of slots per year
pub const SLOTS_PER_YEAR: u64 =
    DEFAULT_TICKS_PER_SECOND / DEFAULT_TICKS_PER_SLOT * SECONDS_PER_DAY * 365;

// Helpers
fn pack_coption_key(src: &COption<Pubkey>, dst: &mut [u8; 4 + PUBKEY_BYTES]) {
    let (tag, body) = mut_array_refs![dst, 4, PUBKEY_BYTES];
    match src {
        COption::Some(key) => {
            *tag = [1, 0, 0, 0];
            body.copy_from_slice(key.as_ref());
        }
        COption::None => {
            *tag = [0; 4];
        }
    }
}

fn unpack_coption_key(src: &[u8; 4 + PUBKEY_BYTES]) -> Result<COption<Pubkey>, ProgramError> {
    let (tag, body) = array_refs![src, 4, PUBKEY_BYTES];
    match *tag {
        [0, 0, 0, 0] => Ok(COption::None),
        [1, 0, 0, 0] => Ok(COption::Some(Pubkey::new_from_array(*body))),
        _ => Err(ProgramError::InvalidAccountData),
    }
}

fn pack_decimal(decimal: Decimal, dst: &mut [u8; 16]) {
    *dst = decimal
        .to_scaled_val()
        .expect("could not pack decimal")
        .to_le_bytes();
}

fn unpack_decimal(src: &[u8; 16]) -> Decimal {
    Decimal::from_scaled_val(u128::from_le_bytes(*src))
}

fn pack_bool(bool: bool, dst: &mut [u8; 1]) {
    *dst = (bool as u8).to_le_bytes()
}

fn unpack_bool(src: &[u8; 1]) -> Result<bool, ProgramError> {
    // @TODO bool::try_from(u8).ok() fails with
    //       ^^^^^^^^^^^^^^ the trait `std::convert::From<u8>` is not implemented for `bool`
    match u8::from_le_bytes(*src) {
        0 => Ok(false),
        1 => Ok(true),
        _ => Err(ProgramError::InvalidAccountData),
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn initial_collateral_rate_sanity() {
        assert_eq!(
            INITIAL_COLLATERAL_RATIO.checked_mul(WAD).unwrap(),
            INITIAL_COLLATERAL_RATE
        );
    }
}
