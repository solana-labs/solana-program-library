//! Crate defining a library with a procedural macro and other
//! dependencies for building Solana progrem errors

#![allow(clippy::integer_arithmetic)]
#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

extern crate self as solana_program_error_derive;

// Make these available downstream for the macro to work without
// additional imports
pub use num_derive::FromPrimitive;
pub use num_traits;
pub use solana_program;
pub use solana_program_error_derive_impl::solana_program_error;
pub use thiserror::Error;
