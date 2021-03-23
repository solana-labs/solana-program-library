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
    pubkey::{Pubkey, PUBKEY_BYTES},
};
use std::convert::{TryFrom, TryInto};

/// Max number of collateral and liquidity reserve accounts combined for an obligation
pub const MAX_OBLIGATION_RESERVES: usize = 10;

/// Borrow obligation state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Obligation {
    /// Version of the struct
    pub version: u8,
    /// Last update to collateral, liquidity, or their market values
    pub last_update: LastUpdate,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Collateral state for the obligation, unique by deposit reserve address
    pub collateral: Vec<ObligationCollateral>,
    /// Liquidity state for the obligation, unique by borrow reserve address
    pub liquidity: Vec<ObligationLiquidity>,
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
            collateral: vec![],
            liquidity: vec![],
        }
    }

    // @TODO: this gets called a lot. we could persist the value on obligation refresh instead,
    //        but that seems sloppy.
    /// Calculate the collateral market value
    pub fn collateral_value(&self) -> Result<Decimal, ProgramError> {
        let mut collateral_value = Decimal::zero();
        for collateral in &self.collateral {
            collateral_value = collateral_value.try_add(collateral.market_value)?;
        }
        Ok(collateral_value)
    }

    // @TODO: this gets called a lot. we could persist the value on obligation refresh instead,
    //        but that seems sloppy.
    /// Calculate the liquidity market value
    pub fn liquidity_value(&self) -> Result<Decimal, ProgramError> {
        let mut liquidity_value = Decimal::zero();
        for liquidity in &self.liquidity {
            liquidity_value = liquidity_value.try_add(liquidity.market_value)?;
        }
        Ok(liquidity_value)
    }

    /// Calculate the ratio of liquidity market value to collateral market value
    pub fn loan_to_value(&self) -> Result<Rate, ProgramError> {
        let collateral_value = self.collateral_value()?;
        if collateral_value == Decimal::zero() {
            return Err(LendingError::ObligationCollateralEmpty.into());
        }
        Rate::try_from(self.liquidity_value()?.try_div(collateral_value)?)
    }

    /// Calculate the maximum collateral value that can be withdrawn for a given loan to value ratio
    pub fn max_withdraw_value(&self, loan_to_value_ratio: Rate) -> Result<Decimal, ProgramError> {
        let min_collateral_value = self.liquidity_value()?.try_div(loan_to_value_ratio)?;
        self.collateral_value()?.try_sub(min_collateral_value)
    }

    /// Calculate the maximum liquidity value that can be borrowed for a given loan to value ratio
    pub fn max_borrow_value(&self, loan_to_value_ratio: Rate) -> Result<Decimal, ProgramError> {
        self.collateral_value()?
            .try_mul(loan_to_value_ratio)?
            .try_sub(self.liquidity_value()?)
    }

    /// Calculate the maximum liquidation amount for a given liquidity
    pub fn max_liquidation_amount(
        &self,
        liquidity: &ObligationLiquidity,
    ) -> Result<Decimal, ProgramError> {
        let max_liquidation_value = self
            .liquidity_value()?
            .try_mul(Rate::from_percent(LIQUIDATION_CLOSE_FACTOR))?
            .min(liquidity.market_value);
        let max_liquidation_pct = max_liquidation_value.try_div(liquidity.market_value)?;
        liquidity.borrowed_amount_wads.try_mul(max_liquidation_pct)
    }

    /// Find collateral by deposit reserve
    pub fn find_collateral(
        &self,
        deposit_reserve: Pubkey,
    ) -> Result<&ObligationCollateral, ProgramError> {
        if self.collateral.len() == 0 {
            return Err(LendingError::ObligationCollateralEmpty.into());
        }
        if let Some(collateral) = self
            .collateral
            .iter()
            .find(|collateral| collateral.deposit_reserve == deposit_reserve)
        {
            Ok(collateral)
        } else {
            Err(LendingError::InvalidObligationCollateral.into())
        }
    }

    /// Find collateral by deposit reserve and borrow a mutable reference to it
    pub fn find_collateral_mut(
        &mut self,
        deposit_reserve: Pubkey,
    ) -> Result<&mut ObligationCollateral, ProgramError> {
        if self.collateral.len() == 0 {
            return Err(LendingError::ObligationCollateralEmpty.into());
        }
        if let Some(collateral) = self
            .collateral
            .iter_mut()
            .find(|collateral| collateral.deposit_reserve == deposit_reserve)
        {
            Ok(collateral)
        } else {
            Err(LendingError::InvalidObligationCollateral.into())
        }
    }

    /// Find or add collateral by deposit reserve
    pub fn find_or_add_collateral(
        &mut self,
        deposit_reserve: Pubkey,
    ) -> Result<&mut ObligationCollateral, ProgramError> {
        if let Some(collateral) = self
            .collateral
            .iter_mut()
            .find(|collateral| collateral.deposit_reserve == deposit_reserve)
        {
            Ok(collateral)
        } else if self.collateral.len() + self.liquidity.len() >= MAX_OBLIGATION_RESERVES {
            Err(LendingError::ObligationReserveLimit.into())
        } else {
            self.collateral
                .push(ObligationCollateral::new(deposit_reserve));
            Ok(self.collateral.last_mut().unwrap())
        }
    }

    /// Find liquidity by borrow reserve
    pub fn find_liquidity(
        &self,
        borrow_reserve: Pubkey,
    ) -> Result<&ObligationLiquidity, ProgramError> {
        if self.liquidity.len() == 0 {
            return Err(LendingError::ObligationLiquidityEmpty.into());
        }
        if let Some(liquidity) = self
            .liquidity
            .iter()
            .find(|liquidity| liquidity.borrow_reserve == borrow_reserve)
        {
            Ok(liquidity)
        } else {
            Err(LendingError::InvalidObligationLiquidity.into())
        }
    }

    /// Find liquidity by borrow reserve and borrow a mutable reference to it
    pub fn find_liquidity_mut(
        &mut self,
        borrow_reserve: Pubkey,
    ) -> Result<&mut ObligationLiquidity, ProgramError> {
        if self.liquidity.len() == 0 {
            return Err(LendingError::ObligationLiquidityEmpty.into());
        }
        if let Some(liquidity) = self
            .liquidity
            .iter_mut()
            .find(|liquidity| liquidity.borrow_reserve == borrow_reserve)
        {
            Ok(liquidity)
        } else {
            Err(LendingError::InvalidObligationLiquidity.into())
        }
    }

    /// Find or add liquidity by borrow reserve
    pub fn find_or_add_liquidity(
        &mut self,
        borrow_reserve: Pubkey,
    ) -> Result<&mut ObligationLiquidity, ProgramError> {
        if let Some(liquidity) = self
            .liquidity
            .iter_mut()
            .find(|liquidity| liquidity.borrow_reserve == borrow_reserve)
        {
            Ok(liquidity)
        } else if self.collateral.len() + self.liquidity.len() >= MAX_OBLIGATION_RESERVES {
            Err(LendingError::ObligationReserveLimit.into())
        } else {
            self.liquidity
                .push(ObligationLiquidity::new(borrow_reserve));
            Ok(self.liquidity.last_mut().unwrap())
        }
    }
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
    /// Reserve collateral is deposited to
    pub deposit_reserve: Pubkey,
    /// Amount of collateral deposited
    pub deposited_amount: u64,
    /// Collateral market value in quote currency
    pub market_value: Decimal,
}

