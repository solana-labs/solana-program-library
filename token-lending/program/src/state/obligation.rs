use super::*;
use crate::{
    error::LendingError,
    math::{Decimal, Rate, TryAdd, TryDiv, TryMul, TrySub},
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    clock::Slot,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use std::convert::{TryFrom, TryInto};

// @TODO: rename / relocate; true max is potentially 28
/// Max number of collateral and liquidity accounts combined for an obligation
pub const MAX_OBLIGATION_ACCOUNTS: usize = 10;

/// Number of slots market values are considered stale after
pub const STALE_AFTER_SLOTS: u64 = 10;

/// Borrow obligation state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Obligation {
    /// Version of the struct
    pub version: u8,
    /// Last slot when loan to value updated; set to 0 if collateral or liquidity changed
    pub last_update_slot: Slot,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Collateral market value in quote currency
    pub collateral_value: Decimal,
    /// Liquidity market value in quote currency
    pub liquidity_value: Decimal,
    /// Collateral accounts for the obligation
    pub collateral: Vec<Pubkey>,
    /// Liquidity accounts for the obligation
    pub liquidity: Vec<Pubkey>,
}

/// Create new obligation
pub struct NewObligationParams {
    /// Lending market address
    pub lending_market: Pubkey,
    /// Collateral for the obligation
    pub collateral: Vec<Pubkey>,
    /// Liquidity for the obligation
    pub liquidity: Vec<Pubkey>,
}

impl Obligation {
    /// Create new obligation
    pub fn new(params: NewObligationParams) -> Self {
        let NewObligationParams {
            lending_market,
            collateral,
            liquidity,
        } = params;

        Self {
            version: PROGRAM_VERSION,
            last_update_slot: 0,
            lending_market,
            collateral_value: Decimal::zero(),
            liquidity_value: Decimal::zero(),
            collateral,
            liquidity,
        }
    }

