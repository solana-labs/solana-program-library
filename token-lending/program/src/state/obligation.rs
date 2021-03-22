use super::*;
use crate::{
    error::LendingError,
    instruction::AmountType,
    math::{Decimal, Rate, TryAdd, TryDiv, TryMul, TrySub},
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    clock::Slot,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::{Pubkey, PUBKEY_BYTES},
};
use std::collections::HashMap;
use std::convert::{TryFrom, TryInto};

// @TODO: rename / relocate; true max is potentially 28
/// Max number of collateral and liquidity accounts combined for an obligation
pub const MAX_OBLIGATION_DATA: usize = 10;

/// Borrow obligation state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Obligation<'a> {
    /// Version of the struct
    pub version: u8,
    /// Last update to collateral, liquidity, or their market values
    pub last_update: LastUpdate,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Collateral state for the obligation, keyed by deposit reserve address
    pub collateral: HashMap<Pubkey, ObligationCollateral>,
    /// Liquidity state for the obligation, keyed by borrow reserve address
    pub liquidity: HashMap<Pubkey, ObligationLiquidity>,
}

/// Create new obligation
pub struct NewObligationParams {
    /// Current slot
    pub current_slot: Slot,
    /// Lending market address
    pub lending_market: Pubkey,
}

impl Obligation {
    /// Create new obligation
    pub fn new(params: NewObligationParams) -> Self {
        let NewObligationParams {
            current_slot,
            lending_market,
        } = params;

        Self {
            version: PROGRAM_VERSION,
            last_update: LastUpdate::new(current_slot),
            lending_market,
            collateral: HashMap::new(),
            liquidity: HashMap::new(),
        }
    }

    /// Calculate the ratio of liquidity market value to collateral market value
    pub fn loan_to_value(&self) -> Result<Decimal, ProgramError> {
        let mut liquidity_value = Decimal::zero();
        for (_, liquidity) in self.liquidity {
            liquidity_value = liquidity_value.try_add(liquidity.market_value)?;
        }

        let mut collateral_value = Decimal::zero();
        for (_, collateral) in self.collateral {
            collateral_value = collateral_value.try_add(collateral.market_value)?;
        }

        // @TODO: error if collateral value is zero?
        liquidity_value.try_div(collateral_value)
    }

    pub fn withdraw_collateral(
        &self,
        collateral_amount: u64,
        collateral_amount_type: AmountType,
        obligation_collateral: &ObligationCollateral,
        loan_to_value_ratio: Rate,
        obligation_token_supply: u64,
    ) -> Result<WithdrawCollateralResult, ProgramError> {
        let min_collateral_value = self.liquidity_value.try_div(loan_to_value_ratio)?;
        let max_withdraw_value = self.collateral_value.try_sub(min_collateral_value)?;

        let withdraw_amount = match collateral_amount_type {
            AmountType::ExactAmount => {
                let withdraw_amount = collateral_amount.min(obligation_collateral.deposited_amount);
                let withdraw_pct = Decimal::from(withdraw_amount)
                    .try_div(obligation_collateral.deposited_amount)?;
                let withdraw_value = self.collateral_value.try_mul(withdraw_pct)?;
                if withdraw_value > max_withdraw_value {
                    return Err(LendingError::ObligationCollateralWithdrawTooLarge.into());
                }

                withdraw_amount
            }
            AmountType::PercentAmount => {
                let withdraw_pct = Decimal::from_percent(u8::try_from(collateral_amount)?);
                let withdraw_value = max_withdraw_value
                    .try_mul(withdraw_pct)?
                    .min(obligation_collateral.value);
                let withdraw_amount = withdraw_value
                    .try_div(obligation_collateral.value)?
                    .try_mul(obligation_collateral.deposited_amount)?
                    .try_floor_u64()?;

                withdraw_amount
            }
        };

        let obligation_token_amount = obligation_collateral
            .collateral_to_obligation_token_amount(withdraw_amount, obligation_token_supply)?;

        Ok(WithdrawCollateralResult {
            withdraw_amount,
            obligation_token_amount,
        })
    }
}

/// Withdraw collateral result
#[derive(Debug)]
pub struct WithdrawCollateralResult {
    /// Collateral tokens to withdraw
    withdraw_amount: u64,
    /// Obligation tokens to burn
    obligation_token_amount: u64,
}

