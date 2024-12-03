//! Slashing program
#![deny(missing_docs)]

pub mod duplicate_block_proof;
mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
mod shred;
pub mod state;

// Export current SDK types for downstream users building with a different SDK
// version
pub use solana_program;

solana_program::declare_id!("S1ashing11111111111111111111111111111111111");
