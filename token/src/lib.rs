#![deny(missing_docs)]

//! An ERC20-like Token program for the Solana blockchain

pub mod error;
pub mod instruction;
pub mod native_mint;
pub mod option;
pub mod processor;
pub mod state;

solana_sdk::declare_id!("TokenSVp5gheXUvJ6jGWGeCsgPKgnE3YgdGKRVCMY9o");
