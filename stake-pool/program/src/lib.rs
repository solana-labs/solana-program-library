#![deny(missing_docs)]

//! A program for creating pools of Solana stakes managed by a Stake-o-Matic

pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod stake;
pub mod state;

// Export current solana-sdk types for downstream users who may also be building with a different
// solana-sdk version
pub use solana_sdk;

solana_sdk::declare_id!("STAKEPQQL1111111111111111111111111111111111");
