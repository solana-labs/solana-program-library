//! Program state

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{clock::UnixTimestamp, pubkey::Pubkey};

/// Message
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct Message {
    /// The proposal the message is for
    pub proposal: Pubkey,

    /// Author of the proposal
    pub author: Pubkey,

    /// Message timestamp
    pub post_at: UnixTimestamp,

    /// Parent message
    pub parent: Option<Pubkey>,

    /// Body of the message
    pub body: String,
}
