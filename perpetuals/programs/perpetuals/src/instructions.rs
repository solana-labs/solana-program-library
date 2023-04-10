// admin instructions
pub mod add_custody;
pub mod add_pool;
pub mod init;
pub mod remove_custody;
pub mod remove_pool;
pub mod set_admin_signers;
pub mod set_custody_config;
pub mod set_permissions;
pub mod upgrade_custody;
pub mod withdraw_fees;
pub mod withdraw_sol_fees;

// test instructions
pub mod set_test_oracle_price;
pub mod set_test_time;
pub mod test_init;

// public instructions
pub mod add_collateral;
pub mod add_liquidity;
pub mod close_position;
pub mod get_add_liquidity_amount_and_fee;
pub mod get_assets_under_management;
pub mod get_entry_price_and_fee;
pub mod get_exit_price_and_fee;
pub mod get_liquidation_price;
pub mod get_liquidation_state;
pub mod get_oracle_price;
pub mod get_pnl;
pub mod get_remove_liquidity_amount_and_fee;
pub mod get_swap_amount_and_fees;
pub mod liquidate;
pub mod open_position;
pub mod remove_collateral;
pub mod remove_liquidity;
pub mod swap;

// bring everything in scope
pub use {
    add_collateral::*, add_custody::*, add_liquidity::*, add_pool::*, close_position::*,
    get_add_liquidity_amount_and_fee::*, get_assets_under_management::*,
    get_entry_price_and_fee::*, get_exit_price_and_fee::*, get_liquidation_price::*,
    get_liquidation_state::*, get_oracle_price::*, get_pnl::*,
    get_remove_liquidity_amount_and_fee::*, get_swap_amount_and_fees::*, init::*, liquidate::*,
    open_position::*, remove_collateral::*, remove_custody::*, remove_liquidity::*, remove_pool::*,
    set_admin_signers::*, set_custody_config::*, set_permissions::*, set_test_oracle_price::*,
    set_test_time::*, swap::*, test_init::*, upgrade_custody::*, withdraw_fees::*,
    withdraw_sol_fees::*,
};
