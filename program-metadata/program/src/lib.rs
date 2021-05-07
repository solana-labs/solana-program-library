//! A Program Metadata program for the Solana blockchain.

pub mod entrypoint;
pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;
pub mod utils;
pub use solana_program;

solana_program::declare_id!("metaNX4YXzpBHkyYkzB1sHeRacfdJV8PVgf59KE9oub");
