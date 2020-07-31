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

solana_sdk::declare_id!("TokenSVp5gheXUvJ6jGWGeCsgPKgnE3YgdGKRVCMY9o");
