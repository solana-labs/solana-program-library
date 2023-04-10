// Contains fixtures values usable in tests, made to reduce boilerplate

use {
    anchor_lang::prelude::Pubkey,
    perpetuals::{
        instructions::InitParams,
        state::{
            custody::{BorrowRateParams, Fees, FeesMode, OracleParams, PricingParams},
            oracle::OracleType,
            perpetuals::Permissions,
        },
    },
};

pub fn permissions_full() -> Permissions {
    Permissions {
        allow_swap: true,
        allow_add_liquidity: true,
        allow_remove_liquidity: true,
        allow_open_position: true,
        allow_close_position: true,
        allow_pnl_withdrawal: true,
        allow_collateral_withdrawal: true,
        allow_size_change: true,
    }
}

pub fn borrow_rate_regular() -> BorrowRateParams {
    BorrowRateParams {
        base_rate: 0,
        slope1: 80_000,
        slope2: 120_000,
        optimal_utilization: 800_000_000,
    }
}

pub fn fees_linear_regular() -> Fees {
    Fees {
        mode: FeesMode::Linear,
        ratio_mult: 20_000,
        utilization_mult: 20_000,
        swap_in: 100,
        swap_out: 100,
        stable_swap_in: 100,
        stable_swap_out: 100,
        add_liquidity: 200,
        remove_liquidity: 300,
        open_position: 100,
        close_position: 100,
        liquidation: 50,
        protocol_share: 25,
    }
}

pub fn pricing_params_regular(use_ema: bool) -> PricingParams {
    PricingParams {
        use_ema,
        use_unrealized_pnl_in_aum: true,
        trade_spread_long: 100,
        trade_spread_short: 100,
        swap_spread: 300,
        min_initial_leverage: 10_000,
        max_initial_leverage: 100_000,
        max_leverage: 100_000,
        max_payoff_mult: 10_000,
        max_utilization: 0,
        max_position_locked_usd: 0,
        max_total_locked_usd: 0,
    }
}

pub fn oracle_params_regular(oracle_account: Pubkey) -> OracleParams {
    OracleParams {
        oracle_account,
        oracle_type: OracleType::Test,
        max_price_error: 1_000_000,
        max_price_age_sec: 30,
    }
}

pub fn init_params_permissions_full(min_signatures: u8) -> InitParams {
    InitParams {
        min_signatures,
        allow_swap: true,
        allow_add_liquidity: true,
        allow_remove_liquidity: true,
        allow_open_position: true,
        allow_close_position: true,
        allow_pnl_withdrawal: true,
        allow_collateral_withdrawal: true,
        allow_size_change: true,
    }
}
