use super::*;
use crate::{
    error::LendingError,
    math::{Decimal, Rate, TryDiv, TryMul},
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use std::convert::TryInto;

/// Borrow obligation state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Obligation {
    /// Version of the obligation
    pub version: u8,
    /// Amount of collateral tokens deposited for this obligation
    pub deposited_collateral_tokens: u64,
    /// Reserve which collateral tokens were deposited into
    pub collateral_reserve: Pubkey,
    /// Borrow rate used for calculating interest.
    pub cumulative_borrow_rate_wads: Decimal,
    /// Amount of tokens borrowed for this obligation plus interest
    pub borrowed_liquidity_wads: Decimal,
    /// Reserve which tokens were borrowed from
    pub borrow_reserve: Pubkey,
    /// Mint address of the tokens for this obligation
    pub token_mint: Pubkey,
}

impl Obligation {
    /// Accrue interest
    pub fn accrue_interest(&mut self, cumulative_borrow_rate: Decimal) -> Result<(), ProgramError> {
        if cumulative_borrow_rate < self.cumulative_borrow_rate_wads {
            return Err(LendingError::NegativeInterestRate.into());
        }

        let compounded_interest_rate: Rate = cumulative_borrow_rate
            .try_div(self.cumulative_borrow_rate_wads)?
            .try_into()?;

        self.borrowed_liquidity_wads = self
            .borrowed_liquidity_wads
            .try_mul(compounded_interest_rate)?;

        self.cumulative_borrow_rate_wads = cumulative_borrow_rate;

        Ok(())
    }
}

impl Sealed for Obligation {}
impl IsInitialized for Obligation {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

const OBLIGATION_LEN: usize = 265;
impl Pack for Obligation {
    const LEN: usize = 265;

    /// Unpacks a byte buffer into a [ObligationInfo](struct.ObligationInfo.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, OBLIGATION_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            deposited_collateral_tokens,
            collateral_supply,
            cumulative_borrow_rate,
            borrowed_liquidity_wads,
            borrow_reserve,
            token_mint,
            _padding,
        ) = array_refs![input, 1, 8, 32, 16, 16, 32, 32, 128];
        Ok(Self {
            version: u8::from_le_bytes(*version),
            deposited_collateral_tokens: u64::from_le_bytes(*deposited_collateral_tokens),
            collateral_reserve: Pubkey::new_from_array(*collateral_supply),
            cumulative_borrow_rate_wads: unpack_decimal(cumulative_borrow_rate),
            borrowed_liquidity_wads: unpack_decimal(borrowed_liquidity_wads),
            borrow_reserve: Pubkey::new_from_array(*borrow_reserve),
            token_mint: Pubkey::new_from_array(*token_mint),
        })
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, OBLIGATION_LEN];
        let (
            version,
            deposited_collateral_tokens,
            collateral_supply,
            cumulative_borrow_rate,
            borrowed_liquidity_wads,
            borrow_reserve,
            token_mint,
            _padding,
        ) = mut_array_refs![output, 1, 8, 32, 16, 16, 32, 32, 128];

        *version = self.version.to_le_bytes();
        *deposited_collateral_tokens = self.deposited_collateral_tokens.to_le_bytes();
        collateral_supply.copy_from_slice(self.collateral_reserve.as_ref());
        pack_decimal(self.cumulative_borrow_rate_wads, cumulative_borrow_rate);
        pack_decimal(self.borrowed_liquidity_wads, borrowed_liquidity_wads);
        borrow_reserve.copy_from_slice(self.borrow_reserve.as_ref());
        token_mint.copy_from_slice(self.token_mint.as_ref());
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::math::TryAdd;
    use proptest::prelude::*;

    const MAX_COMPOUNDED_INTEREST: u64 = 100; // 10,000%

    #[test]
    fn obligation_accrue_interest_failure() {
        assert_eq!(
            Obligation {
                cumulative_borrow_rate_wads: Decimal::zero(),
                ..Obligation::default()
            }
            .accrue_interest(Decimal::one()),
            Err(LendingError::MathOverflow.into())
        );

        assert_eq!(
            Obligation {
                cumulative_borrow_rate_wads: Decimal::from(2u64),
                ..Obligation::default()
            }
            .accrue_interest(Decimal::one()),
            Err(LendingError::NegativeInterestRate.into())
        );

        assert_eq!(
            Obligation {
                cumulative_borrow_rate_wads: Decimal::one(),
                borrowed_liquidity_wads: Decimal::from(u64::MAX),
                ..Obligation::default()
            }
            .accrue_interest(Decimal::from(10 * MAX_COMPOUNDED_INTEREST)),
            Err(LendingError::MathOverflow.into())
        );
    }

    // Creates rates (r1, r2) where 0 < r1 <= r2 <= 100*r1
    fn cumulative_rates() -> impl Strategy<Value = (u128, u128)> {
        prop::num::u128::ANY.prop_flat_map(|rate| {
            let current_rate = rate.max(1);
            let max_new_rate = current_rate.saturating_mul(MAX_COMPOUNDED_INTEREST as u128);
            (Just(current_rate), current_rate..=max_new_rate)
        })
    }

    proptest! {
        #[test]
        fn obligation_accrue_interest(
            borrowed_liquidity in 0..=u64::MAX,
            (current_borrow_rate, new_borrow_rate) in cumulative_rates(),
        ) {
            let borrowed_liquidity_wads = Decimal::from(borrowed_liquidity);
            let cumulative_borrow_rate_wads = Decimal::one().try_add(Decimal::from_scaled_val(current_borrow_rate))?;
            let mut state = Obligation {
                borrowed_liquidity_wads,
                cumulative_borrow_rate_wads,
                ..Obligation::default()
            };

            let next_cumulative_borrow_rate = Decimal::one().try_add(Decimal::from_scaled_val(new_borrow_rate))?;
            state.accrue_interest(next_cumulative_borrow_rate)?;

            if next_cumulative_borrow_rate > cumulative_borrow_rate_wads {
                assert!(state.borrowed_liquidity_wads > borrowed_liquidity_wads);
            } else {
                assert!(state.borrowed_liquidity_wads == borrowed_liquidity_wads);
            }
        }
    }
}
