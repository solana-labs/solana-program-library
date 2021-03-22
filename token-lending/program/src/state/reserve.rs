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
    program_option::COption,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::{Pubkey, PUBKEY_BYTES},
};
use std::convert::{TryFrom, TryInto};

/// Percentage of an obligation that can be repaid during each liquidation call
pub const LIQUIDATION_CLOSE_FACTOR: u8 = 50;

/// Obligation borrow amount that is small enough to close out
pub const LIQUIDATION_CLOSE_AMOUNT: u64 = 2;

/// Lending market reserve state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reserve {
    /// Version of the struct
    pub version: u8,
    /// Last slot when supply and rates updated
    pub last_update: LastUpdate,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Reserve liquidity
    pub liquidity: ReserveLiquidity,
    /// Reserve collateral
    pub collateral: ReserveCollateral,
    /// Reserve configuration values
    pub config: ReserveConfig,
}

impl Reserve {
    /// Initialize new reserve state
    pub fn new(params: NewReserveParams) -> Self {
        let NewReserveParams {
            current_slot,
            lending_market,
            liquidity,
            collateral,
            config,
        } = params;

        Self {
            version: PROGRAM_VERSION,
            last_update: LastUpdate::new(current_slot),
            lending_market,
            liquidity,
            collateral,
            config,
        }
    }

    /// Record deposited liquidity and return amount of collateral tokens to mint
    pub fn deposit_liquidity(&mut self, liquidity_amount: u64) -> Result<u64, ProgramError> {
        let collateral_amount = self
            .collateral_exchange_rate()?
            .liquidity_to_collateral(liquidity_amount)?;

        self.liquidity
            .available_amount
            .checked_add(liquidity_amount)
            .ok_or(LendingError::MathOverflow)?;
        self.collateral
            .mint_total_supply
            .checked_add(collateral_amount)
            .ok_or(LendingError::MathOverflow)?;

        Ok(collateral_amount)
    }

    /// Record redeemed collateral and return amount of liquidity to withdraw
    pub fn redeem_collateral(&mut self, collateral_amount: u64) -> Result<u64, ProgramError> {
        let collateral_exchange_rate = self.collateral_exchange_rate()?;
        let liquidity_amount =
            collateral_exchange_rate.collateral_to_liquidity(collateral_amount)?;
        if liquidity_amount > self.liquidity.available_amount {
            return Err(LendingError::InsufficientLiquidity.into());
        }

        self.liquidity.available_amount = self
            .liquidity
            .available_amount
            .checked_sub(liquidity_amount)
            .ok_or(LendingError::MathOverflow)?;
        self.collateral.mint_total_supply = self
            .collateral
            .mint_total_supply
            .checked_sub(collateral_amount)
            .ok_or(LendingError::MathOverflow)?;

        Ok(liquidity_amount)
    }

    /// Calculate the current borrow rate
    pub fn current_borrow_rate(&self) -> Result<Rate, ProgramError> {
        let utilization_rate = self.liquidity.utilization_rate()?;
        let optimal_utilization_rate = Rate::from_percent(self.config.optimal_utilization_rate);
        let low_utilization = utilization_rate < optimal_utilization_rate;
        if low_utilization || self.config.optimal_utilization_rate == 100 {
            let normalized_rate = utilization_rate.try_div(optimal_utilization_rate)?;
            let min_rate = Rate::from_percent(self.config.min_borrow_rate);
            let rate_range = Rate::from_percent(
                self.config
                    .optimal_borrow_rate
                    .checked_sub(self.config.min_borrow_rate)
                    .ok_or(LendingError::MathOverflow)?,
            );

            Ok(normalized_rate.try_mul(rate_range)?.try_add(min_rate)?)
        } else {
            let normalized_rate = utilization_rate
                .try_sub(optimal_utilization_rate)?
                .try_div(Rate::from_percent(
                    100u8
                        .checked_sub(self.config.optimal_utilization_rate)
                        .ok_or(LendingError::MathOverflow)?,
                ))?;
            let min_rate = Rate::from_percent(self.config.optimal_borrow_rate);
            let rate_range = Rate::from_percent(
                self.config
                    .max_borrow_rate
                    .checked_sub(self.config.optimal_borrow_rate)
                    .ok_or(LendingError::MathOverflow)?,
            );

            Ok(normalized_rate.try_mul(rate_range)?.try_add(min_rate)?)
        }
    }

