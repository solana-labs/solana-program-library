//! Program state

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, clock::UnixTimestamp, program_error::ProgramError, pubkey::Pubkey,
};

use spl_governance_tools::account::{assert_is_valid_account, AccountMaxSize};

/// Defines all GovernanceChat accounts types
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum GovernanceChatAccountType {
    /// Default uninitialized account state
    Uninitialized,

    /// Chat message
    ChatMessage,
}

/// Chat message body
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum MessageBody {
    /// Text message encoded as utf-8 string
    Text(String),

    /// Emoticon encoded using utf-8 characters
    /// In the UI reactions are displayed together under the parent message (as opposed to hierarchical replies)
    Reaction(String),
}

/// Chat message
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct ChatMessage {
    /// Account type
    pub account_type: GovernanceChatAccountType,

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
        let body_size = match &self.body {
            MessageBody::Text(body) => body.len(),
            MessageBody::Reaction(body) => body.len(),
        };

        Some(body_size + 111)
    }
}

/// Checks whether realm account exists, is initialized and  owned by Governance program
pub fn assert_is_valid_chat_message(
    program_id: &Pubkey,
    chat_message_info: &AccountInfo,
) -> Result<(), ProgramError> {
    assert_is_valid_account(
        chat_message_info,
        GovernanceChatAccountType::ChatMessage,
        program_id,
    )
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_max_size() {
        let message = ChatMessage {
            account_type: GovernanceChatAccountType::ChatMessage,
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
