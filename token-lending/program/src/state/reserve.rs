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
    program_option::COption,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};
use std::convert::{TryFrom, TryInto};

/// Lending market reserve state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reserve {
    /// Version of the struct
    pub version: u8,
    /// Last slot when supply and rates updated
    pub last_update_slot: Slot,
    /// Cumulative borrow rate
    pub cumulative_borrow_rate_wads: Decimal,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Dex market state account
    pub dex_market: COption<Pubkey>,
    /// Reserve liquidity info
    pub liquidity: ReserveLiquidity,
    /// Reserve collateral info
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
            collateral: collateral_info,
            liquidity: liquidity_info,
            dex_market,
            config,
        } = params;

        Self {
            version: PROGRAM_VERSION,
            last_update_slot: current_slot,
            cumulative_borrow_rate_wads: Decimal::one(),
            lending_market,
            collateral: collateral_info,
            liquidity: liquidity_info,
            dex_market,
            config,
        }
    }

    /// Calculate the current borrow rate
    pub fn current_borrow_rate(&self) -> Result<Rate, ProgramError> {
        let utilization_rate = self.liquidity.utilization_rate()?;
        let optimal_utilization_rate = Rate::from_percent(self.config.optimal_utilization_rate);
        let low_utilization = utilization_rate < optimal_utilization_rate;
        if low_utilization || self.config.optimal_utilization_rate == 100 {
            let normalized_rate = utilization_rate.try_div(optimal_utilization_rate)?;
            let min_rate = Rate::from_percent(self.config.min_borrow_rate);
            let rate_range =
                Rate::from_percent(self.config.optimal_borrow_rate - self.config.min_borrow_rate);

            Ok(normalized_rate.try_mul(rate_range)?.try_add(min_rate)?)
        } else {
            let normalized_rate = utilization_rate
                .try_sub(optimal_utilization_rate)?
                .try_div(Rate::from_percent(
                    100 - self.config.optimal_utilization_rate,
                ))?;
            let min_rate = Rate::from_percent(self.config.optimal_borrow_rate);
            let rate_range =
                Rate::from_percent(self.config.max_borrow_rate - self.config.optimal_borrow_rate);

            Ok(normalized_rate.try_mul(rate_range)?.try_add(min_rate)?)
        }
    }

    /// Record deposited liquidity and return amount of collateral tokens to mint
    pub fn deposit_liquidity(&mut self, liquidity_amount: u64) -> Result<u64, ProgramError> {
        let collateral_exchange_rate = self.collateral_exchange_rate()?;
        let collateral_amount =
            collateral_exchange_rate.liquidity_to_collateral(liquidity_amount)?;

        self.liquidity.available_amount += liquidity_amount;
        self.collateral.mint_total_supply += collateral_amount;

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

        self.liquidity.available_amount -= liquidity_amount;
        self.collateral.mint_total_supply -= collateral_amount;

        Ok(liquidity_amount)
    }

    /// Update borrow rate and accrue interest
    pub fn accrue_interest(&mut self, current_slot: Slot) -> Result<(), ProgramError> {
        let slots_elapsed = self.update_slot(current_slot);
        if slots_elapsed > 0 {
            let current_borrow_rate = self.current_borrow_rate()?;
            let compounded_interest_rate =
                self.compound_interest(current_borrow_rate, slots_elapsed)?;
            self.liquidity.borrowed_amount_wads = self
                .liquidity
                .borrowed_amount_wads
                .try_mul(compounded_interest_rate)?;
        }
        Ok(())
    }

    /// Collateral exchange rate
    pub fn collateral_exchange_rate(&self) -> Result<CollateralExchangeRate, ProgramError> {
        let total_liquidity = self.liquidity.total_supply()?;
        self.collateral.exchange_rate(total_liquidity)
    }

    /// Return slots elapsed since last update
    fn update_slot(&mut self, slot: Slot) -> u64 {
        let slots_elapsed = slot - self.last_update_slot;
        self.last_update_slot = slot;
        slots_elapsed
    }

    /// Compound current borrow rate over elapsed slots
    fn compound_interest(
        &mut self,
        current_borrow_rate: Rate,
        slots_elapsed: u64,
    ) -> Result<Rate, ProgramError> {
        let slot_interest_rate: Rate = current_borrow_rate.try_div(SLOTS_PER_YEAR)?;
        let compounded_interest_rate = Rate::one()
            .try_add(slot_interest_rate)?
            .try_pow(slots_elapsed)?;
        self.cumulative_borrow_rate_wads = self
            .cumulative_borrow_rate_wads
            .try_mul(compounded_interest_rate)?;
        Ok(compounded_interest_rate)
    }
}