    /// Collateral exchange rate
    pub fn collateral_exchange_rate(&self) -> Result<CollateralExchangeRate, ProgramError> {
        let total_liquidity = self.liquidity.total_supply()?;
        self.collateral.exchange_rate(total_liquidity)
    }

    /// Update borrow rate and accrue interest
    pub fn accrue_interest(&mut self, current_slot: Slot) -> ProgramResult {
        let slots_elapsed = self.last_update.slots_elapsed(current_slot)?;
        if slots_elapsed > 0 {
            let current_borrow_rate = self.current_borrow_rate()?;
            self.liquidity
                .compound_interest(current_borrow_rate, slots_elapsed)?;
            self.last_update.update_slot(current_slot);
        }
        Ok(())
    }

    /// Borrow liquidity up to a maximum market value
    pub fn borrow_liquidity(
        &self,
        liquidity_amount: u64,
        liquidity_amount_type: AmountType,
        obligation: &Obligation,
        loan_to_value_ratio: Rate,
        token_converter: impl TokenConverter,
        quote_token_mint: &Pubkey,
    ) -> Result<BorrowLiquidityResult, ProgramError> {
        let max_borrow_value = obligation
            .collateral_value
            .try_mul(loan_to_value_ratio)?
            .try_sub(obligation.liquidity_value)?;

        match liquidity_amount_type {
            AmountType::ExactAmount => {
                let receive_amount = liquidity_amount;
                let (borrow_fee, host_fee) = self
                    .config
                    .fees
                    .calculate_borrow_fees(receive_amount, FeeCalculation::Exclusive)?;
                let borrow_amount = receive_amount
                    .checked_add(borrow_fee)
                    .ok_or(LendingError::MathOverflow)?;
                let borrow_value =
                    token_converter.convert(borrow_amount.into(), &self.liquidity.mint_pubkey)?;
                if borrow_value > max_borrow_value {
                    return Err(LendingError::BorrowTooLarge.into());
                }

                Ok(BorrowLiquidityResult {
                    borrow_amount,
                    receive_amount,
                    borrow_fee,
                    host_fee,
                })
            }
            AmountType::PercentAmount => {
                let borrow_pct = Decimal::from_percent(u8::try_from(liquidity_amount)?);
                let borrow_value = max_borrow_value.try_mul(borrow_pct)?;
                let borrow_amount = token_converter
                    .convert(borrow_value, &quote_token_mint)?
                    .try_floor_u64()?
                    .min(self.liquidity.available_amount);
                let (origination_fee, host_fee) = self
                    .config
                    .fees
                    .calculate_borrow_fees(borrow_amount, FeeCalculation::Inclusive)?;
                let receive_amount = borrow_amount
                    .checked_sub(origination_fee)
                    .ok_or(LendingError::MathOverflow)?;

                Ok(BorrowLiquidityResult {
                    borrow_amount,
                    receive_amount,
                    borrow_fee: origination_fee,
                    host_fee,
                })
            }
        }
    }

    pub fn repay_liquidity(
        &self,
        liquidity_amount: u64,
        liquidity_amount_type: AmountType,
        obligation_liquidity: &ObligationLiquidity,
    ) -> Result<RepayLiquidityResult, ProgramError> {
        let settle_amount = match liquidity_amount_type {
            AmountType::ExactAmount => {
                Decimal::from(liquidity_amount).min(obligation_liquidity.borrowed_amount_wads)
            }
            AmountType::PercentAmount => {
                let settle_pct = Rate::from_percent(u8::try_from(liquidity_amount)?);
                obligation_liquidity
                    .borrowed_amount_wads
                    .try_mul(settle_pct)?
            }
        };
        let repay_amount = if settle_amount == obligation_liquidity.borrowed_amount_wads {
            settle_amount.try_ceil_u64()?
        } else {
            settle_amount.try_floor_u64()?
        };

        Ok(RepayLiquidityResult {
            settle_amount,
            repay_amount,
        })
    }

