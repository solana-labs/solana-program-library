//! Crate defining the Token Collection program implementing the
//! SPL Token Group interface.

#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod processor;

#[cfg(not(feature = "no-entrypoint"))]
mod entrypoint;
