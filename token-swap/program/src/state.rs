//! State transition types

use crate::instruction::Fee;
use solana_sdk::pubkey::Pubkey;

/// Initialized program details.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct SwapInfo {
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