    /// Liquidate some or all of an unhealthy obligation
    pub fn liquidate_obligation(
        &self,
        liquidity_amount: u64,
        liquidity_amount_type: AmountType,
        obligation: &Obligation,
        obligation_liquidity: &ObligationLiquidity,
        obligation_collateral: &ObligationCollateral,
    ) -> Result<LiquidateObligationResult, ProgramError> {
        let max_withdraw_amount = Decimal::from(obligation_collateral.deposited_amount);
        let bonus_rate = Rate::from_percent(self.config.liquidation_bonus).try_add(Rate::one())?;

        let liquidate_amount = match liquidity_amount_type {
            AmountType::ExactAmount => {
                Decimal::from(liquidity_amount).min(obligation_liquidity.borrowed_amount_wads)
            }
            AmountType::PercentAmount => {
                let liquidate_pct = Rate::from_percent(u8::try_from(liquidity_amount)?);
                obligation_liquidity
                    .borrowed_amount_wads
                    .try_mul(liquidate_pct)?
            }
        };

        // Close out obligations that are too small to liquidate normally
        if obligation_liquidity.borrowed_amount_wads < LIQUIDATION_CLOSE_AMOUNT.into() {
            let settle_amount = obligation_liquidity.borrowed_amount_wads;
            let settle_value = obligation_liquidity
                .value
                .try_mul(bonus_rate)?
                .min(obligation_collateral.value);
            let settle_pct = settle_value.try_div(obligation_collateral.value)?;
            let repay_amount = liquidate_amount.min(settle_amount).try_ceil_u64()?;

            let collateral_amount = max_withdraw_amount.try_mul(settle_pct)?;
            let withdraw_amount = if collateral_amount == max_withdraw_amount {
                collateral_amount.try_ceil_u64()?
            } else {
                collateral_amount.try_floor_u64()?
            };

            return Ok(LiquidateObligationResult {
                settle_amount,
                repay_amount,
                withdraw_amount,
            });
        }

        let max_liquidation_value = obligation
            .liquidity_value
            .try_mul(Rate::from_percent(LIQUIDATION_CLOSE_FACTOR))?
            .min(obligation_liquidity.value);
        let max_liquidation_pct = max_liquidation_value.try_div(obligation_liquidity.value)?;
        let max_liquidation_amount = obligation_liquidity
            .borrowed_amount_wads
            .try_mul(max_liquidation_pct)?;

        let liquidation_amount = liquidate_amount.min(max_liquidation_amount);
        let liquidation_pct =
            liquidation_amount.try_div(obligation_liquidity.borrowed_amount_wads)?;

        let settle_value = obligation_liquidity
            .value
            .try_mul(liquidation_pct)?
            .try_mul(bonus_rate)?
            .min(obligation_collateral.value);
        let settle_pct = settle_value.try_div(obligation_collateral.value)?;
        let settle_amount = liquidation_amount.try_mul(settle_pct)?;

        let repay_amount = if settle_amount == max_liquidation_amount {
            settle_amount.try_ceil_u64()?
        } else {
            settle_amount.try_floor_u64()?
        };

        let collateral_amount = max_withdraw_amount.try_mul(settle_pct)?;
        let withdraw_amount = if collateral_amount == max_withdraw_amount {
            collateral_amount.try_ceil_u64()?
        } else {
            collateral_amount.try_floor_u64()?
        };

        Ok(LiquidateObligationResult {
            settle_amount,
            repay_amount,
            withdraw_amount,
        })
    }
}

/// Create new reserve
pub struct NewReserveParams {
    /// Current slot
    pub current_slot: Slot,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Reserve collateral
    pub collateral: ReserveCollateral,
    /// Reserve liquidity
    pub liquidity: ReserveLiquidity,
    /// Reserve configuration values
    pub config: ReserveConfig,
}

/// Borrow liquidity result
#[derive(Debug)]
pub struct BorrowLiquidityResult {
    /// Total amount of borrow including fees
    pub borrow_amount: u64,
    /// Borrow amount portion of total amount
    pub receive_amount: u64,
    /// Loan origination fee
    pub borrow_fee: u64,
    /// Host fee portion of origination fee
    pub host_fee: u64,
}

/// Repay liquidity result
#[derive(Debug)]
pub struct RepayLiquidityResult {
    /// Amount of liquidity that is settled from the obligation.
    pub settle_amount: Decimal,
    /// Amount that will be repaid as u64
    pub repay_amount: u64,
}

/// Liquidate obligation result
#[derive(Debug)]
pub struct LiquidateObligationResult {
    /// Amount of liquidity that is settled from the obligation. It includes
    /// the amount of loan that was defaulted if collateral is depleted.
    pub settle_amount: Decimal,
    /// Amount that will be repaid as u64
    pub repay_amount: u64,
    /// Amount of collateral to withdraw in exchange for repay amount
    pub withdraw_amount: u64,
}