impl Sealed for Obligation {}
impl IsInitialized for Obligation {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

/// Obligation collateral state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObligationCollateral {
    /// Amount of collateral deposited
    pub deposited_amount: u64,
    /// Collateral market value in quote currency
    pub market_value: Decimal,
}

impl ObligationCollateral {
    /// Create new obligation collateral
    pub fn new() -> Self {
        Self {
            deposited_amount: 0,
            // @TODO: should this be initialized with a real value on deposit?
            market_value: Decimal::zero(),
        }
    }

    /// Increase deposited collateral
    pub fn deposit(&mut self, collateral_amount: u64) -> ProgramResult {
        self.deposited_amount = self
            .deposited_amount
            .checked_add(collateral_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    /// Decrease deposited collateral
    pub fn withdraw(&mut self, collateral_amount: u64) -> ProgramResult {
        self.deposited_amount = self
            .deposited_amount
            .checked_sub(collateral_amount)
            .ok_or(LendingError::MathOverflow)?;
        Ok(())
    }

    /// Amount of obligation tokens for given collateral
    pub fn collateral_to_obligation_token_amount(
        &self,
        collateral_amount: u64,
        obligation_token_supply: u64,
    ) -> Result<u64, ProgramError> {
        let withdraw_pct = Decimal::from(collateral_amount).try_div(self.deposited_amount)?;
        withdraw_pct
            .try_mul(obligation_token_supply)?
            .try_floor_u64()
    }
}

/// Obligation liquidity state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ObligationLiquidity {
    /// Borrow rate used for calculating interest
    pub cumulative_borrow_rate_wads: Decimal,
    /// Amount of liquidity borrowed plus interest
    pub borrowed_amount_wads: Decimal,
    /// Liquidity market value in quote currency
    pub market_value: Decimal,
}

impl ObligationLiquidity {
    /// Create new obligation liquidity
    pub fn new() -> Self {
        Self {
            cumulative_borrow_rate_wads: Decimal::one(),
            borrowed_amount_wads: Decimal::zero(),
            // @TODO: should this be initialized with a real value on borrow?
            market_value: Decimal::zero(),
        }
    }

    /// Decrease borrowed liquidity
    pub fn repay(&mut self, settle_amount: Decimal) -> ProgramResult {
        self.borrowed_amount_wads = self.borrowed_amount_wads.try_sub(settle_amount)?;
        Ok(())
    }

    /// Increase borrowed liquidity
    pub fn borrow(&mut self, borrow_amount: u64) -> ProgramResult {
        self.borrowed_amount_wads = self.borrowed_amount_wads.try_add(borrow_amount.into())?;
        Ok(())
    }

    /// Accrue interest
    pub fn accrue_interest(&mut self, cumulative_borrow_rate_wads: Decimal) -> ProgramResult {
        if cumulative_borrow_rate_wads < self.cumulative_borrow_rate_wads {
            return Err(LendingError::NegativeInterestRate.into());
        }

        let compounded_interest_rate: Rate = cumulative_borrow_rate_wads
            .try_div(self.cumulative_borrow_rate_wads)?
            .try_into()?;

        self.borrowed_amount_wads = self
            .borrowed_amount_wads
            .try_mul(compounded_interest_rate)?;
        self.cumulative_borrow_rate_wads = cumulative_borrow_rate_wads;

        Ok(())
    }
}

const OBLIGATION_COLLATERAL_LEN: usize = 40; // 32 + 8
const OBLIGATION_LIQUIDITY_LEN: usize = 64; // 32 + 16 + 16
const OBLIGATION_LEN: usize = 716; // 1 + 8 + 1 + 32 + 16 + 16 + 1 + 1 + (64 * 10)
impl Pack for Obligation {
    const LEN: usize = OBLIGATION_LEN;

