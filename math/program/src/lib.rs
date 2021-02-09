//! Precise calculations using unsigned integers
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod entrypoint;
pub mod error;
pub mod instruction;
pub mod precise_number;
pub mod processor;
pub mod uint;

solana_program::declare_id!("Math111111111111111111111111111111111111111");
