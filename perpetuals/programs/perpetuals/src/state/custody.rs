use {
    crate::{
        error::PerpetualsError,
        math,
        state::{
            oracle::{OraclePrice, OracleType},
            perpetuals::{Permissions, Perpetuals},
            position::{Position, Side},
        },
    },
    anchor_lang::prelude::*,
};

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Debug)]
pub enum FeesMode {
    Fixed,
    Linear,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct Fees {
    pub mode: FeesMode,
    // fees have implied BPS_DECIMALS decimals
    pub ratio_mult: u64,
    pub utilization_mult: u64,
    pub swap_in: u64,
    pub swap_out: u64,
    pub stable_swap_in: u64,
    pub stable_swap_out: u64,
    pub add_liquidity: u64,
    pub remove_liquidity: u64,
    pub open_position: u64,
    pub close_position: u64,
    pub liquidation: u64,
    pub protocol_share: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct FeesStats {
    pub swap_usd: u64,
    pub add_liquidity_usd: u64,
    pub remove_liquidity_usd: u64,
    pub open_position_usd: u64,
    pub close_position_usd: u64,
    pub liquidation_usd: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct VolumeStats {
    pub swap_usd: u64,
    pub add_liquidity_usd: u64,
    pub remove_liquidity_usd: u64,
    pub open_position_usd: u64,
    pub close_position_usd: u64,
    pub liquidation_usd: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct TradeStats {
    pub profit_usd: u64,
    pub loss_usd: u64,
    // open interest
    pub oi_long_usd: u64,
    pub oi_short_usd: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct Assets {
    // collateral debt
    pub collateral: u64,
    // protocol_fees are part of the collected fees that is reserved for the protocol
    pub protocol_fees: u64,
    // owned = total_assets - collateral + collected_fees - protocol_fees
    pub owned: u64,
    // locked funds for pnl payoff
    pub locked: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct OracleParams {
    pub oracle_account: Pubkey,
    pub oracle_type: OracleType,
    pub max_price_error: u64,
    pub max_price_age_sec: u32,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct PricingParams {
    pub use_ema: bool,
    // whether to account for unrealized pnl in assets under management calculations
    pub use_unrealized_pnl_in_aum: bool,
    // pricing params have implied BPS_DECIMALS decimals (except ended with _usd)
    pub trade_spread_long: u64,
    pub trade_spread_short: u64,
    pub swap_spread: u64,
    pub min_initial_leverage: u64,
    pub max_initial_leverage: u64,
    pub max_leverage: u64,
    // max_user_profit = position_size * max_payoff_mult
    pub max_payoff_mult: u64,
    pub max_utilization: u64,
    // USD denominated values always have implied USD_DECIMALS decimals
    pub max_position_locked_usd: u64,
    pub max_total_locked_usd: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct BorrowRateParams {
    // borrow rate params have implied RATE_DECIMALS decimals
    pub base_rate: u64,
    pub slope1: u64,
    pub slope2: u64,
    pub optimal_utilization: u64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct BorrowRateState {
    // borrow rates have implied RATE_DECIMALS decimals
    pub current_rate: u64,
    pub cumulative_interest: u128,
    pub last_update: i64,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct PositionStats {
    pub open_positions: u64,
    pub collateral_usd: u64,
    pub size_usd: u64,
    pub locked_amount: u64,
    pub weighted_price: u128,
    pub total_quantity: u128,
    pub cumulative_interest_usd: u64,
    pub cumulative_interest_snapshot: u128,
}

#[account]
#[derive(Default, Debug)]
pub struct Custody {
    // static parameters
    pub pool: Pubkey,
    pub mint: Pubkey,
    pub token_account: Pubkey,
    pub decimals: u8,
    pub is_stable: bool,
    pub oracle: OracleParams,
    pub pricing: PricingParams,
    pub permissions: Permissions,
    pub fees: Fees,
    pub borrow_rate: BorrowRateParams,

    // dynamic variables
    pub assets: Assets,
    pub collected_fees: FeesStats,
    pub volume_stats: VolumeStats,
    pub trade_stats: TradeStats,
    pub long_positions: PositionStats,
    pub short_positions: PositionStats,
    pub borrow_rate_state: BorrowRateState,

    // bumps for address validation
    pub bump: u8,
    pub token_account_bump: u8,
}

#[derive(Copy, Clone, PartialEq, AnchorSerialize, AnchorDeserialize, Default, Debug)]
pub struct DeprecatedPricingParams {
    pub use_ema: bool,
    // whether to account for unrealized pnl in assets under management calculations
    pub use_unrealized_pnl_in_aum: bool,
    // pricing params have implied BPS_DECIMALS decimals
    pub trade_spread_long: u64,
    pub trade_spread_short: u64,
    pub swap_spread: u64,
    pub min_initial_leverage: u64,
    pub max_leverage: u64,
    // max_user_profit = position_size * max_payoff_mult
    pub max_payoff_mult: u64,
}

#[account]
#[derive(Default, Debug)]
pub struct DeprecatedCustody {
    // static parameters
    pub pool: Pubkey,
    pub mint: Pubkey,
    pub token_account: Pubkey,
    pub decimals: u8,
    pub is_stable: bool,
    pub oracle: OracleParams,
    pub pricing: DeprecatedPricingParams,
    pub permissions: Permissions,
    pub fees: Fees,
    pub borrow_rate: BorrowRateParams,

    // dynamic variables
    pub assets: Assets,
    pub collected_fees: FeesStats,
    pub volume_stats: VolumeStats,
    pub trade_stats: TradeStats,
    pub long_positions: PositionStats,
    pub short_positions: PositionStats,
    pub borrow_rate_state: BorrowRateState,

    // bumps for address validation
    pub bump: u8,
    pub token_account_bump: u8,
}

impl Default for FeesMode {
    fn default() -> Self {
        Self::Linear
    }
}

impl Fees {
    pub fn validate(&self) -> bool {
        self.swap_in as u128 <= Perpetuals::BPS_POWER
            && self.swap_out as u128 <= Perpetuals::BPS_POWER
            && self.stable_swap_in as u128 <= Perpetuals::BPS_POWER
            && self.stable_swap_out as u128 <= Perpetuals::BPS_POWER
            && self.add_liquidity as u128 <= Perpetuals::BPS_POWER
            && self.remove_liquidity as u128 <= Perpetuals::BPS_POWER
            && self.open_position as u128 <= Perpetuals::BPS_POWER
            && self.close_position as u128 <= Perpetuals::BPS_POWER
            && self.liquidation as u128 <= Perpetuals::BPS_POWER
            && self.protocol_share as u128 <= Perpetuals::BPS_POWER
    }
}

impl OracleParams {
    pub fn validate(&self) -> bool {
        self.oracle_type == OracleType::None || self.oracle_account != Pubkey::default()
    }
}

impl PricingParams {
    pub fn validate(&self) -> bool {
        (self.min_initial_leverage as u128) >= Perpetuals::BPS_POWER
            && self.min_initial_leverage <= self.max_initial_leverage
            && self.max_initial_leverage <= self.max_leverage
            && (self.trade_spread_long as u128) < Perpetuals::BPS_POWER
            && (self.trade_spread_short as u128) < Perpetuals::BPS_POWER
            && (self.swap_spread as u128) < Perpetuals::BPS_POWER
            && (self.max_utilization as u128) <= Perpetuals::BPS_POWER
            && self.max_position_locked_usd <= self.max_total_locked_usd
    }
}

impl BorrowRateParams {
    pub fn validate(&self) -> bool {
        self.optimal_utilization > 0 && (self.optimal_utilization as u128) <= Perpetuals::RATE_POWER
    }
}

impl Custody {
    pub const LEN: usize = 8 + std::mem::size_of::<Custody>();

    pub fn validate(&self) -> bool {
        self.token_account != Pubkey::default()
            && self.mint != Pubkey::default()
            && self.oracle.validate()
            && self.pricing.validate()
            && self.fees.validate()
            && self.borrow_rate.validate()
    }

    pub fn lock_funds(&mut self, amount: u64) -> Result<()> {
        self.assets.locked = math::checked_add(self.assets.locked, amount)?;

        // check for max utilization
        if self.pricing.max_utilization > 0
            && (self.pricing.max_utilization as u128) < Perpetuals::BPS_POWER
            && self.assets.owned > 0
        {
            let current_utilization = math::checked_as_u64(math::checked_div(
                math::checked_mul(self.assets.locked as u128, Perpetuals::BPS_POWER)?,
                self.assets.owned as u128,
            )?)?;
            require!(
                current_utilization <= self.pricing.max_utilization,
                PerpetualsError::MaxUtilization
            );
        }

        if self.assets.owned < self.assets.locked {
            Err(ProgramError::InsufficientFunds.into())
        } else {
            Ok(())
        }
    }

    pub fn unlock_funds(&mut self, amount: u64) -> Result<()> {
        if amount > self.assets.locked {
            self.assets.locked = 0;
        } else {
            self.assets.locked = math::checked_sub(self.assets.locked, amount)?;
        }

        Ok(())
    }

    pub fn get_interest_amount_usd(&self, position: &Position, curtime: i64) -> Result<u64> {
        if position.size_usd == 0 {
            return Ok(0);
        }

        let cumulative_interest = self.get_cumulative_interest(curtime)?;

        let position_interest = if cumulative_interest > position.cumulative_interest_snapshot {
            math::checked_sub(cumulative_interest, position.cumulative_interest_snapshot)?
        } else {
            return Ok(0);
        };

        math::checked_as_u64(math::checked_div(
            math::checked_mul(position_interest, position.size_usd as u128)?,
            Perpetuals::RATE_POWER,
        )?)
    }

    pub fn get_cumulative_interest(&self, curtime: i64) -> Result<u128> {
        if curtime > self.borrow_rate_state.last_update {
            let cumulative_interest = math::checked_ceil_div(
                math::checked_mul(
                    math::checked_sub(curtime, self.borrow_rate_state.last_update)? as u128,
                    self.borrow_rate_state.current_rate as u128,
                )?,
                3600,
            )?;
            math::checked_add(
                self.borrow_rate_state.cumulative_interest,
                cumulative_interest,
            )
        } else {
            Ok(self.borrow_rate_state.cumulative_interest)
        }
    }

    pub fn update_borrow_rate(&mut self, curtime: i64) -> Result<()> {
        // if current_utilization < optimal_utilization:
        //   rate = base_rate + (current_utilization / optimal_utilization) * slope1
        // else:
        //   rate = base_rate + slope1 + (current_utilization - optimal_utilization) / (1 - optimal_utilization) * slope2

        if self.assets.owned == 0 {
            self.borrow_rate_state.current_rate = 0;
            self.borrow_rate_state.last_update =
                std::cmp::max(curtime, self.borrow_rate_state.last_update);
            return Ok(());
        }

        if curtime > self.borrow_rate_state.last_update {
            // compute interest accumulated since previous update
            self.borrow_rate_state.cumulative_interest = self.get_cumulative_interest(curtime)?;
            self.borrow_rate_state.last_update = curtime;
        }

        // get current utilization
        let current_utilization = math::checked_div(
            math::checked_mul(self.assets.locked as u128, Perpetuals::RATE_POWER)?,
            self.assets.owned as u128,
        )?;

        // compute and save new borrow rate
        let hourly_rate = if current_utilization < (self.borrow_rate.optimal_utilization as u128)
            || (self.borrow_rate.optimal_utilization as u128) >= Perpetuals::RATE_POWER
        {
            math::checked_div(
                math::checked_mul(current_utilization, self.borrow_rate.slope1 as u128)?,
                self.borrow_rate.optimal_utilization as u128,
            )?
        } else {
            math::checked_add(
                self.borrow_rate.slope1 as u128,
                math::checked_div(
                    math::checked_mul(
                        math::checked_sub(
                            current_utilization,
                            self.borrow_rate.optimal_utilization as u128,
                        )?,
                        self.borrow_rate.slope2 as u128,
                    )?,
                    Perpetuals::RATE_POWER - self.borrow_rate.optimal_utilization as u128,
                )?,
            )?
        };
        let hourly_rate = math::checked_add(
            math::checked_as_u64(hourly_rate)?,
            self.borrow_rate.base_rate,
        )?;

        self.borrow_rate_state.current_rate = hourly_rate;

        Ok(())
    }

    pub fn get_collective_position(&self, side: Side) -> Result<Position> {
        let stats = if side == Side::Long {
            &self.long_positions
        } else {
            &self.short_positions
        };
        if stats.open_positions > 0 {
            Ok(Position {
                side,
                price: math::checked_as_u64(math::checked_div(
                    stats.weighted_price,
                    stats.total_quantity,
                )?)?,
                size_usd: stats.size_usd,
                collateral_usd: stats.collateral_usd,
                unrealized_loss_usd: stats.cumulative_interest_usd,
                cumulative_interest_snapshot: stats.cumulative_interest_snapshot,
                locked_amount: stats.locked_amount,
                ..Position::default()
            })
        } else {
            Ok(Position::default())
        }
    }

    pub fn add_position(
        &mut self,
        position: &Position,
        token_price: &OraclePrice,
        curtime: i64,
    ) -> Result<()> {
        // compute accumulated interest
        let collective_position = self.get_collective_position(position.side)?;
        let interest_usd = self.get_interest_amount_usd(&collective_position, curtime)?;

        // update positions
        let stats = if position.side == Side::Long {
            &mut self.long_positions
        } else {
            &mut self.short_positions
        };

        stats.cumulative_interest_usd =
            math::checked_add(stats.cumulative_interest_usd, interest_usd)?;
        stats.cumulative_interest_snapshot = position.cumulative_interest_snapshot;

        stats.open_positions = math::checked_add(stats.open_positions, 1)?;
        stats.collateral_usd = math::checked_add(stats.collateral_usd, position.collateral_usd)?;
        stats.size_usd = math::checked_add(stats.size_usd, position.size_usd)?;
        stats.locked_amount = math::checked_add(stats.locked_amount, position.locked_amount)?;

        let position_price = math::scale_to_exponent(
            position.price,
            -(Perpetuals::PRICE_DECIMALS as i32),
            -(Perpetuals::USD_DECIMALS as i32),
        )?;
        let quantity = math::checked_div(
            math::checked_mul(position.size_usd as u128, Perpetuals::BPS_POWER)?,
            position_price as u128,
        )?;
        stats.weighted_price = math::checked_add(
            stats.weighted_price,
            math::checked_mul(position.price as u128, quantity)?,
        )?;
        stats.total_quantity = math::checked_add(stats.total_quantity, quantity)?;

        // check limits
        if self.pricing.max_position_locked_usd > 0 {
            let locked_amount_usd =
                token_price.get_asset_amount_usd(position.locked_amount, self.decimals)?;
            require!(
                locked_amount_usd <= self.pricing.max_position_locked_usd,
                PerpetualsError::PositionAmountLimit
            );
        }
        if self.pricing.max_total_locked_usd > 0 {
            let locked_amount_usd =
                token_price.get_asset_amount_usd(stats.locked_amount, self.decimals)?;
            require!(
                locked_amount_usd <= self.pricing.max_total_locked_usd,
                PerpetualsError::CustodyAmountLimit
            );
        }

        Ok(())
    }

    pub fn remove_position(&mut self, position: &Position, curtime: i64) -> Result<()> {
        // compute accumulated interest
        let collective_position = self.get_collective_position(position.side)?;
        let interest_usd = self.get_interest_amount_usd(&collective_position, curtime)?;
        let cumulative_interest_snapshot = self.get_cumulative_interest(curtime)?;
        let position_interest_usd = self.get_interest_amount_usd(position, curtime)?;

        // update stats
        let stats = if position.side == Side::Long {
            &mut self.long_positions
        } else {
            &mut self.short_positions
        };

        if stats.open_positions == 1 {
            *stats = PositionStats::default();
            return Ok(());
        }

        stats.cumulative_interest_usd =
            math::checked_add(stats.cumulative_interest_usd, interest_usd)?;
        stats.cumulative_interest_usd = stats
            .cumulative_interest_usd
            .saturating_sub(position_interest_usd);
        stats.cumulative_interest_snapshot = cumulative_interest_snapshot;

        stats.open_positions = math::checked_sub(stats.open_positions, 1)?;
        stats.collateral_usd = math::checked_sub(stats.collateral_usd, position.collateral_usd)?;
        stats.size_usd = math::checked_sub(stats.size_usd, position.size_usd)?;
        stats.locked_amount = math::checked_sub(stats.locked_amount, position.locked_amount)?;

        let position_price = math::scale_to_exponent(
            position.price,
            -(Perpetuals::PRICE_DECIMALS as i32),
            -(Perpetuals::USD_DECIMALS as i32),
        )?;
        let quantity = math::checked_div(
            math::checked_mul(position.size_usd as u128, Perpetuals::BPS_POWER)?,
            position_price as u128,
        )?;
        stats.weighted_price = math::checked_sub(
            stats.weighted_price,
            math::checked_mul(position.price as u128, quantity)?,
        )?;
        stats.total_quantity = math::checked_sub(stats.total_quantity, quantity)?;

        Ok(())
    }

    pub fn add_collateral(&mut self, side: Side, collateral_usd: u64) -> Result<()> {
        let stats = if side == Side::Long {
            &mut self.long_positions
        } else {
            &mut self.short_positions
        };

        stats.collateral_usd = math::checked_add(stats.collateral_usd, collateral_usd)?;

        Ok(())
    }

    pub fn remove_collateral(&mut self, side: Side, collateral_usd: u64) -> Result<()> {
        let stats = if side == Side::Long {
            &mut self.long_positions
        } else {
            &mut self.short_positions
        };

        stats.collateral_usd = math::checked_sub(stats.collateral_usd, collateral_usd)?;

        Ok(())
    }
}

impl DeprecatedCustody {
    pub const LEN: usize = 8 + std::mem::size_of::<DeprecatedCustody>();
}

#[cfg(test)]
mod test {
    use super::*;

    fn get_fixture() -> Custody {
        let assets = Assets {
            owned: 1000,
            locked: 500,
            ..Assets::default()
        };

        let borrow_rate = BorrowRateParams {
            base_rate: 0,
            slope1: 80000,
            slope2: 120000,
            optimal_utilization: 800000000,
        };

        Custody {
            decimals: 5,
            assets,
            borrow_rate,
            ..Custody::default()
        }
    }

    #[test]
    fn test_update_borrow_rate() {
        let mut custody = get_fixture();
        custody.update_borrow_rate(3600).unwrap();
        assert_eq!(
            custody.borrow_rate_state,
            BorrowRateState {
                current_rate: 50000,
                cumulative_interest: 0,
                last_update: 3600
            }
        );
        custody.update_borrow_rate(5400).unwrap();
        assert_eq!(
            custody.borrow_rate_state,
            BorrowRateState {
                current_rate: 50000,
                cumulative_interest: 25000,
                last_update: 5400
            }
        );
        custody.update_borrow_rate(7200).unwrap();
        assert_eq!(
            custody.borrow_rate_state,
            BorrowRateState {
                current_rate: 50000,
                cumulative_interest: 50000,
                last_update: 7200
            }
        );

        let mut custody = get_fixture();
        custody.update_borrow_rate(3600).unwrap();
        custody.update_borrow_rate(7200).unwrap();
        assert_eq!(
            custody.borrow_rate_state,
            BorrowRateState {
                current_rate: 50000,
                cumulative_interest: 50000,
                last_update: 7200
            }
        );

        let mut custody = get_fixture();
        custody.assets.locked = 0;
        custody.update_borrow_rate(3600).unwrap();
        assert_eq!(
            custody.borrow_rate_state,
            BorrowRateState {
                current_rate: 0,
                cumulative_interest: 0,
                last_update: 3600
            }
        );

        let mut custody = get_fixture();
        custody.assets.locked = 800;
        custody.update_borrow_rate(3600).unwrap();
        assert_eq!(custody.borrow_rate_state.current_rate, 80000);

        let mut custody = get_fixture();
        custody.assets.locked = 900;
        custody.update_borrow_rate(3600).unwrap();
        assert_eq!(custody.borrow_rate_state.current_rate, 140000);

        custody.update_borrow_rate(5400).unwrap();
        assert_eq!(custody.borrow_rate_state.cumulative_interest, 70000);

        custody.update_borrow_rate(7200).unwrap();
        assert_eq!(custody.borrow_rate_state.cumulative_interest, 140000);

        custody.assets.locked = 500;
        custody.update_borrow_rate(10800).unwrap();
        assert_eq!(custody.borrow_rate_state.current_rate, 50000);
        assert_eq!(custody.borrow_rate_state.cumulative_interest, 280000);
        custody.update_borrow_rate(14400).unwrap();
        assert_eq!(custody.borrow_rate_state.current_rate, 50000);
        assert_eq!(custody.borrow_rate_state.cumulative_interest, 330000);

        let mut custody = get_fixture();
        custody.assets.locked = 1000;
        custody.update_borrow_rate(3600).unwrap();
        assert_eq!(custody.borrow_rate_state.current_rate, 200000);

        let mut custody = get_fixture();
        custody.assets.locked = 1;
        custody.update_borrow_rate(3600).unwrap();
        assert_eq!(custody.borrow_rate_state.current_rate, 100);

        let mut custody = get_fixture();
        custody.assets.locked = 999;
        custody.update_borrow_rate(3600).unwrap();
        assert_eq!(custody.borrow_rate_state.current_rate, 199400);
    }
}
