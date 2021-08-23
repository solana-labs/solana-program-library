#![deny(missing_docs)]

//! A lending program for the Solana blockchain.

pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod math;
pub mod processor;
pub mod pyth;
pub mod state;
use std::str::FromStr;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("So1endDq2YkqhipRh3WViPa8hdiSpxWy6z3Z6tMCpAo");

/// null pubkey
pub fn null_pubkey() -> solana_program::pubkey::Pubkey {
    solana_program::pubkey::Pubkey::from_str("nu11111111111111111111111111111111111111111").unwrap()
}
