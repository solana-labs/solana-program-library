//! An implementation of Brave's THEMIS for the Solana blockchain
#![forbid(unsafe_code)]

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "exclude_entrypoint"))]
pub mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("F3FWeYPjD1jeR6UykMj1GRbCcmoxtJnDiPuFdTLRGvb6");
