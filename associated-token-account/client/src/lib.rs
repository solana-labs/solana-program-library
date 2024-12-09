//! Client crate for interacting with the spl-associated-token-account program
#![deny(missing_docs)]
#![forbid(unsafe_code)]

pub mod address;
pub mod instruction;

/// Module defining the program id
pub mod program {
    solana_pubkey::declare_id!("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
}