/// Reserve liquidity
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReserveLiquidity {
    /// Reserve liquidity mint address
    pub mint_pubkey: Pubkey,
    /// Reserve liquidity mint decimals
    pub mint_decimals: u8,
    /// Reserve liquidity supply address
    pub supply_pubkey: Pubkey,
    /// Reserve liquidity fee receiver address
    pub fee_receiver: Pubkey,
    /// Optional reserve liquidity aggregator state account
    pub aggregator: COption<Pubkey>,
    /// Reserve liquidity cumulative borrow rate
    pub cumulative_borrow_rate_wads: Decimal,
    /// Reserve liquidity median price in quote currency
    pub median_price: u64,
    /// Reserve liquidity available
    pub available_amount: u64,
    /// Reserve liquidity borrowed
    pub borrowed_amount_wads: Decimal,
}

impl ReserveLiquidity {
    /// Initialize new reserve state
    pub fn new(params: NewReserveLiquidityParams) -> Self {
        let NewReserveLiquidityParams {
            mint_pubkey,
            mint_decimals,
            supply_pubkey,
            fee_receiver,
            aggregator,
            median_price,
        } = params;
        Self {
            mint_pubkey,
            mint_decimals,
            supply_pubkey,
            fee_receiver,
            aggregator,
            cumulative_borrow_rate_wads: Decimal::one(),
            median_price,
            available_amount: 0,
            borrowed_amount_wads: Decimal::zero(),
        }
    }

    /// Calculate the total reserve supply including active loans
    pub fn total_supply(&self) -> Result<Decimal, ProgramError> {
        Decimal::from(self.available_amount).try_add(self.borrowed_amount_wads)
    }

    /// Subtract borrow amount from available liquidity and add to borrows
    pub fn borrow(&mut self, borrow_amount: u64) -> ProgramResult {
        if borrow_amount > self.available_amount {
            return Err(LendingError::InsufficientLiquidity.into());
        }

        self.available_amount = self
            .available_amount
            .checked_sub(borrow_amount)
            .ok_or(LendingError::MathOverflow)?;
        self.borrowed_amount_wads = self.borrowed_amount_wads.try_add(borrow_amount.into())?;

        Ok(())
    }

    /// Add repay amount to available liquidity and subtract settle amount from total borrows
    pub fn repay(&mut self, repay_amount: u64, settle_amount: Decimal) -> ProgramResult {
        self.available_amount = self
            .available_amount
            .checked_add(repay_amount)
            .ok_or(LendingError::MathOverflow)?;
        self.borrowed_amount_wads = self.borrowed_amount_wads.try_sub(settle_amount)?;

        Ok(())
    }

    /// Calculate the liquidity utilization rate of the reserve
    pub fn utilization_rate(&self) -> Result<Rate, ProgramError> {
        let total_supply = self.total_supply()?;
        if total_supply == Decimal::zero() {
            return Ok(Rate::zero());
        }
        self.borrowed_amount_wads.try_div(total_supply)?.try_into()
    }

    /// Compound current borrow rate over elapsed slots
    fn compound_interest(
        &mut self,
        current_borrow_rate: Rate,
        slots_elapsed: u64,
    ) -> ProgramResult {
        let slot_interest_rate: Rate = current_borrow_rate.try_div(SLOTS_PER_YEAR)?;
        let compounded_interest_rate = Rate::one()
            .try_add(slot_interest_rate)?
            .try_pow(slots_elapsed)?;
        self.cumulative_borrow_rate_wads = self
            .cumulative_borrow_rate_wads
            .try_mul(compounded_interest_rate)?;
        self.borrowed_amount_wads = self
            .borrowed_amount_wads
            .try_mul(compounded_interest_rate)?;
        Ok(())
    }
}



/// Reserve liquidity
#[derive(Clone, Debug, Default, PartialEq)]
pub struct NewReserveLiquidityParams {
    /// Reserve liquidity mint address
    pub mint_pubkey: Pubkey,
    /// Reserve liquidity mint decimals
    pub mint_decimals: u8,
    /// Reserve liquidity supply address
    pub supply_pubkey: Pubkey,
    /// Reserve liquidity fee receiver address
    pub fee_receiver: Pubkey,
    /// Reserve liquidity aggregator state account
    pub aggregator: COption<Pubkey>,
    /// Reserve liquidity median price
    pub median_price: u64,
}

