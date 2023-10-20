//! Crate defining an example program for creating SPL token groups
//! using the SPL Token Group interface.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod processor;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;
