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

pub use spl_program_ids::spl_math::*;
