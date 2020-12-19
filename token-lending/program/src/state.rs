//! State types

use crate::{
    error::LendingError,
    math::{Decimal, Rate},
};
use arrayref::{array_mut_ref, array_ref, array_refs, mut_array_refs};
use solana_program::{
    clock::{Slot, DEFAULT_TICKS_PER_SECOND, DEFAULT_TICKS_PER_SLOT, SECONDS_PER_DAY},
    entrypoint::ProgramResult,
    program_error::ProgramError,
    program_option::COption,
    program_pack::{IsInitialized, Pack, Sealed},
    pubkey::Pubkey,
    sysvar::clock::Clock,
};

/// Collateral tokens are initially valued at a ratio of 5:1 (collateral:liquidity)
pub const INITIAL_COLLATERAL_RATE: u64 = 5;

/// Number of slots per year
pub const SLOTS_PER_YEAR: u64 =
    DEFAULT_TICKS_PER_SECOND / DEFAULT_TICKS_PER_SLOT * SECONDS_PER_DAY * 365;

/// Lending market state
#[derive(Clone, Debug, Default, PartialEq)]
pub struct LendingMarket {
    /// Initialized state
    pub is_initialized: bool,
    /// Quote currency token mint
    pub quote_token_mint: Pubkey,
    /// Token program id
    pub token_program_id: Pubkey,
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

    /// Subtract repay amount from total borrows
    pub fn subtract_repay(&mut self, repay_amount: Decimal) {
        self.available_liquidity += repay_amount.round_u64();
        self.borrowed_liquidity_wads -= repay_amount;
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
    /// Slot when obligation was updated. Used for calculating interest.
    pub last_update_slot: u64,
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
    pub fn accrue_interest(&mut self, clock: &Clock, cumulative_borrow_rate: Decimal) {
        let slots_elapsed = clock.slot - self.last_update_slot;
        let borrow_rate =
            (cumulative_borrow_rate / self.cumulative_borrow_rate_wads - Decimal::one()).as_rate();
        let yearly_interest: Decimal = self.borrowed_liquidity_wads * borrow_rate;
        let accrued_interest: Decimal = yearly_interest * slots_elapsed / SLOTS_PER_YEAR;

        self.borrowed_liquidity_wads += accrued_interest;
        self.cumulative_borrow_rate_wads = cumulative_borrow_rate;
        self.last_update_slot = clock.slot;
    }
}

impl Sealed for Reserve {}
impl IsInitialized for Reserve {
    fn is_initialized(&self) -> bool {
        self.state.last_update_slot > 0
    }
}

const RESERVE_LEN: usize = 260;
impl Pack for Reserve {
    const LEN: usize = 260;

