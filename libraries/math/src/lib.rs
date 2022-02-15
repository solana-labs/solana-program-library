//! Math operations using unsigned integers

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod approximations;
pub mod checked_ceil_div;
mod entrypoint;
pub mod error;
pub mod instruction;
pub mod precise_number;
pub mod processor;
pub mod uint;

solana_program::declare_id!("Math111111111111111111111111111111111111111");
