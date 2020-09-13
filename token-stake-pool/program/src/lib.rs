#![deny(missing_docs)]

//! An governance program for the Solana spl-token program.

pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

// Export current solana-sdk types for downstream users who may also be building with a different
// solana-sdk version
pub use solana_sdk;

solana_sdk::declare_id!("TokenGov1111111111111111111111111111111111");