    // @FIXME
    /// Liquidate part of obligation
    pub fn liquidate(&mut self, repay_amount: Decimal, withdraw_amount: u64) -> ProgramResult {
        self.borrowed_wads = self.borrowed_wads.try_sub(repay_amount)?;
        self.deposited_collateral_tokens = self
            .deposited_collateral_tokens
            .checked_sub(withdraw_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    // @FIXME
    /// Repay borrowed tokens
    pub fn repay(
        &mut self,
        liquidity_amount: u64,
        obligation_token_supply: u64,
    ) -> Result<RepayResult, ProgramError> {
        let decimal_repay_amount = Decimal::from(liquidity_amount).min(self.borrowed_wads);
        let integer_repay_amount = decimal_repay_amount.try_ceil_u64()?;
        if integer_repay_amount == 0 {
            return Err(LendingError::ObligationEmpty.into());
        }

        let repay_pct: Decimal = decimal_repay_amount.try_div(self.borrowed_wads)?;
        let collateral_withdraw_amount = {
            let withdraw_amount: Decimal = repay_pct.try_mul(self.deposited_collateral_tokens)?;
            withdraw_amount.try_floor_u64()?
        };

        let obligation_token_amount = self.collateral_to_obligation_token_amount(
            collateral_withdraw_amount,
            obligation_token_supply,
        )?;

        self.liquidate(decimal_repay_amount, collateral_withdraw_amount)?;

        Ok(RepayResult {
            decimal_repay_amount,
            integer_repay_amount,
            collateral_withdraw_amount,
            obligation_token_amount,
        })
    }

    /// Calculate the ratio of liquidity market value to collateral market value
    pub fn loan_to_value(&self) -> Result<Decimal, ProgramError> {
        self.liquidity_value.try_div(self.collateral_value)
    }

    /// Return slots elapsed since given slot
    pub fn slots_elapsed(&self, slot: Slot) -> Result<u64, ProgramError> {
        let slots_elapsed = slot
            .checked_sub(self.last_update_slot)
            .ok_or(LendingError::MathOverflow)?;
        Ok(slots_elapsed)
    }

    /// Set last update slot
    pub fn update_slot(&mut self, slot: Slot) {
        self.last_update_slot = slot;
    }

    /// Set last update slot to 0
    pub fn mark_stale(&mut self) {
        self.update_slot(0);
    }

    /// Check if last update slot is too long ago
    pub fn is_stale(&self, slot: Slot) -> Result<bool, ProgramError> {
        Ok(self.last_update_slot == 0 || self.slots_elapsed(slot)? > STALE_AFTER_SLOTS)
    }
}

/// Obligation repay result
pub struct RepayResult {
    /// Amount of collateral to withdraw
    pub collateral_withdraw_amount: u64,
    /// Amount of obligation tokens to burn
    pub obligation_token_amount: u64,
    /// Amount that will be repaid as precise decimal
    pub decimal_repay_amount: Decimal,
    /// Amount that will be repaid as u64
    pub integer_repay_amount: u64,
}

impl Sealed for Obligation {}
impl IsInitialized for Obligation {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

const OBLIGATION_LEN: usize = 395; // 1 + 8 + 32 + 16 + 16 + 1 + 1 + (32 * 10)
impl Pack for Obligation {
    const LEN: usize = OBLIGATION_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let output = array_mut_ref![dst, 0, OBLIGATION_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            last_update_slot,
            lending_market,
            collateral_value,
            liquidity_value,
            num_collateral,
            num_liquidity,
            accounts_flat,
        ) = mut_array_refs![
            output,
            1,
            8,
            PUBKEY_LEN,
            16,
            16,
            1,
            1,
            PUBKEY_LEN * MAX_OBLIGATION_ACCOUNTS
        ];

        *version = self.version.to_le_bytes();
        *last_update_slot = self.last_update_slot.to_le_bytes();
        lending_market.copy_from_slice(self.lending_market.as_ref());
        pack_decimal(self.collateral_value, collateral_value);
        pack_decimal(self.liquidity_value, liquidity_value);

        // @TODO: this seems clunky, is this correct?
        *num_collateral = u8::try_from(self.collateral.len())?.to_le_bytes();
        *num_liquidity = u8::try_from(self.liquidity.len())?.to_le_bytes();

        let mut offset = 0;
        for pubkey in self.collateral.iter() {
            let account = array_mut_ref![accounts_flat, offset, PUBKEY_LEN];
            account.copy_from_slice(pubkey.as_ref());
            // @FIXME: unchecked math
            offset += PUBKEY_LEN;
        }
        for pubkey in self.liquidity.iter() {
            let account = array_mut_ref![accounts_flat, offset, PUBKEY_LEN];
            account.copy_from_slice(pubkey.as_ref());
            // @FIXME: unchecked math
            offset += PUBKEY_LEN;
        }
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![src, 0, OBLIGATION_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            last_update_slot,
            lending_market,
            collateral_value,
            liquidity_value,
            num_collateral,
            num_liquidity,
            accounts_flat,
        ) = array_refs![
            input,
            1,
            8,
            PUBKEY_LEN,
            16,
            16,
            1,
            1,
            PUBKEY_LEN * MAX_OBLIGATION_ACCOUNTS
        ];

        let collateral_len = u8::from_le_bytes(*num_collateral);
        let liquidity_len = u8::from_le_bytes(*num_liquidity);
        // @FIXME: unchecked math
        let total_len = collateral_len + liquidity_len;

        // @TODO: this seems clunky, is this correct?
        let mut collateral = Vec::with_capacity(collateral_len.try_into()?);
        let mut liquidity = Vec::with_capacity(liquidity_len.try_into()?);

        let mut offset = 0;
        // @TODO: is there a more idiomatic/performant way to iterate this?
        for account in accounts_flat.chunks(PUBKEY_LEN) {
            if offset < collateral_len {
                collateral.push(Pubkey::new(account));
            } else if offset < total_len {
                liquidity.push(Pubkey::new(account));
            } else {
                break;
            }
            // @FIXME: unchecked math
            offset += 1;
        }

        Ok(Self {
            version: u8::from_le_bytes(*version),
            last_update_slot: u64::from_le_bytes(*last_update_slot),
            lending_market: Pubkey::new_from_array(*lending_market),
            collateral_value: unpack_decimal(collateral_value),
            liquidity_value: unpack_decimal(liquidity_value),
            collateral,
            liquidity,
        })
    }
}

// @FIXME: tests
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
                borrowed_wads: Decimal::from(u64::MAX),
                ..Obligation::default()
            }
            .accrue_interest(Decimal::from(10 * MAX_COMPOUNDED_INTEREST)),
            Err(LendingError::MathOverflow.into())
        );
    }

    // Creates rates (r1, r2) where 0 < r1 <= r2 <= 100*r1
    prop_compose! {
        fn cumulative_rates()(rate in 1..=u128::MAX)(
            current_rate in Just(rate),
            max_new_rate in rate..=rate.saturating_mul(MAX_COMPOUNDED_INTEREST as u128)
        ) -> (u128, u128) {
            (current_rate, max_new_rate)
        }
    }

    const MAX_BORROWED: u128 = u64::MAX as u128 * WAD as u128;

    // Creates liquidity amounts (repay, borrow) where repay < borrow
    prop_compose! {
        fn repay_partial_amounts()(repay in 1..=u64::MAX)(
            liquidity_amount in Just(repay),
            borrowed_liquidity in (WAD as u128 * repay as u128 + 1)..=MAX_BORROWED
        ) -> (u64, u128) {
            (liquidity_amount, borrowed_liquidity)
        }
    }

    // Creates liquidity amounts (repay, borrow) where repay >= borrow
    prop_compose! {
        fn repay_full_amounts()(repay in 1..=u64::MAX)(
            liquidity_amount in Just(repay),
            borrowed_liquidity in 0..=(WAD as u128 * repay as u128)
        ) -> (u64, u128) {
            (liquidity_amount, borrowed_liquidity)
        }
    }

    // Creates collateral amounts (collateral, obligation tokens) where c <= ot
    prop_compose! {
        fn collateral_amounts()(collateral in 1..=u64::MAX)(
            deposited_collateral_tokens in Just(collateral),
            obligation_tokens in collateral..=u64::MAX
        ) -> (u64, u64) {
            (deposited_collateral_tokens, obligation_tokens)
        }
    }

    proptest! {
        #[test]
        fn repay_partial(
            (liquidity_amount, borrowed_liquidity) in repay_partial_amounts(),
            (deposited_collateral_tokens, obligation_tokens) in collateral_amounts(),
        ) {
            let borrowed_wads = Decimal::from_scaled_val(borrowed_liquidity);
            let mut state = Obligation {
                borrowed_wads,
                deposited_collateral_tokens,
                ..Obligation::default()
            };

            let repay_result = state.repay(liquidity_amount, obligation_tokens)?;
            assert!(repay_result.decimal_repay_amount <= Decimal::from(repay_result.integer_repay_amount));
            assert!(repay_result.collateral_withdraw_amount < deposited_collateral_tokens);
            assert!(repay_result.obligation_token_amount < obligation_tokens);
            assert!(state.borrowed_wads < borrowed_wads);
            assert!(state.borrowed_wads > Decimal::zero());
            assert!(state.deposited_collateral_tokens > 0);

            let obligation_token_rate = Decimal::from(repay_result.obligation_token_amount).try_div(Decimal::from(obligation_tokens))?;
            let collateral_withdraw_rate = Decimal::from(repay_result.collateral_withdraw_amount).try_div(Decimal::from(deposited_collateral_tokens))?;
            assert!(obligation_token_rate <= collateral_withdraw_rate);
        }

        #[test]
        fn repay_full(
            (liquidity_amount, borrowed_liquidity) in repay_full_amounts(),
            (deposited_collateral_tokens, obligation_tokens) in collateral_amounts(),
        ) {
            let borrowed_wads = Decimal::from_scaled_val(borrowed_liquidity);
            let mut state = Obligation {
                borrowed_wads,
                deposited_collateral_tokens,
                ..Obligation::default()
            };

            let repay_result = state.repay(liquidity_amount, obligation_tokens)?;
            assert!(repay_result.decimal_repay_amount <= Decimal::from(repay_result.integer_repay_amount));
            assert_eq!(repay_result.collateral_withdraw_amount, deposited_collateral_tokens);
            assert_eq!(repay_result.obligation_token_amount, obligation_tokens);
            assert_eq!(repay_result.decimal_repay_amount, borrowed_wads);
            assert_eq!(state.borrowed_wads, Decimal::zero());
            assert_eq!(state.deposited_collateral_tokens, 0);
        }

        #[test]
        fn accrue_interest(
            borrowed_liquidity in 0..=u64::MAX,
            (current_borrow_rate, new_borrow_rate) in cumulative_rates(),
        ) {
            let borrowed_wads = Decimal::from(borrowed_liquidity);
            let cumulative_borrow_rate_wads = Decimal::one().try_add(Decimal::from_scaled_val(current_borrow_rate))?;
            let mut state = Obligation {
                borrowed_wads,
                cumulative_borrow_rate_wads,
                ..Obligation::default()
            };

            let next_cumulative_borrow_rate_wads = Decimal::one().try_add(Decimal::from_scaled_val(new_borrow_rate))?;
            state.accrue_interest(next_cumulative_borrow_rate_wads)?;

            if next_cumulative_borrow_rate_wads > cumulative_borrow_rate_wads {
                assert!(state.borrowed_wads > borrowed_wads);
            } else {
                assert!(state.borrowed_wads == borrowed_wads);
            }
        }
    }
}
