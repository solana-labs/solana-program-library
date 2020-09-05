//! State transition types

use crate::{
    error::Error,
    instruction::{unpack, Fee},
};
use solana_sdk::{entrypoint::ProgramResult, program_error::ProgramError, pubkey::Pubkey};
use std::mem::size_of;

/// Initialized program details.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SwapInfo {
    /// Nonce used in program address.
    /// The program address is created deterministically with the nonce,
    /// swap program id, and swap account pubkey.  This program address has
    /// authority over the swap's token A account, token B account, and pool
    /// token mint.
    pub nonce: u8,
    /// Token A
    /// The Liquidity token is issued against this value.
    pub token_a: Pubkey,
    /// Token B
    pub token_b: Pubkey,
    /// Pool tokens are issued when A or B tokens are deposited.
    /// Pool tokens can be withdrawn back to the original A or B token.
    pub pool_mint: Pubkey,
    /// Fee applied to the input token amount prior to output calculation.
    pub fee: Fee,
}

/// Program states.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
pub enum State {
    /// Unallocated state, may be initialized into another state.
    Unallocated,
    /// Initialized state.
    Init(SwapInfo),
}

impl State {
    /// Deserializes a byte buffer into a [State](struct.State.html).
    pub fn deserialize(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(match input[0] {
            0 => Self::Unallocated,
            1 => {
                let swap: &SwapInfo = unpack(input)?;
                Self::Init(*swap)
            }
            _ => return Err(ProgramError::InvalidAccountData),
        })
    }

    /// Serializes [State](struct.State.html) into a byte buffer.
    pub fn serialize(&self, output: &mut [u8]) -> ProgramResult {
        if output.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        match self {
            Self::Unallocated => output[0] = 0,
            Self::Init(swap) => {
                if output.len() < size_of::<u8>() + size_of::<SwapInfo>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                output[0] = 1;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut SwapInfo) };
                *value = *swap;
            }
        }
        Ok(())
    }

    /// Gets the `SwapInfo` from `State`
    pub fn token_swap(&self) -> Result<SwapInfo, ProgramError> {
        if let State::Init(swap) = &self {
            Ok(*swap)
        } else {
            Err(Error::InvalidState.into())
        }
    }
}

/// The Uniswap invariant calculator.
pub struct Invariant {
    /// Token A
    pub token_a: u64,
    /// Token B
    pub token_b: u64,
    /// Fee
    pub fee: Fee,
}

impl Invariant {
    /// Swap
    pub fn swap(&mut self, token_a: u64) -> Option<u64> {
        let invariant = self.token_a.checked_mul(self.token_b)?;
        let new_a = self.token_a.checked_add(token_a)?;
        let new_b = invariant.checked_div(new_a)?;
        let remove = self.token_b.checked_sub(new_b)?;
        let fee = remove
            .checked_mul(self.fee.numerator)?
            .checked_div(self.fee.denominator)?;
        let new_b_with_fee = new_b.checked_add(fee)?;
        let remove_less_fee = remove.checked_sub(fee)?;
        self.token_a = new_a;
        self.token_b = new_b_with_fee;
        Some(remove_less_fee)
    }
    /// Exchange rate
    pub fn exchange_rate(&self, token_a: u64) -> Option<u64> {
        token_a.checked_mul(self.token_b)?.checked_div(self.token_a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::mem::size_of;

    #[test]
    fn test_state_swap_info_deserialization() {
        let nonce = 255;
        let token_a_raw = [1u8; 32];
        let token_b_raw = [2u8; 32];
        let pool_mint_raw = [3u8; 32];
        let token_a = Pubkey::new_from_array(token_a_raw);
        let token_b = Pubkey::new_from_array(token_b_raw);
        let pool_mint = Pubkey::new_from_array(pool_mint_raw);
        let numerator = 1;
        let denominator = 4;
        let fee = Fee {
            numerator,
            denominator,
        };
        let state = State::Init(SwapInfo {
            nonce,
            token_a,
            token_b,
            pool_mint,
            fee,
        });

        let mut data = [0u8; size_of::<State>()];
        state.serialize(&mut data).unwrap();
        let deserialized = State::deserialize(&data).unwrap();
        assert_eq!(state, deserialized);

        let mut data = vec![];
        data.push(1 as u8);
        data.push(nonce);
        data.extend_from_slice(&token_a_raw);
        data.extend_from_slice(&token_b_raw);
        data.extend_from_slice(&pool_mint_raw);
        data.extend_from_slice(&[0u8; 7]); // padding
        data.push(denominator as u8);
        data.extend_from_slice(&[0u8; 7]); // padding
        data.push(numerator as u8);
        data.extend_from_slice(&[0u8; 7]); // padding
        data.extend_from_slice(&[0u8; 7]); // padding
        let deserialized = State::deserialize(&data).unwrap();
        assert_eq!(state, deserialized);
    }
}
