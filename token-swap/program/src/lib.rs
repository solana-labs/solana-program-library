#![deny(missing_docs)]

//! An Uniswap-like program for the Solana blockchain.

pub mod curve;
pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

// Export current solana-sdk types for downstream users who may also be building with a different
// solana-sdk version
pub use solana_sdk;

solana_sdk::declare_id!("TokenSwap1111111111111111111111111111111111");
