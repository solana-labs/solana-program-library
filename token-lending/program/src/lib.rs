#![allow(clippy::arithmetic_side_effects)]
#![deny(missing_docs)]

//! A lending program for the Solana blockchain.

pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod math;
pub mod processor;
pub mod pyth;
pub mod state;

// Export current sdk types for downstream users building with a different sdk
// version
pub use solana_program;

pub use spl_program_ids::spl_token_lending::*;
