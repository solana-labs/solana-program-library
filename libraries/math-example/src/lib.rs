//! Math operations using unsigned integers

#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;

pub use spl_math::{approximations, checked_ceil_div, precise_number, uint};

solana_program::declare_id!("Math111111111111111111111111111111111111111");