/// Create new reserve
pub struct NewReserveParams {
    /// Current slot
    pub current_slot: Slot,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Reserve collateral info
    pub collateral: ReserveCollateral,
    /// Reserve liquidity info
    pub liquidity: ReserveLiquidity,
    /// Optional dex market address
    pub dex_market: COption<Pubkey>,
    /// Reserve configuration values
    pub config: ReserveConfig,
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
    /// Reserve liquidity available
    pub available_amount: u64,
    /// Reserve liquidity borrowed
    pub borrowed_amount_wads: Decimal,
}

impl ReserveLiquidity {
    /// New reserve liquidity info
    pub fn new(mint_pubkey: Pubkey, mint_decimals: u8, supply_pubkey: Pubkey) -> Self {
        Self {
            mint_pubkey,
            mint_decimals,
            supply_pubkey,
            available_amount: 0,
            borrowed_amount_wads: Decimal::zero(),
        }
    }

    /// Calculate the total reserve supply including active loans
    pub fn total_supply(&self) -> Result<Decimal, ProgramError> {
        Decimal::from(self.available_amount).try_add(self.borrowed_amount_wads)
    }

    /// Add new borrow amount to total borrows
    pub fn borrow(&mut self, borrow_amount: u64) -> ProgramResult {
        if borrow_amount > self.available_amount {
            return Err(LendingError::InsufficientLiquidity.into());
        }

        self.available_amount -= borrow_amount;
        self.borrowed_amount_wads = self
            .borrowed_amount_wads
            .try_add(Decimal::from(borrow_amount))?;
        Ok(())
    }

