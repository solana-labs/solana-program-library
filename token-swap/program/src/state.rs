//! State transition types

use crate::error::SwapError;
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_sdk::{program_error::ProgramError, pubkey::Pubkey};

/// Program states.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SwapInfo {
    /// Initialized state.
    pub is_initialized: bool,
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
    /// Numerator of fee applied to the input token amount prior to output calculation.
    pub fee_numerator: u64,
    /// Denominator of fee applied to the input token amount prior to output calculation.
    pub fee_denominator: u64,
}

impl SwapInfo {
    /// Helper function to get the more efficient packed size of the struct
    const fn get_packed_len() -> usize {
        114
    }

    /// Unpacks a byte buffer into a [SwapInfo](struct.SwapInfo.html) and checks
    /// that it is initialized.
    pub fn unpack(input: &[u8]) -> Result<Self, ProgramError> {
        let value = Self::unpack_unchecked(input)?;
        if value.is_initialized {
            Ok(value)
        } else {
            Err(SwapError::InvalidSwapInfo.into())
        }
    }

    /// Unpacks a byte buffer into a [SwapInfo](struct.SwapInfo.html).
    pub fn unpack_unchecked(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, SwapInfo::get_packed_len()];
        #[allow(clippy::ptr_offset_with_cast)]
        let (is_initialized, nonce, token_a, token_b, pool_mint, fee_numerator, fee_denominator) =
            array_refs![input, 1, 1, 32, 32, 32, 8, 8];
        Ok(Self {
            is_initialized: match is_initialized {
                [0] => false,
                [1] => true,
                _ => return Err(ProgramError::InvalidAccountData),
            },
            nonce: nonce[0],
            token_a: Pubkey::new_from_array(*token_a),
            token_b: Pubkey::new_from_array(*token_b),
            pool_mint: Pubkey::new_from_array(*pool_mint),
            fee_numerator: u64::from_le_bytes(*fee_numerator),
            fee_denominator: u64::from_le_bytes(*fee_denominator),
        })
    }

    /// Packs [SwapInfo](struct.SwapInfo.html) into a byte buffer.
    pub fn pack(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, SwapInfo::get_packed_len()];
        let (is_initialized, nonce, token_a, token_b, pool_mint, fee_numerator, fee_denominator) =
            mut_array_refs![output, 1, 1, 32, 32, 32, 8, 8];
        is_initialized[0] = self.is_initialized as u8;
        nonce[0] = self.nonce;
        token_a.copy_from_slice(self.token_a.as_ref());
        token_b.copy_from_slice(self.token_b.as_ref());
        pool_mint.copy_from_slice(self.pool_mint.as_ref());
        *fee_numerator = self.fee_numerator.to_le_bytes();
        *fee_denominator = self.fee_denominator.to_le_bytes();
    }
}

/// Encodes all results of swapping from a source token to a destination token
pub struct SwapResult {
    /// New amount of source token
    pub new_source: u64,
    /// New amount of destination token
    pub new_destination: u64,
    /// Amount of destination token swapped
    pub amount_swapped: u64,
}

impl SwapResult {
    /// SwapResult for swap from one currency into another, given pool information
    /// and fee
    pub fn swap_to(
        source: u64,
        source_amount: u64,
        dest_amount: u64,
        fee_numerator: u64,
        fee_denominator: u64,
    ) -> Option<SwapResult> {
        let invariant = source_amount.checked_mul(dest_amount)?;
        let new_source = source_amount.checked_add(source)?;
        let new_destination = invariant.checked_div(new_source)?;
        let remove = dest_amount.checked_sub(new_destination)?;
        let fee = remove
            .checked_mul(fee_numerator)?
            .checked_div(fee_denominator)?;
        let new_destination = new_destination.checked_add(fee)?;
        let amount_swapped = remove.checked_sub(fee)?;
        Some(SwapResult {
            new_source,
            new_destination,
            amount_swapped,
        })
    }
}

/// The Uniswap invariant calculator.
pub struct Invariant {
    /// Token A
    pub token_a: u64,
    /// Token B
    pub token_b: u64,
    /// Fee numerator
    pub fee_numerator: u64,
    /// Fee denominator
    pub fee_denominator: u64,
}

impl Invariant {
    /// Swap token a to b
    pub fn swap_a_to_b(&mut self, token_a: u64) -> Option<u64> {
        let result = SwapResult::swap_to(
            token_a,
            self.token_a,
            self.token_b,
            self.fee_numerator,
            self.fee_denominator,
        )?;
        self.token_a = result.new_source;
        self.token_b = result.new_destination;
        Some(result.amount_swapped)
    }

    /// Swap token b to a
    pub fn swap_b_to_a(&mut self, token_b: u64) -> Option<u64> {
        let result = SwapResult::swap_to(
            token_b,
            self.token_b,
            self.token_a,
            self.fee_numerator,
            self.fee_denominator,
        )?;
        self.token_b = result.new_source;
        self.token_a = result.new_destination;
        Some(result.amount_swapped)
    }

    /// Exchange rate
    pub fn exchange_rate(&self, token_a: u64) -> Option<u64> {
        token_a.checked_mul(self.token_b)?.checked_div(self.token_a)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_swap_info_packing() {
        let nonce = 255;
        let token_a_raw = [1u8; 32];
        let token_b_raw = [2u8; 32];
        let pool_mint_raw = [3u8; 32];
        let token_a = Pubkey::new_from_array(token_a_raw);
        let token_b = Pubkey::new_from_array(token_b_raw);
        let pool_mint = Pubkey::new_from_array(pool_mint_raw);
        let fee_numerator = 1;
        let fee_denominator = 4;
        let is_initialized = true;
        let swap_info = SwapInfo {
            is_initialized,
            nonce,
            token_a,
            token_b,
            pool_mint,
            fee_numerator,
            fee_denominator,
        };

        let mut packed = [0u8; SwapInfo::get_packed_len()];
        swap_info.pack(&mut packed);
        let unpacked = SwapInfo::unpack(&packed).unwrap();
        assert_eq!(swap_info, unpacked);

        let mut packed = vec![];
        packed.push(1 as u8);
        packed.push(nonce);
        packed.extend_from_slice(&token_a_raw);
        packed.extend_from_slice(&token_b_raw);
        packed.extend_from_slice(&pool_mint_raw);
        packed.push(fee_numerator as u8);
        packed.extend_from_slice(&[0u8; 7]); // padding
        packed.push(fee_denominator as u8);
        packed.extend_from_slice(&[0u8; 7]); // padding
        let unpacked = SwapInfo::unpack(&packed).unwrap();
        assert_eq!(swap_info, unpacked);

        let packed = [0u8; SwapInfo::get_packed_len()];
        let swap_info: SwapInfo = Default::default();
        let unpack_unchecked = SwapInfo::unpack_unchecked(&packed).unwrap();
        assert_eq!(unpack_unchecked, swap_info);
        let err = SwapInfo::unpack(&packed).unwrap_err();
        assert_eq!(err, SwapError::InvalidSwapInfo.into());
    }
}