/// Reserve collateral
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ReserveCollateral {
    /// Reserve collateral mint address
    pub mint_pubkey: Pubkey,
    /// Reserve collateral mint supply, used for exchange rate
    pub mint_total_supply: u64,
    /// Reserve collateral supply address
    pub supply_pubkey: Pubkey,
}

impl ReserveCollateral {
    /// New reserve collateral info
    pub fn new(mint_pubkey: Pubkey, supply_pubkey: Pubkey) -> Self {
        Self {
            mint_pubkey,
            mint_total_supply: 0,
            supply_pubkey,
        }
    }

    /// Return the current collateral exchange rate.
    fn exchange_rate(
        &self,
        total_liquidity: Decimal,
    ) -> Result<CollateralExchangeRate, ProgramError> {
        let rate = if self.mint_total_supply == 0 || total_liquidity == Decimal::zero() {
            Rate::from_scaled_val(INITIAL_COLLATERAL_RATE)
        } else {
            let collateral_supply = Decimal::from(self.mint_total_supply);
            Rate::try_from(collateral_supply.try_div(total_liquidity)?)?
        };

        Ok(CollateralExchangeRate(rate))
    }
}

/// Collateral exchange rate
#[derive(Clone, Copy, Debug)]
pub struct CollateralExchangeRate(Rate);

impl CollateralExchangeRate {
    /// Convert reserve collateral to liquidity
    pub fn collateral_to_liquidity(&self, collateral_amount: u64) -> Result<u64, ProgramError> {
        Decimal::from(collateral_amount)
            .try_div(self.0)?
            .try_round_u64()
    }

    /// Convert reserve collateral to liquidity
    pub fn decimal_collateral_to_liquidity(
        &self,
        collateral_amount: Decimal,
    ) -> Result<Decimal, ProgramError> {
        collateral_amount.try_div(self.0)
    }

    /// Convert reserve liquidity to collateral
    pub fn liquidity_to_collateral(&self, liquidity_amount: u64) -> Result<u64, ProgramError> {
        self.0.try_mul(liquidity_amount)?.try_round_u64()
    }

    /// Convert reserve liquidity to collateral
    pub fn decimal_liquidity_to_collateral(
        &self,
        liquidity_amount: Decimal,
    ) -> Result<Decimal, ProgramError> {
        liquidity_amount.try_mul(self.0)
    }
}

impl From<CollateralExchangeRate> for Rate {
    fn from(exchange_rate: CollateralExchangeRate) -> Self {
        exchange_rate.0
    }
}

/// Reserve configuration values
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReserveConfig {
    /// Optimal utilization rate as a percent
    pub optimal_utilization_rate: u8,
    /// The percent bonus the liquidator gets when repaying liquidity to an unhealthy obligation
    pub liquidation_bonus: u8,
    /// Min borrow APY
    pub min_borrow_rate: u8,
    /// Optimal (utilization) borrow APY
    pub optimal_borrow_rate: u8,
    /// Max borrow APY
    pub max_borrow_rate: u8,
    /// Program owner fees assessed, separate from gains due to interest accrual
    pub fees: ReserveFees,
}

/// Additional fee information on a reserve
///
/// These exist separately from interest accrual fees, and are specifically for the program owner
/// and frontend host. The fees are paid out as a percentage of liquidity token amounts during
/// repayments and liquidations.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReserveFees {
    /// Fee assessed on `BorrowObligationLiquidity`, expressed as a Wad.
    /// Must be between 0 and 10^18, such that 10^18 = 1.  A few examples for
    /// clarity:
    /// 1% = 10_000_000_000_000_000
    /// 0.01% (1 basis point) = 100_000_000_000_000
    /// 0.00001% (Aave borrow fee) = 100_000_000_000
    pub borrow_fee_wad: u64,
    /// Amount of fee going to host account, if provided in liquidate and repay
    pub host_fee_percentage: u8,
}

/// Calculate fees exlusive or inclusive of an amount
pub enum FeeCalculation {
    /// Fee added to amount: fee = rate * amount
    Exclusive,
    /// Fee included in amount: fee = (rate / (1 + rate)) * amount
    Inclusive,
}

