//! State types

use crate::{
    error::LendingError,
    math::{Decimal, Rate, SCALE},
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    clock::{Slot, DEFAULT_TICKS_PER_SECOND, DEFAULT_TICKS_PER_SLOT, SECONDS_PER_DAY},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_option::COption,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
};

/// Collateral tokens are initially valued at a ratio of 5:1 (collateral:liquidity)
pub const INITIAL_COLLATERAL_RATE: u64 = 5;

/// Current version of the program and all new accounts created
pub const PROGRAM_VERSION: u8 = 1;

/// Accounts are created with data zeroed out, so uninitialized state instances
/// will have the version set to 0.
const UNINITIALIZED_VERSION: u8 = 0;

/// Number of slots per year
pub const SLOTS_PER_YEAR: u64 =
    DEFAULT_TICKS_PER_SECOND / DEFAULT_TICKS_PER_SLOT * SECONDS_PER_DAY * 365;

/// Lending market state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LendingMarket {
    /// Version of lending market
    pub version: u8,
    /// Bump seed for derived authority address
    pub bump_seed: u8,
    /// Quote currency token mint
    pub quote_token_mint: Pubkey,
    /// Token program id
    pub token_program_id: Pubkey,
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
        let borrow_fee_rate = Rate::new(self.borrow_fee_wad, SCALE);
        let host_fee_rate = Rate::from_percent(self.host_fee_percentage);
        if borrow_fee_rate > Rate::zero() && collateral_amount > 0 {
            let need_to_assess_host_fee = host_fee_rate > Rate::zero();
            let minimum_fee = if need_to_assess_host_fee {
                2 // 1 token to owner, 1 to host
            } else {
                1 // 1 token to owner, nothing else
            };
            let borrow_fee = std::cmp::max(
                minimum_fee,
                (borrow_fee_rate * collateral_amount).round_u64(),
            );
            let host_fee = if need_to_assess_host_fee {
                std::cmp::max(1, (host_fee_rate * borrow_fee).round_u64())
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

/// Reserve state
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ReserveState {
    /// Last slot when supply and rates updated
    pub last_update_slot: Slot,
    /// Cumulative borrow rate
    pub cumulative_borrow_rate_wads: Decimal,
    /// Borrowed liquidity, plus interest
    pub borrowed_liquidity_wads: Decimal,
    /// Available liquidity currently held in reserve
    pub available_liquidity: u64,
    /// Total collateral mint supply, used to calculate exchange rate
    pub collateral_mint_supply: u64,
}

impl ReserveState {
    /// Initialize new reserve state
    pub fn new(current_slot: Slot, liquidity_amount: u64) -> Self {
        Self {
            last_update_slot: current_slot,
            cumulative_borrow_rate_wads: Decimal::one(),
            available_liquidity: liquidity_amount,
            collateral_mint_supply: INITIAL_COLLATERAL_RATE * liquidity_amount, // TODO check overflow
            borrowed_liquidity_wads: Decimal::zero(),
        }
    }
}

/// Lending market reserve state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct Reserve {
    /// Version of the struct
    pub version: u8,
    /// Lending market address
    pub lending_market: Pubkey,
    /// Reserve liquidity mint
    pub liquidity_mint: Pubkey,
    /// Reserve liquidity supply
    pub liquidity_mint_decimals: u8,
    /// Reserve liquidity supply
    pub liquidity_supply: Pubkey,
    /// Collateral tokens are minted when liquidity is deposited in the reserve.
    /// Collateral tokens can be withdrawn back to the underlying liquidity token.
    pub collateral_mint: Pubkey,
    /// Reserve collateral supply
    /// Collateral is stored rather than burned to keep an accurate total collateral supply
    pub collateral_supply: Pubkey,
    /// Collateral account receiving owner fees on liquidate and repay
    pub collateral_fees_receiver: Pubkey,
    /// Dex market state account
    pub dex_market: COption<Pubkey>,

    /// Reserve state
    pub state: ReserveState,

    /// Reserve configuration values
    pub config: ReserveConfig,
}

/// Collateral exchange rate
pub struct CollateralExchangeRate(Rate);

impl CollateralExchangeRate {
    /// Convert reserve collateral to liquidity
    pub fn collateral_to_liquidity(&self, collateral_amount: u64) -> u64 {
        (Decimal::from(collateral_amount) / self.0).round_u64()
    }

    /// Convert reserve collateral to liquidity
    pub fn decimal_collateral_to_liquidity(&self, collateral_amount: Decimal) -> Decimal {
        collateral_amount / self.0
    }

    /// Convert reserve liquidity to collateral
    pub fn liquidity_to_collateral(&self, liquidity_amount: u64) -> u64 {
        (self.0 * liquidity_amount).round_u64()
    }

    /// Convert reserve liquidity to collateral
    pub fn decimal_liquidity_to_collateral(&self, liquidity_amount: Decimal) -> Decimal {
        liquidity_amount * self.0
    }
}

impl From<CollateralExchangeRate> for Rate {
    fn from(exchange_rate: CollateralExchangeRate) -> Self {
        exchange_rate.0
    }
}

impl ReserveState {
    /// Add new borrow amount to total borrows
    pub fn add_borrow(&mut self, borrow_amount: u64) -> ProgramResult {
        if borrow_amount > self.available_liquidity {
            return Err(LendingError::InsufficientLiquidity.into());
        }

        self.available_liquidity -= borrow_amount;
        self.borrowed_liquidity_wads += Decimal::from(borrow_amount);
        Ok(())
    }

    /// Subtract repay amount from total borrows and return rounded repay value
    pub fn subtract_repay(&mut self, repay_amount: Decimal) -> Result<u64, ProgramError> {
        let rounded_repay_amount = repay_amount.round_u64();
        if rounded_repay_amount == 0 {
            return Err(LendingError::ObligationTooSmall.into());
        }

        self.available_liquidity += rounded_repay_amount;
        self.borrowed_liquidity_wads -= repay_amount;

        Ok(rounded_repay_amount)
    }

    /// Calculate the current utilization rate of the reserve
    pub fn current_utilization_rate(&self) -> Rate {
        let available_liquidity = Decimal::from(self.available_liquidity);
        let total_supply = self.borrowed_liquidity_wads + available_liquidity;

        let zero = Decimal::zero();
        if total_supply == zero {
            return Rate::zero();
        }

        (self.borrowed_liquidity_wads / total_supply).as_rate()
    }

    // TODO: is exchange rate fixed within a slot?
    /// Return the current collateral exchange rate.
    pub fn collateral_exchange_rate(&self) -> CollateralExchangeRate {
        if self.collateral_mint_supply == 0 {
            CollateralExchangeRate(Rate::from(INITIAL_COLLATERAL_RATE))
        } else {
            let collateral_supply = Decimal::from(self.collateral_mint_supply);
            let total_supply =
                self.borrowed_liquidity_wads + Decimal::from(self.available_liquidity);
            CollateralExchangeRate((collateral_supply / total_supply).as_rate())
        }
    }

    /// Return slots elapsed since last update
    fn update_slot(&mut self, slot: Slot) -> u64 {
        let slots_elapsed = slot - self.last_update_slot;
        self.last_update_slot = slot;
        slots_elapsed
    }

    fn apply_interest(&mut self, compounded_interest_rate: Rate) {
        self.borrowed_liquidity_wads *= compounded_interest_rate;
        self.cumulative_borrow_rate_wads *= compounded_interest_rate;
    }
}

impl Reserve {
    /// Calculate the current borrow rate
    pub fn current_borrow_rate(&self) -> Rate {
        let utilization_rate = self.state.current_utilization_rate();
        let optimal_utilization_rate = Rate::from_percent(self.config.optimal_utilization_rate);
        if self.config.optimal_utilization_rate == 100
            || utilization_rate < optimal_utilization_rate
        {
            let normalized_rate = utilization_rate / optimal_utilization_rate;
            normalized_rate
                * Rate::from_percent(self.config.optimal_borrow_rate - self.config.min_borrow_rate)
                + Rate::from_percent(self.config.min_borrow_rate)
        } else {
            let normalized_rate = (utilization_rate - optimal_utilization_rate)
                / Rate::from_percent(100 - self.config.optimal_utilization_rate);
            normalized_rate
                * Rate::from_percent(self.config.max_borrow_rate - self.config.optimal_borrow_rate)
                + Rate::from_percent(self.config.optimal_borrow_rate)
        }
    }

    /// Record deposited liquidity and return amount of collateral tokens to mint
    pub fn deposit_liquidity(&mut self, liquidity_amount: u64) -> u64 {
        let collateral_exchange_rate = self.state.collateral_exchange_rate();
        let collateral_amount = collateral_exchange_rate.liquidity_to_collateral(liquidity_amount);

        self.state.available_liquidity += liquidity_amount;
        self.state.collateral_mint_supply += collateral_amount;

        collateral_amount
    }

    /// Record redeemed collateral and return amount of liquidity to withdraw
    pub fn redeem_collateral(&mut self, collateral_amount: u64) -> Result<u64, ProgramError> {
        let collateral_exchange_rate = self.state.collateral_exchange_rate();
        let liquidity_amount = collateral_exchange_rate.collateral_to_liquidity(collateral_amount);
        if liquidity_amount > self.state.available_liquidity {
            return Err(LendingError::InsufficientLiquidity.into());
        }

        self.state.available_liquidity -= liquidity_amount;
        self.state.collateral_mint_supply -= collateral_amount;

        Ok(liquidity_amount)
    }

    /// Update borrow rate and accrue interest
    pub fn accrue_interest(&mut self, current_slot: Slot) {
        let slots_elapsed = self.state.update_slot(current_slot);
        if slots_elapsed > 0 {
            let borrow_rate = self.current_borrow_rate();
            let slot_interest_rate: Rate = borrow_rate / SLOTS_PER_YEAR;
            let compounded_interest_rate = (Rate::one() + slot_interest_rate).pow(slots_elapsed);
            self.state.apply_interest(compounded_interest_rate);
        }
    }
}

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
        let compounded_interest_rate: Rate =
            (cumulative_borrow_rate / self.cumulative_borrow_rate_wads).as_rate();

        if compounded_interest_rate < Rate::one() {
            return Err(LendingError::NegativeInterestRate.into());
        }

        self.borrowed_liquidity_wads *= compounded_interest_rate;
        self.cumulative_borrow_rate_wads = cumulative_borrow_rate;
        Ok(())
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
            lending_market: Pubkey::new_from_array(*lending_market),
            liquidity_mint: Pubkey::new_from_array(*liquidity_mint),
            liquidity_mint_decimals: u8::from_le_bytes(*liquidity_mint_decimals),
            liquidity_supply: Pubkey::new_from_array(*liquidity_supply),
            collateral_mint: Pubkey::new_from_array(*collateral_mint),
            collateral_supply: Pubkey::new_from_array(*collateral_supply),
            collateral_fees_receiver: Pubkey::new_from_array(*collateral_fees_receiver),
            dex_market: unpack_coption_key(dex_market)?,
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
            state: ReserveState {
                last_update_slot: u64::from_le_bytes(*last_update_slot),
                cumulative_borrow_rate_wads: unpack_decimal(cumulative_borrow_rate),
                borrowed_liquidity_wads: unpack_decimal(total_borrows),
                available_liquidity: u64::from_le_bytes(*available_liquidity),
                collateral_mint_supply: u64::from_le_bytes(*collateral_mint_supply),
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
        *last_update_slot = self.state.last_update_slot.to_le_bytes();
        lending_market.copy_from_slice(self.lending_market.as_ref());
        liquidity_mint.copy_from_slice(self.liquidity_mint.as_ref());
        *liquidity_mint_decimals = self.liquidity_mint_decimals.to_le_bytes();
        liquidity_supply.copy_from_slice(self.liquidity_supply.as_ref());
        collateral_mint.copy_from_slice(self.collateral_mint.as_ref());
        collateral_supply.copy_from_slice(self.collateral_supply.as_ref());
        collateral_fees_receiver.copy_from_slice(self.collateral_fees_receiver.as_ref());
        pack_coption_key(&self.dex_market, dex_market);
        *optimal_utilization_rate = self.config.optimal_utilization_rate.to_le_bytes();
        *loan_to_value_ratio = self.config.loan_to_value_ratio.to_le_bytes();
        *liquidation_bonus = self.config.liquidation_bonus.to_le_bytes();
        *liquidation_threshold = self.config.liquidation_threshold.to_le_bytes();
        *min_borrow_rate = self.config.min_borrow_rate.to_le_bytes();
        *optimal_borrow_rate = self.config.optimal_borrow_rate.to_le_bytes();
        *max_borrow_rate = self.config.max_borrow_rate.to_le_bytes();
        *borrow_fee_wad = self.config.fees.borrow_fee_wad.to_le_bytes();
        *host_fee_percentage = self.config.fees.host_fee_percentage.to_le_bytes();
        pack_decimal(
            self.state.cumulative_borrow_rate_wads,
            cumulative_borrow_rate,
        );
        pack_decimal(self.state.borrowed_liquidity_wads, total_borrows);
        *available_liquidity = self.state.available_liquidity.to_le_bytes();
        *collateral_mint_supply = self.state.collateral_mint_supply.to_le_bytes();
    }
}

