//! Program state

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{clock::UnixTimestamp, pubkey::Pubkey};
use spl_governance::tools::account::AccountMaxSize;

/// Chat message body
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum MessageBody {
    /// Text message encoded as utf-8 string
    Text(String),

    /// Emoticon encoded using utf-8 characters
    Reaction(String),
}

/// Chat message
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct ChatMessage {
    /// The proposal the message is for
    pub proposal: Pubkey,

    /// Author of the message
    pub author: Pubkey,

    /// Message timestamp
    pub posted_at: UnixTimestamp,

    /// Parent message
    pub reply_to: Option<Pubkey>,

    /// Body of the message
    pub body: MessageBody,
}

impl AccountMaxSize for ChatMessage {
    fn get_max_size(&self) -> Option<usize> {
        let body_size = match self.body.clone() {
            MessageBody::Text(body) => body.len(),
            MessageBody::Reaction(body) => body.len(),
        };

        Some(body_size + 110)
    }
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_max_size() {
        let message = ChatMessage {
            proposal: Pubkey::new_unique(),
            author: Pubkey::new_unique(),
            posted_at: 10,
            reply_to: Some(Pubkey::new_unique()),
            body: MessageBody::Text("message".to_string()),
        };
        let size = message.try_to_vec().unwrap().len();

        assert_eq!(message.get_max_size(), Some(size));
    }
}
