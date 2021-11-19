#![deny(missing_docs)]
//! A Governance program for the Solana blockchain.

pub mod addins;
pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod tools;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

/// Seed prefix for Governance  PDAs
pub const PROGRAM_AUTHORITY_SEED: &[u8] = b"governance";
