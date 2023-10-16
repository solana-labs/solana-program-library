//! Crate defining an example program for creating SPL token collections
//! using the SPL Token Group interface.

#![deny(missing_docs)]
#![cfg_attr(not(test), forbid(unsafe_code))]

pub mod processor;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;
