#![deny(missing_docs)]

//! A simple program that accepts a string of encoded characters and verifies that it parses. Currently handles UTF-8.

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;

// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo");
