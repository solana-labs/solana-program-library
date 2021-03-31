#![deny(missing_docs)]
//! A metadata program for the Solana blockchain.

pub mod entrypoint;
pub mod error;
/// instruction
pub mod instruction;
/// processor
pub mod processor;
///state
pub mod state;
/// utils
pub mod utils;
// Export current sdk types for downstream users building with a different sdk version
pub use solana_program;

solana_program::declare_id!("metaTMm9vFU2h7m97KYJA9k2JQUG21rfb434rWUQBiH");
