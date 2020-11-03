#![deny(missing_docs)]

//! A lending program for the Solana blockchain.

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("TokenLend1ng1111111111111111111111111111111");
