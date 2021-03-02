#![deny(missing_docs)]

//! A program for creating pools of Solana stakes managed by a Stake-o-Matic

pub mod error;
pub mod instruction;
pub mod processor;
pub mod stake;
pub mod state;

/// Current program version
pub const PROGRAM_VERSION: u8 = 1;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("poo1B9L9nR3CrcaziKVYVpRX6A9Y1LAXYasjjfCbApj");