impl Sealed for LendingMarket {}
impl IsInitialized for LendingMarket {
    fn is_initialized(&self) -> bool {
        self.version != UNINITIALIZED_VERSION
    }
}

const LENDING_MARKET_LEN: usize = 128;
impl Pack for LendingMarket {
    const LEN: usize = 128;

    /// Unpacks a byte buffer into a [LendingMarketInfo](struct.LendingMarketInfo.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, LENDING_MARKET_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (version, bump_seed, quote_token_mint, token_program_id, _padding) =
            array_refs![input, 1, 1, 32, 32, 62];
        let version = u8::from_le_bytes(*version);
        let bump_seed = u8::from_le_bytes(*bump_seed);
        match version {
            PROGRAM_VERSION | UNINITIALIZED_VERSION => Ok(Self {
                version,
                bump_seed,
                quote_token_mint: Pubkey::new_from_array(*quote_token_mint),
                token_program_id: Pubkey::new_from_array(*token_program_id),
            }),
            _ => Err(ProgramError::InvalidAccountData),
        }
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, LENDING_MARKET_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (version, bump_seed, quote_token_mint, token_program_id, _padding) =
            mut_array_refs![output, 1, 1, 32, 32, 62];
        *version = self.version.to_le_bytes();
        *bump_seed = self.bump_seed.to_le_bytes();
        quote_token_mint.copy_from_slice(self.quote_token_mint.as_ref());
        token_program_id.copy_from_slice(self.token_program_id.as_ref());
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

// Helpers
fn pack_coption_key(src: &COption<Pubkey>, dst: &mut [u8; 36]) {
    let (tag, body) = mut_array_refs![dst, 4, 32];
    match src {
        COption::Some(key) => {
            *tag = [1, 0, 0, 0];
            body.copy_from_slice(key.as_ref());
        }
        COption::None => {
            *tag = [0; 4];
        }
    }
}

fn unpack_coption_key(src: &[u8; 36]) -> Result<COption<Pubkey>, ProgramError> {
    let (tag, body) = array_refs![src, 4, 32];
    match *tag {
        [0, 0, 0, 0] => Ok(COption::None),
        [1, 0, 0, 0] => Ok(COption::Some(Pubkey::new_from_array(*body))),
        _ => Err(ProgramError::InvalidAccountData),
    }
}

fn pack_decimal(decimal: Decimal, dst: &mut [u8; 16]) {
    *dst = decimal.to_scaled_val().to_le_bytes();
}

fn unpack_decimal(src: &[u8; 16]) -> Decimal {
    Decimal::from_scaled_val(u128::from_le_bytes(*src))
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::math::WAD;
    use proptest::prelude::*;

    proptest! {
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
            let (total_fee, host_fee) = fees.calculate_borrow_fees(borrow_amount).unwrap();

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
