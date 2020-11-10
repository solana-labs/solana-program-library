//! An implementation of Brave's THEMIS for the Solana blockchain
#![forbid(unsafe_code)]

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

#[cfg(not(feature = "no-entrypoint"))]
pub mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("C8tR6A3CWcEL46KHx7TJcbyR4hdoPi1wrBBQa42FuJMF");
