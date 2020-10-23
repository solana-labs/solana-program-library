//! An implementation of Brave's THEMIS for the Solana blockchain
#![forbid(unsafe_code)]

pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

// Export current solana-sdk types for downstream users who may also be building with a different
// solana-sdk version
pub use solana_sdk;

solana_sdk::declare_id!("C8tR6A3CWcEL46KHx7TJcbyR4hdoPi1wrBBQa42FuJMF");