impl ObligationCollateral {
    /// Create new obligation collateral
    pub fn new(deposit_reserve: Pubkey) -> Self {
        Self {
            deposit_reserve,
            deposited_amount: 0,
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
    /// Reserve liquidity is borrowed from
    pub borrow_reserve: Pubkey,
    /// Borrow rate used for calculating interest
    pub cumulative_borrow_rate_wads: Decimal,
    /// Amount of liquidity borrowed plus interest
    pub borrowed_amount_wads: Decimal,
    /// Liquidity market value in quote currency
    pub market_value: Decimal,
}

impl ObligationLiquidity {
    /// Create new obligation liquidity
    pub fn new(borrow_reserve: Pubkey) -> Self {
        Self {
            borrow_reserve,
            cumulative_borrow_rate_wads: Decimal::one(),
            borrowed_amount_wads: Decimal::zero(),
            market_value: Decimal::zero(),
        }
    }

    /// Decrease borrowed liquidity
    pub fn repay(&mut self, settle_amount: Decimal) -> ProgramResult {
        self.borrowed_amount_wads = self.borrowed_amount_wads.try_sub(settle_amount)?;
        Ok(())
    }

    /// Increase borrowed liquidity
    pub fn borrow(&mut self, borrow_amount: Decimal) -> ProgramResult {
        self.borrowed_amount_wads = self.borrowed_amount_wads.try_add(borrow_amount)?;
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

// @TODO: adjust padding. what's a reasonable number?
const OBLIGATION_COLLATERAL_LEN: usize = 56; // 32 + 8 + 16
const OBLIGATION_LIQUIDITY_LEN: usize = 80; // 32 + 16 + 16 + 16
const OBLIGATION_LEN: usize = 820; // 1 + 8 + 1 + 32 + 1 + 1 + (56 * 1) + (80 * 9)
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
            OBLIGATION_COLLATERAL_LEN + (OBLIGATION_LIQUIDITY_LEN * (MAX_OBLIGATION_RESERVES - 1))
        ];

        *version = self.version.to_le_bytes();
        *last_update_slot = self.last_update.slot.to_le_bytes();
        pack_bool(self.last_update.stale, last_update_stale);
        lending_market.copy_from_slice(self.lending_market.as_ref());
        *collateral_len = u8::try_from(self.collateral.len()).unwrap().to_le_bytes();
        *liquidity_len = u8::try_from(self.liquidity.len()).unwrap().to_le_bytes();

        let mut offset = 0;
        for collateral in &self.collateral {
            let collateral_flat = array_mut_ref![data_flat, offset, OBLIGATION_COLLATERAL_LEN];
            #[allow(clippy::ptr_offset_with_cast)]
            let (deposit_reserve, deposited_amount, market_value) =
                mut_array_refs![collateral_flat, PUBKEY_BYTES, 8, 16];
            deposit_reserve.copy_from_slice(collateral.deposit_reserve.as_ref());
            *deposited_amount = collateral.deposited_amount.to_le_bytes();
            pack_decimal(collateral.market_value, market_value);
            offset += OBLIGATION_COLLATERAL_LEN;
        }
        for liquidity in &self.liquidity {
            let liquidity_flat = array_mut_ref![data_flat, offset, OBLIGATION_LIQUIDITY_LEN];
            #[allow(clippy::ptr_offset_with_cast)]
            let (borrow_reserve, cumulative_borrow_rate_wads, borrowed_amount_wads, market_value) =
                mut_array_refs![liquidity_flat, PUBKEY_BYTES, 16, 16, 16];
            borrow_reserve.copy_from_slice(liquidity.borrow_reserve.as_ref());
            pack_decimal(
                liquidity.cumulative_borrow_rate_wads,
                cumulative_borrow_rate_wads,
            );
            pack_decimal(liquidity.borrowed_amount_wads, borrowed_amount_wads);
            pack_decimal(liquidity.market_value, market_value);
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
            OBLIGATION_COLLATERAL_LEN + (OBLIGATION_LIQUIDITY_LEN * (MAX_OBLIGATION_RESERVES - 1))
        ];

        let collateral_len = u8::from_le_bytes(*collateral_len);
        let liquidity_len = u8::from_le_bytes(*liquidity_len);
        let mut collateral = Vec::with_capacity(collateral_len as usize);
        let mut liquidity = Vec::with_capacity(liquidity_len as usize);

        let mut offset = 0;
        for _ in 0..collateral_len {
            let collateral_flat = array_ref![data_flat, offset, OBLIGATION_COLLATERAL_LEN];
            let (deposit_reserve, deposited_amount, market_value) =
                array_refs![collateral_flat, PUBKEY_BYTES, 8, 16];
            collateral.push(ObligationCollateral {
                deposit_reserve: Pubkey::new(deposit_reserve),
                deposited_amount: u64::from_le_bytes(*deposited_amount),
                market_value: unpack_decimal(market_value),
            });
            offset += OBLIGATION_COLLATERAL_LEN;
        }
        for _ in 0..liquidity_len {
            let liquidity_flat = array_ref![data_flat, offset, OBLIGATION_LIQUIDITY_LEN];
            let (borrow_reserve, cumulative_borrow_rate_wads, borrowed_amount_wads, market_value) =
                array_refs![liquidity_flat, PUBKEY_BYTES, 16, 16, 16];
            liquidity.push(ObligationLiquidity {
                borrow_reserve: Pubkey::new(borrow_reserve),
                cumulative_borrow_rate_wads: unpack_decimal(cumulative_borrow_rate_wads),
                borrowed_amount_wads: unpack_decimal(borrowed_amount_wads),
                market_value: unpack_decimal(market_value),
            });
            offset += OBLIGATION_LIQUIDITY_LEN;
        }

        Ok(Self {
            version: u8::from_le_bytes(*version),
            last_update: LastUpdate {
                slot: u64::from_le_bytes(*last_update_slot),
                stale: unpack_bool(last_update_stale)?,
            },
            lending_market: Pubkey::new_from_array(*lending_market),
            collateral,
            liquidity,
        })
    }
}
