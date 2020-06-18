#![deny(missing_docs)]

//! An ERC20-like Token program for the Solana blockchain

pub mod error;
pub mod instruction;
pub mod processor;
pub mod state;

solana_sdk::declare_id!("Token11111111111111111111111111111111111111");