    /// Subtract repay amount from total borrows and add to available liquidity
    pub fn repay(
        &mut self,
        integer_amount: u64,
        decimal_amount: Decimal,
    ) -> Result<(), ProgramError> {
        self.available_amount = self
            .available_amount
            .checked_add(integer_amount)
            .ok_or(LendingError::MathOverflow)?;
        self.borrowed_amount_wads = self.borrowed_amount_wads.try_sub(decimal_amount)?;

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
    /// Reserve collateral fees receiver address
    pub fees_receiver: Pubkey,
}

impl ReserveCollateral {
    /// New reserve collateral info
    pub fn new(mint_pubkey: Pubkey, supply_pubkey: Pubkey, fees_receiver: Pubkey) -> Self {
        Self {
            mint_pubkey,
            supply_pubkey,
            fees_receiver,
            ..Self::default()
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
    /// The ratio of the loan to the value of the collateral as a percent
    pub loan_to_value_ratio: u8,
    /// The percent discount the liquidator gets when buying collateral for an unhealthy obligation
    pub liquidation_bonus: u8,
    /// The percent at which an obligation is considered unhealthy
    pub liquidation_threshold: u8,
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
/// These exist separately from interest accrual fees, and are specifically for
/// the program owner and frontend host.  The fees are paid out as a percentage
/// of collateral token amounts during repayments and liquidations.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReserveFees {
    /// Fee assessed on `BorrowReserveLiquidity`, expressed as a Wad.
    /// Must be between 0 and 10^18, such that 10^18 = 1.  A few examples for
    /// clarity:
    /// 1% = 10_000_000_000_000_000
    /// 0.01% (1 basis point) = 100_000_000_000_000
    /// 0.00001% (Aave borrow fee) = 100_000_000_000
    pub borrow_fee_wad: u64,
    /// Amount of fee going to host account, if provided in liquidate and repay
    pub host_fee_percentage: u8,
}

impl ReserveFees {
    /// Calculate the owner and host fees on borrow
    pub fn calculate_borrow_fees(
        &self,
        collateral_amount: u64,
    ) -> Result<(u64, u64), ProgramError> {
        let borrow_fee_rate = Rate::from_scaled_val(self.borrow_fee_wad);
        let host_fee_rate = Rate::from_percent(self.host_fee_percentage);
        if borrow_fee_rate > Rate::zero() && collateral_amount > 0 {
            let need_to_assess_host_fee = host_fee_rate > Rate::zero();
            let minimum_fee = if need_to_assess_host_fee {
                2 // 1 token to owner, 1 to host
            } else {
                1 // 1 token to owner, nothing else
            };

            let borrow_fee = borrow_fee_rate
                .try_mul(collateral_amount)?
                .try_round_u64()?
                .max(minimum_fee);

            let host_fee = if need_to_assess_host_fee {
                host_fee_rate.try_mul(borrow_fee)?.try_round_u64()?.max(1)
            } else {
                0
            };

            if borrow_fee >= collateral_amount {
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

const RESERVE_LEN: usize = 602;
impl Pack for Reserve {
    const LEN: usize = 602;

    /// Unpacks a byte buffer into a [ReserveInfo](struct.ReserveInfo.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, RESERVE_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            version,
            last_update_slot,
            lending_market,
            liquidity_mint,
            liquidity_mint_decimals,
            liquidity_supply,
            collateral_mint,
            collateral_supply,
            collateral_fees_receiver,
            dex_market,
            optimal_utilization_rate,
            loan_to_value_ratio,
            liquidation_bonus,
            liquidation_threshold,
            min_borrow_rate,
            optimal_borrow_rate,
            max_borrow_rate,
            borrow_fee_wad,
            host_fee_percentage,
            cumulative_borrow_rate,
            total_borrows,
            available_liquidity,
            collateral_mint_supply,
            __padding,
        ) = array_refs![
            input, 1, 8, 32, 32, 1, 32, 32, 32, 32, 36, 1, 1, 1, 1, 1, 1, 1, 8, 1, 16, 16, 8, 8,
            300
        ];
        Ok(Self {
            version: u8::from_le_bytes(*version),
            last_update_slot: u64::from_le_bytes(*last_update_slot),
            cumulative_borrow_rate_wads: unpack_decimal(cumulative_borrow_rate),
            lending_market: Pubkey::new_from_array(*lending_market),
            dex_market: unpack_coption_key(dex_market)?,
            liquidity: ReserveLiquidity {
                mint_pubkey: Pubkey::new_from_array(*liquidity_mint),
                mint_decimals: u8::from_le_bytes(*liquidity_mint_decimals),
                supply_pubkey: Pubkey::new_from_array(*liquidity_supply),
                available_amount: u64::from_le_bytes(*available_liquidity),
                borrowed_amount_wads: unpack_decimal(total_borrows),
            },
            collateral: ReserveCollateral {
                mint_pubkey: Pubkey::new_from_array(*collateral_mint),
                mint_total_supply: u64::from_le_bytes(*collateral_mint_supply),
                supply_pubkey: Pubkey::new_from_array(*collateral_supply),
                fees_receiver: Pubkey::new_from_array(*collateral_fees_receiver),
            },
            config: ReserveConfig {
                optimal_utilization_rate: u8::from_le_bytes(*optimal_utilization_rate),
                loan_to_value_ratio: u8::from_le_bytes(*loan_to_value_ratio),
                liquidation_bonus: u8::from_le_bytes(*liquidation_bonus),
                liquidation_threshold: u8::from_le_bytes(*liquidation_threshold),
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

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, RESERVE_LEN];
        let (
            version,
            last_update_slot,
            lending_market,
            liquidity_mint,
            liquidity_mint_decimals,
            liquidity_supply,
            collateral_mint,
            collateral_supply,
            collateral_fees_receiver,
            dex_market,
            optimal_utilization_rate,
            loan_to_value_ratio,
            liquidation_bonus,
            liquidation_threshold,
            min_borrow_rate,
            optimal_borrow_rate,
            max_borrow_rate,
            borrow_fee_wad,
            host_fee_percentage,
            cumulative_borrow_rate,
            total_borrows,
            available_liquidity,
            collateral_mint_supply,
            _padding,
        ) = mut_array_refs![
            output, 1, 8, 32, 32, 1, 32, 32, 32, 32, 36, 1, 1, 1, 1, 1, 1, 1, 8, 1, 16, 16, 8, 8,
            300
        ];
        *version = self.version.to_le_bytes();
        *last_update_slot = self.last_update_slot.to_le_bytes();
        pack_decimal(self.cumulative_borrow_rate_wads, cumulative_borrow_rate);
        lending_market.copy_from_slice(self.lending_market.as_ref());
        pack_coption_key(&self.dex_market, dex_market);

        // liquidity info
        liquidity_mint.copy_from_slice(self.liquidity.mint_pubkey.as_ref());
        *liquidity_mint_decimals = self.liquidity.mint_decimals.to_le_bytes();
        liquidity_supply.copy_from_slice(self.liquidity.supply_pubkey.as_ref());
        *available_liquidity = self.liquidity.available_amount.to_le_bytes();
        pack_decimal(self.liquidity.borrowed_amount_wads, total_borrows);

        // collateral info
        collateral_mint.copy_from_slice(self.collateral.mint_pubkey.as_ref());
        collateral_supply.copy_from_slice(self.collateral.supply_pubkey.as_ref());
        collateral_fees_receiver.copy_from_slice(self.collateral.fees_receiver.as_ref());
        *collateral_mint_supply = self.collateral.mint_total_supply.to_le_bytes();

        // config
        *optimal_utilization_rate = self.config.optimal_utilization_rate.to_le_bytes();
        *loan_to_value_ratio = self.config.loan_to_value_ratio.to_le_bytes();
        *liquidation_bonus = self.config.liquidation_bonus.to_le_bytes();
        *liquidation_threshold = self.config.liquidation_threshold.to_le_bytes();
        *min_borrow_rate = self.config.min_borrow_rate.to_le_bytes();
        *optimal_borrow_rate = self.config.optimal_borrow_rate.to_le_bytes();
        *max_borrow_rate = self.config.max_borrow_rate.to_le_bytes();
        *borrow_fee_wad = self.config.fees.borrow_fee_wad.to_le_bytes();
        *host_fee_percentage = self.config.fees.host_fee_percentage.to_le_bytes();
    }
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::math::WAD;
    use proptest::prelude::*;
    use std::cmp::Ordering;

    const MAX_LIQUIDITY: u64 = u64::MAX / 5;

    // Creates rates (min, opt, max) where 0 <= min <= opt <= max <= MAX
    prop_compose! {
        fn borrow_rates()(optimal_rate in 0..=u8::MAX)(
            min_rate in 0..=optimal_rate,
            optimal_rate in Just(optimal_rate),
            max_rate in optimal_rate..=u8::MAX,
        ) -> (u8, u8, u8) {
            (min_rate, optimal_rate, max_rate)
        }
    }

    proptest! {
        #[test]
        fn current_borrow_rate(
            total_liquidity in 0..=MAX_LIQUIDITY,
            borrowed_percent in 0..=WAD,
            optimal_utilization_rate in 0..=100u8,
            (min_borrow_rate, optimal_borrow_rate, max_borrow_rate) in borrow_rates(),
        ) {
            let borrowed_amount_wads = Decimal::from(total_liquidity).try_mul(Rate::from_scaled_val(borrowed_percent))?;
            let reserve = Reserve {
                liquidity: ReserveLiquidity {
                    borrowed_amount_wads,
                    available_amount: total_liquidity - borrowed_amount_wads.try_round_u64()?,
                    ..ReserveLiquidity::default()
                },
                config: ReserveConfig {
                    min_borrow_rate,
                    optimal_borrow_rate,
                    max_borrow_rate,
                    optimal_utilization_rate,
                    ..ReserveConfig::default()
                },
                ..Reserve::default()
            };

            let current_borrow_rate = reserve.current_borrow_rate()?;
            assert!(current_borrow_rate >= Rate::from_percent(min_borrow_rate));
            assert!(current_borrow_rate <= Rate::from_percent(max_borrow_rate));

            let optimal_borrow_rate = Rate::from_percent(optimal_borrow_rate);
            let current_rate = reserve.liquidity.utilization_rate()?;
            match current_rate.cmp(&Rate::from_percent(optimal_utilization_rate)) {
                Ordering::Less => {
                    if min_borrow_rate == reserve.config.optimal_borrow_rate {
                        assert_eq!(current_borrow_rate, optimal_borrow_rate);
                    } else {
                        assert!(current_borrow_rate < optimal_borrow_rate);
                    }
                }
                Ordering::Equal => assert!(current_borrow_rate == optimal_borrow_rate),
                Ordering::Greater => {
                    if max_borrow_rate == reserve.config.optimal_borrow_rate {
                        assert_eq!(current_borrow_rate, optimal_borrow_rate);
                    } else {
                        assert!(current_borrow_rate > optimal_borrow_rate);
                    }
                }
            }
        }

        #[test]
        fn current_utilization_rate(
            total_liquidity in 0..=MAX_LIQUIDITY,
            borrowed_percent in 0..=WAD,
        ) {
            let borrowed_amount_wads = Decimal::from(total_liquidity).try_mul(Rate::from_scaled_val(borrowed_percent))?;
            let liquidity = ReserveLiquidity {
                borrowed_amount_wads,
                available_amount: total_liquidity - borrowed_amount_wads.try_round_u64()?,
                ..ReserveLiquidity::default()
            };

            let current_rate = liquidity.utilization_rate()?;
            assert!(current_rate <= Rate::one());
        }

        #[test]
        fn collateral_exchange_rate(
            total_liquidity in 0..=MAX_LIQUIDITY,
            borrowed_percent in 0..=WAD,
            collateral_multiplier in 0..=(5*WAD),
            borrow_rate in 0..=u8::MAX,
        ) {
            let borrowed_liquidity_wads = Decimal::from(total_liquidity).try_mul(Rate::from_scaled_val(borrowed_percent))?;
            let available_liquidity = total_liquidity - borrowed_liquidity_wads.try_round_u64()?;
            let mint_total_supply = Decimal::from(total_liquidity).try_mul(Rate::from_scaled_val(collateral_multiplier))?.try_round_u64()?;

            let mut reserve = Reserve {
                collateral: ReserveCollateral {
                    mint_total_supply,
                    ..ReserveCollateral::default()
                },
                liquidity: ReserveLiquidity {
                    borrowed_amount_wads: borrowed_liquidity_wads,
                    available_amount: available_liquidity,
                    ..ReserveLiquidity::default()
                },
                config: ReserveConfig {
                    min_borrow_rate: borrow_rate,
                    optimal_borrow_rate: borrow_rate,
                    optimal_utilization_rate: 100,
                    ..ReserveConfig::default()
                },
                ..Reserve::default()
            };

            let exchange_rate = reserve.collateral_exchange_rate()?;
            assert!(exchange_rate.0.to_scaled_val() <= 5u128 * WAD as u128);

            // After interest accrual, total liquidity increases and collateral are worth more
            reserve.accrue_interest(1)?;

            let new_exchange_rate = reserve.collateral_exchange_rate()?;
            if borrow_rate > 0 && total_liquidity > 0 && borrowed_percent > 0 {
                assert!(new_exchange_rate.0 < exchange_rate.0);
            } else {
                assert_eq!(new_exchange_rate.0, exchange_rate.0);
            }
        }

        #[test]
        fn compound_interest(
            slots_elapsed in 0..=SLOTS_PER_YEAR,
            borrow_rate in 0..=u8::MAX,
        ) {
            let mut reserve = Reserve::default();
            let borrow_rate = Rate::from_percent(borrow_rate);

            // Simulate running for max 1000 years, assuming that interest is
            // compounded at least once a year
            for _ in 0..1000 {
                reserve.compound_interest(borrow_rate, slots_elapsed)?;
                reserve.cumulative_borrow_rate_wads.to_scaled_val()?;
            }
        }

        #[test]
        fn reserve_accrue_interest(
            slots_elapsed in 0..=SLOTS_PER_YEAR,
            borrowed_liquidity in 0..=u64::MAX,
            borrow_rate in 0..=u8::MAX,
        ) {
            let borrowed_amount_wads = Decimal::from(borrowed_liquidity);
            let mut reserve = Reserve {
                liquidity: ReserveLiquidity {
                    borrowed_amount_wads,
                    ..ReserveLiquidity::default()
                },
                config: ReserveConfig {
                    max_borrow_rate: borrow_rate,
                    ..ReserveConfig::default()
                },
                ..Reserve::default()
            };

            reserve.accrue_interest(slots_elapsed)?;

            if borrow_rate > 0 && slots_elapsed > 0 {
                assert!(reserve.liquidity.borrowed_amount_wads > borrowed_amount_wads);
            } else {
                assert!(reserve.liquidity.borrowed_amount_wads == borrowed_amount_wads);
            }
        }

        #[test]
        fn borrow_fee_calculation(
            borrow_fee_wad in 0..WAD, // at WAD, fee == borrow amount, which fails
            host_fee_percentage in 0..=100u8,
            borrow_amount in 3..=u64::MAX, // start at 3 to ensure calculation success
                                           // 0, 1, and 2 are covered in the minimum tests
        ) {
            let fees = ReserveFees {
                borrow_fee_wad,
                host_fee_percentage,
            };
            let (total_fee, host_fee) = fees.calculate_borrow_fees(borrow_amount)?;

            // The total fee can't be greater than the amount borrowed, as long
            // as amount borrowed is greater than 2.
            // At a borrow amount of 2, we can get a total fee of 2 if a host
            // fee is also specified.
            assert!(total_fee <= borrow_amount);

            // the host fee can't be greater than the total fee
            assert!(host_fee <= total_fee);

            // for all fee rates greater than 0, we must have some fee
            if borrow_fee_wad > 0 {
                assert!(total_fee > 0);
            }

            if host_fee_percentage == 100 {
                // if the host fee percentage is maxed at 100%, it should get all the fee
                assert_eq!(host_fee, total_fee);
            }

            // if there's a host fee and some borrow fee, host fee must be greater than 0
            if host_fee_percentage > 0 && borrow_fee_wad > 0 {
                assert!(host_fee > 0);
            } else {
                assert_eq!(host_fee, 0);
            }
        }
    }

    #[test]
    fn borrow_fee_calculation_min_host() {
        let fees = ReserveFees {
            borrow_fee_wad: 10_000_000_000_000_000, // 1%
            host_fee_percentage: 20,
        };

        // only 2 tokens borrowed, get error
        let err = fees.calculate_borrow_fees(2).unwrap_err();
        assert_eq!(err, LendingError::BorrowTooSmall.into()); // minimum of 3 tokens

        // only 1 token borrowed, get error
        let err = fees.calculate_borrow_fees(1).unwrap_err();
        assert_eq!(err, LendingError::BorrowTooSmall.into());

        // 0 amount borrowed, 0 fee
        let (total_fee, host_fee) = fees.calculate_borrow_fees(0).unwrap();
        assert_eq!(total_fee, 0);
        assert_eq!(host_fee, 0);
    }

    #[test]
    fn borrow_fee_calculation_min_no_host() {
        let fees = ReserveFees {
            borrow_fee_wad: 10_000_000_000_000_000, // 1%
            host_fee_percentage: 0,
        };

        // only 2 tokens borrowed, ok
        let (total_fee, host_fee) = fees.calculate_borrow_fees(2).unwrap();
        assert_eq!(total_fee, 1);
        assert_eq!(host_fee, 0);

        // only 1 token borrowed, get error
        let err = fees.calculate_borrow_fees(1).unwrap_err();
        assert_eq!(err, LendingError::BorrowTooSmall.into()); // minimum of 2 tokens

        // 0 amount borrowed, 0 fee
        let (total_fee, host_fee) = fees.calculate_borrow_fees(0).unwrap();
        assert_eq!(total_fee, 0);
        assert_eq!(host_fee, 0);
    }

    #[test]
    fn borrow_fee_calculation_host() {
        let fees = ReserveFees {
            borrow_fee_wad: 10_000_000_000_000_000, // 1%
            host_fee_percentage: 20,
        };

        let (total_fee, host_fee) = fees.calculate_borrow_fees(1000).unwrap();

        assert_eq!(total_fee, 10); // 1% of 1000
        assert_eq!(host_fee, 2); // 20% of 10
    }

    #[test]
    fn borrow_fee_calculation_no_host() {
        let fees = ReserveFees {
            borrow_fee_wad: 10_000_000_000_000_000, // 1%
            host_fee_percentage: 0,
        };

        let (total_fee, host_fee) = fees.calculate_borrow_fees(1000).unwrap();

        assert_eq!(total_fee, 10); // 1% of 1000
        assert_eq!(host_fee, 0); // 0 host fee
    }
}
