//! Program state

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{clock::UnixTimestamp, pubkey::Pubkey};
use spl_governance::tools::account::AccountMaxSize;

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
    pub reply_to: Option<Pubkey>,

    /// Body of the message
    pub body: String,
}

impl AccountMaxSize for Message {
    fn get_max_size(&self) -> Option<usize> {
        Some(self.body.len() + 109)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_max_size() {
        let message = Message {
            proposal: Pubkey::new_unique(),
            author: Pubkey::new_unique(),
            post_at: 10,
            reply_to: Some(Pubkey::new_unique()),
            body: "message".to_string(),
        };
        let size = message.try_to_vec().unwrap().len();

        assert_eq!(message.get_max_size(), Some(size));
    }
}
