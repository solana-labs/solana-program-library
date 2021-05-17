#![allow(missing_docs)]

//! An Uniswap-like program for the Solana blockchain.

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod swap;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("MokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