    fn pack_into_slice(&self, dst: &mut [u8]) {
        let output = array_mut_ref![dst, 0, OBLIGATION_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            last_update_slot,
            last_update_stale,
            lending_market,
            collateral_len,
            liquidity_len,
            data_flat,
        ) = mut_array_refs![
            output,
            1,
            8,
            1,
            PUBKEY_BYTES,
            1,
            1,
            OBLIGATION_LIQUIDITY_LEN * MAX_OBLIGATION_DATA
        ];

        *version = self.version.to_le_bytes();
        *last_update_slot = self.last_update.slot.to_le_bytes();
        *last_update_stale = u8::from(self.last_update.stale).to_le_bytes();
        lending_market.copy_from_slice(self.lending_market.as_ref());
        *collateral_len = u8::try_from(self.collateral.len())?.to_le_bytes();
        *liquidity_len = u8::try_from(self.liquidity.len())?.to_le_bytes();

        let mut offset = 0;
        for (deposit_reserve, collateral) in self.collateral.iter() {
            let collateral_flat = array_mut_ref![data_flat, offset, OBLIGATION_COLLATERAL_LEN];
            #[allow(clippy::ptr_offset_with_cast)]
            let (reserve, deposited_amount) = mut_array_refs![collateral_flat, PUBKEY_BYTES, 8];
            reserve.copy_from_slice(deposit_reserve.as_ref());
            *deposited_amount = collateral.deposited_amount.to_le_bytes();
            offset += OBLIGATION_COLLATERAL_LEN;
        }
        for (borrow_reserve, liquidity) in self.liquidity.iter() {
            let liquidity_flat = array_mut_ref![data_flat, offset, OBLIGATION_LIQUIDITY_LEN];
            #[allow(clippy::ptr_offset_with_cast)]
            let (reserve, cumulative_borrow_rate_wads, borrowed_amount_wads) =
                mut_array_refs![liquidity_flat, PUBKEY_BYTES, 16, 16];
            reserve.copy_from_slice(borrow_reserve.as_ref());
            *cumulative_borrow_rate_wads = liquidity.cumulative_borrow_rate_wads.to_le_bytes();
            *borrowed_amount_wads = liquidity.borrowed_amount_wads.to_le_bytes();
            offset += OBLIGATION_LIQUIDITY_LEN;
        }
    }

    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![src, 0, OBLIGATION_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            last_update_slot,
            last_update_stale,
            lending_market,
            collateral_len,
            liquidity_len,
            data_flat,
        ) = array_refs![
            input,
            1,
            8,
            1,
            PUBKEY_BYTES,
            1,
            1,
            OBLIGATION_LIQUIDITY_LEN * MAX_OBLIGATION_DATA
        ];

        let collateral_len = u8::from_le_bytes(*collateral_len);
        let liquidity_len = u8::from_le_bytes(*liquidity_len);
        let mut collateral = HashMap::with_capacity(usize::from(collateral_len));
        let mut liquidity = HashMap::with_capacity(usize::from(liquidity_len));

        let mut offset = 0;
        for _ in collateral_len {
            let collateral_flat = array_ref![data_flat, offset, OBLIGATION_COLLATERAL_LEN];
            let (deposit_reserve, deposited_amount) = array_refs![collateral_flat, PUBKEY_BYTES, 8];
            collateral.insert(
                Pubkey::new(deposit_reserve),
                ObligationCollateral {
                    deposited_amount: u64::from_le_bytes(*deposited_amount),
                },
            );
            offset += OBLIGATION_COLLATERAL_LEN;
        }
        for _ in liquidity_len {
            let liquidity_flat = array_ref![data_flat, offset, OBLIGATION_LIQUIDITY_LEN];
            let (borrow_reserve, cumulative_borrow_rate_wads, borrowed_amount_wads) =
                array_refs![liquidity_flat, PUBKEY_BYTES, 16, 16];
            liquidity.insert(
                Pubkey::new(borrow_reserve),
                ObligationLiquidity {
                    cumulative_borrow_rate_wads: unpack_decimal(cumulative_borrow_rate_wads),
                    borrowed_amount_wads: unpack_decimal(borrowed_amount_wads),
                },
            );
            offset += OBLIGATION_LIQUIDITY_LEN;
        }

        Ok(Self {
            version: u8::from_le_bytes(*version),
            last_update: LastUpdate {
                slot: u64::from_le_bytes(*last_update_slot),
                stale: bool::from(u8::from_le_bytes(*last_update_stale)),
            },
            lending_market: Pubkey::new_from_array(*lending_market),
            collateral,
            liquidity,
        })
    }
}
