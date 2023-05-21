//! Crate defining a library with a procedural macro and other
//! dependencies for building Solana program errors

#![allow(clippy::integer_arithmetic)]
#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

extern crate self as spl_program_error;

// Make these available downstream for the macro to work without
// additional imports
pub use num_derive::FromPrimitive;
pub use num_traits;
pub use solana_program;
pub use spl_program_error_derive::{
    spl_program_error, DecodeError, IntoProgramError, PrintProgramError,
};
pub use thiserror::Error;