impl ReserveFees {
    /// Calculate the owner and host fees on borrow
    pub fn calculate_borrow_fees(
        &self,
        borrow_amount: u64,
        fee_calculation: FeeCalculation,
    ) -> Result<(u64, u64), ProgramError> {
        let borrow_fee_rate = Rate::from_scaled_val(self.borrow_fee_wad);
        let host_fee_rate = Rate::from_percent(self.host_fee_percentage);
        if borrow_fee_rate > Rate::zero() && borrow_amount > 0 {
            let need_to_assess_host_fee = host_fee_rate > Rate::zero();
            let minimum_fee = if need_to_assess_host_fee {
                2 // 1 token to owner, 1 to host
            } else {
                1 // 1 token to owner, nothing else
            };

            let borrow_fee_amount = match fee_calculation {
                FeeCalculation::Exclusive => borrow_fee_rate.try_mul(borrow_amount)?,
                FeeCalculation::Inclusive => borrow_fee_rate
                    .try_div(borrow_fee_rate.try_add(Rate::one())?)?
                    .try_mul(borrow_amount)?,
            };

            let borrow_fee = borrow_fee_amount.try_round_u64()?.max(minimum_fee);

            let host_fee = if need_to_assess_host_fee {
                host_fee_rate.try_mul(borrow_fee)?.try_round_u64()?.max(1)
            } else {
                0
            };

            if borrow_fee >= borrow_amount {
                Err(LendingError::BorrowTooSmall.into())
            } else {
                Ok((borrow_fee, host_fee))
            }
        } else {
            Ok((0, 0))
        }
    }
}

impl Sealed for Reserve {}
impl IsInitialized for Reserve {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

const RESERVE_LEN: usize = 609; // 1 + 8 + 1 + 32 + 32 + 1 + 32 + 32 + (4 + 32) + 16 + 8 + 8 + 16 + 32 + 32 + 8 + 1 + 1 + 1 + 1 + 1 + 8 + 1 + 300
impl Pack for Reserve {
    const LEN: usize = RESERVE_LEN;

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, RESERVE_LEN];
        let (
            version,
            last_update_slot,
            last_update_stale,
            lending_market,
            liquidity_mint,
            liquidity_mint_decimals,
            liquidity_supply,
            liquidity_fee_receiver,
            liquidity_aggregator,
            liquidity_cumulative_borrow_rate_wads,
            liquidity_median_price,
            liquidity_available_amount,
            liquidity_borrowed_amount_wads,
            collateral_supply,
            collateral_mint,
            collateral_mint_supply,
            optimal_utilization_rate,
            liquidation_bonus,
            min_borrow_rate,
            optimal_borrow_rate,
            max_borrow_rate,
            borrow_fee_wad,
            host_fee_percentage,
            _padding,
        ) = mut_array_refs![
            output,
            1,
            8,
            1,
            PUBKEY_BYTES,
            PUBKEY_BYTES,
            1,
            PUBKEY_BYTES,
            PUBKEY_BYTES,
            4 + PUBKEY_BYTES,
            16,
            8,
            8,
            16,
            PUBKEY_BYTES,
            PUBKEY_BYTES,
            8,
            1,
            1,
            1,
            1,
            1,
            8,
            1,
            300
        ];
        *version = self.version.to_le_bytes();
        *last_update_slot = self.last_update.slot.to_le_bytes();
        *last_update_stale = u8::from(self.last_update.stale).to_le_bytes();
        lending_market.copy_from_slice(self.lending_market.as_ref());

        // liquidity
        liquidity_mint.copy_from_slice(self.liquidity.mint_pubkey.as_ref());
        *liquidity_mint_decimals = self.liquidity.mint_decimals.to_le_bytes();
        liquidity_supply.copy_from_slice(self.liquidity.supply_pubkey.as_ref());
        liquidity_fee_receiver.copy_from_slice(self.liquidity.fee_receiver.as_ref());
        pack_coption_key(&self.liquidity.aggregator, liquidity_aggregator);
        pack_decimal(
            self.liquidity.cumulative_borrow_rate_wads,
            liquidity_cumulative_borrow_rate_wads,
        );
        *liquidity_median_price = self.liquidity.median_price.to_le_bytes();
        *liquidity_available_amount = self.liquidity.available_amount.to_le_bytes();
        pack_decimal(
            self.liquidity.borrowed_amount_wads,
            liquidity_borrowed_amount_wads,
        );

