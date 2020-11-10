//! Rust example demonstrating invoking another program
#![deny(missing_docs)]
#![forbid(unsafe_code)]

mod entrypoint;
pub mod processor;

solana_program::declare_id!("invoker111111111111111111111111111111111111");
