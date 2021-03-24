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

/// Lending market obligation state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Obligation {
    /// Version of the struct
    pub version: u8,
    /// Last update to collateral, liquidity, or their market values
    pub last_update: LastUpdate,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Deposited collateral for the obligation, unique by deposit reserve address
    pub deposits: Vec<ObligationCollateral>,
    /// Borrowed liquidity for the obligation, unique by borrow reserve address
    pub borrows: Vec<ObligationLiquidity>,
}

impl Obligation {
    // @TODO: this gets called a lot. we could persist the value on obligation refresh instead,
    //        but that seems sloppy.
    /// Calculate the deposited collateral market value
    pub fn deposited_value(&self) -> Result<Decimal, ProgramError> {
        let mut deposited_value = Decimal::zero();
        for collateral in &self.deposits {
            deposited_value = deposited_value.try_add(collateral.market_value)?;
        }
        Ok(deposited_value)
    }

    // @TODO: this gets called a lot. we could persist the value on obligation refresh instead,
    //        but that seems sloppy.
    /// Calculate the borrowed liquidity market value
    pub fn borrowed_value(&self) -> Result<Decimal, ProgramError> {
        let mut borrowed_value = Decimal::zero();
        for liquidity in &self.borrows {
            borrowed_value = borrowed_value.try_add(liquidity.market_value)?;
        }
        Ok(borrowed_value)
    }

    /// Calculate the ratio of liquidity market value to collateral market value
    pub fn loan_to_value(&self) -> Result<Rate, ProgramError> {
        let deposited_value = self.deposited_value()?;
        if deposited_value == Decimal::zero() {
            return Err(LendingError::ObligationCollateralEmpty.into());
        }
        Rate::try_from(self.borrowed_value()?.try_div(deposited_value)?)
    }

    /// Calculate the maximum collateral value that can be withdrawn for a given loan to value ratio
    pub fn max_withdraw_value(&self, loan_to_value_ratio: Rate) -> Result<Decimal, ProgramError> {
        let min_deposited_value = self.borrowed_value()?.try_div(loan_to_value_ratio)?;
        self.deposited_value()?.try_sub(min_deposited_value)
    }

    /// Calculate the maximum liquidity value that can be borrowed for a given loan to value ratio
    pub fn max_borrow_value(&self, loan_to_value_ratio: Rate) -> Result<Decimal, ProgramError> {
        self.deposited_value()?
            .try_mul(loan_to_value_ratio)?
            .try_sub(self.borrowed_value()?)
    }

    /// Calculate the maximum liquidation amount for a given liquidity
    pub fn max_liquidation_amount(
        &self,
        liquidity: &ObligationLiquidity,
    ) -> Result<Decimal, ProgramError> {
        let max_liquidation_value = self
            .borrowed_value()?
            .try_mul(Rate::from_percent(LIQUIDATION_CLOSE_FACTOR))?
            .min(liquidity.market_value);
        let max_liquidation_pct = max_liquidation_value.try_div(liquidity.market_value)?;
        liquidity.borrowed_amount_wads.try_mul(max_liquidation_pct)
    }

    /// Find collateral by deposit reserve
    pub fn find_deposit(
        &self,
        deposit_reserve: Pubkey,
    ) -> Result<(&ObligationCollateral, usize), ProgramError> {
        if self.deposits.is_empty() {
            return Err(LendingError::ObligationCollateralEmpty.into());
        }
        let deposit_index = self
            ._find_deposit_index(deposit_reserve)
            .ok_or(LendingError::InvalidObligationCollateral)?;
        Ok((&self.deposits[deposit_index], deposit_index))
    }

    /// Find or add collateral by deposit reserve
    pub fn find_or_add_deposit(
        &mut self,
        deposit_reserve: Pubkey,
    ) -> Result<&mut ObligationCollateral, ProgramError> {
        if let Some(deposit_index) = self._find_deposit_index(deposit_reserve) {
            Ok(&mut self.deposits[deposit_index])
        } else if self.deposits.len() + self.borrows.len() >= MAX_OBLIGATION_RESERVES {
            Err(LendingError::ObligationReserveLimit.into())
        } else {
            let collateral = ObligationCollateral::new(deposit_reserve);
            self.deposits.push(collateral);
            Ok(self.deposits.last_mut().unwrap())
        }
    }

    fn _find_deposit_index(&self, deposit_reserve: Pubkey) -> Option<usize> {
        self.deposits
            .iter()
            .position(|collateral| collateral.deposit_reserve == deposit_reserve)
    }