    /// Unpacks a byte buffer into a [ReserveInfo](struct.ReserveInfo.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, RESERVE_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            last_update_slot,
            lending_market,
            liquidity_mint,
            liquidity_mint_decimals,
            liquidity_supply,
            collateral_mint,
            collateral_supply,
            dex_market,
            optimal_utilization_rate,
            loan_to_value_ratio,
            liquidation_bonus,
            liquidation_threshold,
            min_borrow_rate,
            optimal_borrow_rate,
            max_borrow_rate,
            cumulative_borrow_rate,
            total_borrows,
            available_liquidity,
            collateral_mint_supply,
        ) = array_refs![input, 8, 32, 32, 1, 32, 32, 32, 36, 1, 1, 1, 1, 1, 1, 1, 16, 16, 8, 8];
        Ok(Self {
            lending_market: Pubkey::new_from_array(*lending_market),
            liquidity_mint: Pubkey::new_from_array(*liquidity_mint),
            liquidity_mint_decimals: u8::from_le_bytes(*liquidity_mint_decimals),
            liquidity_supply: Pubkey::new_from_array(*liquidity_supply),
            collateral_mint: Pubkey::new_from_array(*collateral_mint),
            collateral_supply: Pubkey::new_from_array(*collateral_supply),
            dex_market: unpack_coption_key(dex_market)?,
            config: ReserveConfig {
                optimal_utilization_rate: u8::from_le_bytes(*optimal_utilization_rate),
                loan_to_value_ratio: u8::from_le_bytes(*loan_to_value_ratio),
                liquidation_bonus: u8::from_le_bytes(*liquidation_bonus),
                liquidation_threshold: u8::from_le_bytes(*liquidation_threshold),
                min_borrow_rate: u8::from_le_bytes(*min_borrow_rate),
                optimal_borrow_rate: u8::from_le_bytes(*optimal_borrow_rate),
                max_borrow_rate: u8::from_le_bytes(*max_borrow_rate),
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
            last_update_slot,
            lending_market,
            liquidity_mint,
            liquidity_mint_decimals,
            liquidity_supply,
            collateral_mint,
            collateral_supply,
            dex_market,
            optimal_utilization_rate,
            loan_to_value_ratio,
            liquidation_bonus,
            liquidation_threshold,
            min_borrow_rate,
            optimal_borrow_rate,
            max_borrow_rate,
            cumulative_borrow_rate,
            total_borrows,
            available_liquidity,
            collateral_mint_supply,
        ) = mut_array_refs![
            output, 8, 32, 32, 1, 32, 32, 32, 36, 1, 1, 1, 1, 1, 1, 1, 16, 16, 8, 8
        ];
        *last_update_slot = self.state.last_update_slot.to_le_bytes();
        lending_market.copy_from_slice(self.lending_market.as_ref());
        liquidity_mint.copy_from_slice(self.liquidity_mint.as_ref());
        *liquidity_mint_decimals = self.liquidity_mint_decimals.to_le_bytes();
        liquidity_supply.copy_from_slice(self.liquidity_supply.as_ref());
        collateral_mint.copy_from_slice(self.collateral_mint.as_ref());
        collateral_supply.copy_from_slice(self.collateral_supply.as_ref());
        pack_coption_key(&self.dex_market, dex_market);
        *optimal_utilization_rate = self.config.optimal_utilization_rate.to_le_bytes();
        *loan_to_value_ratio = self.config.loan_to_value_ratio.to_le_bytes();
        *liquidation_bonus = self.config.liquidation_bonus.to_le_bytes();
        *liquidation_threshold = self.config.liquidation_threshold.to_le_bytes();
        *min_borrow_rate = self.config.min_borrow_rate.to_le_bytes();
        *optimal_borrow_rate = self.config.optimal_borrow_rate.to_le_bytes();
        *max_borrow_rate = self.config.max_borrow_rate.to_le_bytes();
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
        self.is_initialized
    }
}

const LENDING_MARKET_LEN: usize = 65;
impl Pack for LendingMarket {
    const LEN: usize = 65;

    /// Unpacks a byte buffer into a [LendingMarketInfo](struct.LendingMarketInfo.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, LENDING_MARKET_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (is_initialized, quote_token_mint, token_program_id) = array_refs![input, 1, 32, 32];
        Ok(Self {
            is_initialized: match is_initialized {
                [0] => false,
                [1] => true,
                _ => return Err(ProgramError::InvalidAccountData),
            },
            quote_token_mint: Pubkey::new_from_array(*quote_token_mint),
            token_program_id: Pubkey::new_from_array(*token_program_id),
        })
    }

    fn pack_into_slice(&self, output: &mut [u8]) {
        let output = array_mut_ref![output, 0, LENDING_MARKET_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (is_initialized, quote_token_mint, token_program_id) =
            mut_array_refs![output, 1, 32, 32];
        *is_initialized = [self.is_initialized as u8];
        quote_token_mint.copy_from_slice(self.quote_token_mint.as_ref());
        token_program_id.copy_from_slice(self.token_program_id.as_ref());
    }
}

impl Sealed for Obligation {}
impl IsInitialized for Obligation {
    fn is_initialized(&self) -> bool {
        self.last_update_slot > 0
    }
}

const OBLIGATION_LEN: usize = 144;
impl Pack for Obligation {
    const LEN: usize = 144;

    /// Unpacks a byte buffer into a [ObligationInfo](struct.ObligationInfo.html).
    fn unpack_from_slice(input: &[u8]) -> Result<Self, ProgramError> {
        let input = array_ref![input, 0, OBLIGATION_LEN];
        #[allow(clippy::ptr_offset_with_cast)]
        let (
            last_update_slot,
            deposited_collateral_tokens,
            collateral_supply,
            cumulative_borrow_rate,
            borrowed_liquidity_wads,
            borrow_reserve,
            token_mint,
        ) = array_refs![input, 8, 8, 32, 16, 16, 32, 32];
        Ok(Self {
            last_update_slot: u64::from_le_bytes(*last_update_slot),
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
            last_update_slot,
            deposited_collateral_tokens,
            collateral_supply,
            cumulative_borrow_rate,
            borrowed_liquidity_wads,
            borrow_reserve,
            token_mint,
        ) = mut_array_refs![output, 8, 8, 32, 16, 16, 32, 32];

        *last_update_slot = self.last_update_slot.to_le_bytes();
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