        // collateral
        collateral_mint.copy_from_slice(self.collateral.mint_pubkey.as_ref());
        collateral_supply.copy_from_slice(self.collateral.supply_pubkey.as_ref());
        *collateral_mint_supply = self.collateral.mint_total_supply.to_le_bytes();

        // config
        *optimal_utilization_rate = self.config.optimal_utilization_rate.to_le_bytes();
        *liquidation_bonus = self.config.liquidation_bonus.to_le_bytes();
        *min_borrow_rate = self.config.min_borrow_rate.to_le_bytes();
        *optimal_borrow_rate = self.config.optimal_borrow_rate.to_le_bytes();
        *max_borrow_rate = self.config.max_borrow_rate.to_le_bytes();
        *borrow_fee_wad = self.config.fees.borrow_fee_wad.to_le_bytes();
        *host_fee_percentage = self.config.fees.host_fee_percentage.to_le_bytes();
    }

    /// Unpacks a byte buffer into a [ReserveInfo](struct.ReserveInfo.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, RESERVE_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            last_update_slot,
            last_update_stale,
            lending_market,
            liquidity_mint,
            liquidity_mint_decimals,
            liquidity_supply,
            liquidity_fee_receiver,
            liquidity_aggregator,
            liquidity_cumulative_borrow_rate_wads,
            liquidity_median_price,
            liquidity_available_amount,
            liquidity_borrowed_amount_wads,
            collateral_supply,
            collateral_mint,
            collateral_mint_supply,
            optimal_utilization_rate,
            liquidation_bonus,
            min_borrow_rate,
            optimal_borrow_rate,
            max_borrow_rate,
            borrow_fee_wad,
            host_fee_percentage,
            __padding,
        ) = array_refs![
            input,
            1,
            8,
            1,
            PUBKEY_BYTES,
            PUBKEY_BYTES,
            1,
            PUBKEY_BYTES,
            PUBKEY_BYTES,
            4 + PUBKEY_BYTES,
            16,
            8,
            8,
            16,
            PUBKEY_BYTES,
            PUBKEY_BYTES,
            8,
            1,
            1,
            1,
            1,
            1,
            8,
            1,
            300
        ];
        Ok(Self {
            version: u8::from_le_bytes(*version),
            last_update: LastUpdate {
                slot: u64::from_le_bytes(*last_update_slot),
                stale: bool::from(u8::from_le_bytes(*last_update_stale)),
            },
            lending_market: Pubkey::new_from_array(*lending_market),
            liquidity: ReserveLiquidity {
                mint_pubkey: Pubkey::new_from_array(*liquidity_mint),
                mint_decimals: u8::from_le_bytes(*liquidity_mint_decimals),
                supply_pubkey: Pubkey::new_from_array(*liquidity_supply),
                fee_receiver: Pubkey::new_from_array(*liquidity_fee_receiver),
                aggregator: unpack_coption_key(liquidity_aggregator)?,
                cumulative_borrow_rate_wads: unpack_decimal(liquidity_cumulative_borrow_rate_wads),
                median_price: u64::from_le_bytes(*liquidity_median_price),
                available_amount: u64::from_le_bytes(*liquidity_available_amount),
                borrowed_amount_wads: unpack_decimal(liquidity_borrowed_amount_wads),
            },
            collateral: ReserveCollateral {
                mint_pubkey: Pubkey::new_from_array(*collateral_mint),
                mint_total_supply: u64::from_le_bytes(*collateral_mint_supply),
                supply_pubkey: Pubkey::new_from_array(*collateral_supply),
            },
            config: ReserveConfig {
                optimal_utilization_rate: u8::from_le_bytes(*optimal_utilization_rate),
                liquidation_bonus: u8::from_le_bytes(*liquidation_bonus),
                min_borrow_rate: u8::from_le_bytes(*min_borrow_rate),
                optimal_borrow_rate: u8::from_le_bytes(*optimal_borrow_rate),
                max_borrow_rate: u8::from_le_bytes(*max_borrow_rate),
                fees: ReserveFees {
                    borrow_fee_wad: u64::from_le_bytes(*borrow_fee_wad),
                    host_fee_percentage: u8::from_le_bytes(*host_fee_percentage),
                },
            },
        })
    }
}