    /// Find liquidity by borrow reserve
    pub fn find_borrow(
        &self,
        borrow_reserve: Pubkey,
    ) -> Result<(&ObligationLiquidity, usize), ProgramError> {
        if self.borrows.is_empty() {
            return Err(LendingError::ObligationLiquidityEmpty.into());
        }
        let borrow_index = self
            ._find_borrow_index(borrow_reserve)
            .ok_or(LendingError::InvalidObligationLiquidity)?;
        Ok((&self.borrows[borrow_index], borrow_index))
    }

    /// Find or add liquidity by borrow reserve
    pub fn find_or_add_borrow(
        &mut self,
        borrow_reserve: Pubkey,
    ) -> Result<&mut ObligationLiquidity, ProgramError> {
        if let Some(borrow_index) = self._find_borrow_index(borrow_reserve) {
            Ok(&mut self.borrows[borrow_index])
        } else if self.deposits.len() + self.borrows.len() >= MAX_OBLIGATION_RESERVES {
            Err(LendingError::ObligationReserveLimit.into())
        } else {
            let liquidity = ObligationLiquidity::new(borrow_reserve);
            self.borrows.push(liquidity);
            Ok(self.borrows.last_mut().unwrap())
        }
    }

    fn _find_borrow_index(&self, borrow_reserve: Pubkey) -> Option<usize> {
        self.borrows
            .iter()
            .position(|liquidity| liquidity.borrow_reserve == borrow_reserve)
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
            deposits_len,
            borrows_len,
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
        *deposits_len = u8::try_from(self.deposits.len()).unwrap().to_le_bytes();
        *borrows_len = u8::try_from(self.borrows.len()).unwrap().to_le_bytes();

        let mut offset = 0;
        for collateral in &self.deposits {
            let deposits_flat = array_mut_ref![data_flat, offset, OBLIGATION_COLLATERAL_LEN];
            #[allow(clippy::ptr_offset_with_cast)]
            let (deposit_reserve, deposited_amount, market_value) =
                mut_array_refs![deposits_flat, PUBKEY_BYTES, 8, 16];
            deposit_reserve.copy_from_slice(collateral.deposit_reserve.as_ref());
            *deposited_amount = collateral.deposited_amount.to_le_bytes();
            pack_decimal(collateral.market_value, market_value);
            offset += OBLIGATION_COLLATERAL_LEN;
        }
        for liquidity in &self.borrows {
            let borrows_flat = array_mut_ref![data_flat, offset, OBLIGATION_LIQUIDITY_LEN];
            #[allow(clippy::ptr_offset_with_cast)]
            let (borrow_reserve, cumulative_borrow_rate_wads, borrowed_amount_wads, market_value) =
                mut_array_refs![borrows_flat, PUBKEY_BYTES, 16, 16, 16];
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
            deposits_len,
            borrows_len,
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

        let deposits_len = u8::from_le_bytes(*deposits_len);
        let borrows_len = u8::from_le_bytes(*borrows_len);
        let mut deposits = Vec::with_capacity(deposits_len as usize + 1);
        let mut borrows = Vec::with_capacity(borrows_len as usize + 1);

        let mut offset = 0;
        for _ in 0..deposits_len {
            let deposits_flat = array_ref![data_flat, offset, OBLIGATION_COLLATERAL_LEN];
            #[allow(clippy::ptr_offset_with_cast)]
            let (deposit_reserve, deposited_amount, market_value) =
                array_refs![deposits_flat, PUBKEY_BYTES, 8, 16];
            deposits.push(ObligationCollateral {
                deposit_reserve: Pubkey::new(deposit_reserve),
                deposited_amount: u64::from_le_bytes(*deposited_amount),
                market_value: unpack_decimal(market_value),
            });
            offset += OBLIGATION_COLLATERAL_LEN;
        }
        for _ in 0..borrows_len {
            let borrows_flat = array_ref![data_flat, offset, OBLIGATION_LIQUIDITY_LEN];
            #[allow(clippy::ptr_offset_with_cast)]
            let (borrow_reserve, cumulative_borrow_rate_wads, borrowed_amount_wads, market_value) =
                array_refs![borrows_flat, PUBKEY_BYTES, 16, 16, 16];
            borrows.push(ObligationLiquidity {
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
            deposits,
            borrows,
        })
    }
}
