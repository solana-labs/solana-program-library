#![deny(missing_docs)]

//! An ERC20-like Token program for the Solana blockchain

pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod native_mint;
pub mod option;
pub mod processor;
pub mod state;

// Export current solana-sdk types for downstream users who may also be building with a different
// solana-sdk version
pub use solana_sdk;

/// Convert the UI representation of a token amount (using the decimals field defined in its mint)
/// to the raw amount
pub fn ui_amount_to_amount(ui_amount: f64, decimals: u8) -> u64 {
    (ui_amount * 10_usize.pow(decimals as u32) as f64) as u64
}

/// Convert a raw amount to its UI representation (using the decimals field defined in its mint)
pub fn amount_to_ui_amount(amount: u64, decimals: u8) -> f64 {
    amount as f64 / 10_usize.pow(decimals as u32) as f64
}

solana_sdk::declare_id!("TokenSVp5gheXUvJ6jGWGeCsgPKgnE3YgdGKRVCMY9o");
