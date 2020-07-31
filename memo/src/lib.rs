#![deny(missing_docs)]

//! A simple program that accepts a string of encoded characters and verifies that it parses. Currently handles UTF-8.

pub mod entrypoint;

// Export current solana-sdk types for downstream users who may also be building with a different
// solana-sdk version
pub use solana_sdk;

solana_sdk::declare_id!("Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo");
