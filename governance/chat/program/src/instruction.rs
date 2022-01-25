//! Program instructions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

use crate::state::MessageBody;

/// Instructions supported by the GovernanceChat program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
#[allow(clippy::large_enum_variant)]
pub enum GovernanceChatInstruction {
    /// Posts a message with a comment for a Proposal
    ///
    ///   0. `[]` Governance program id
    ///   1. `[]` Governance account the Proposal is for    
    ///   2. `[]` Proposal account   
    ///   3. `[]` TokenOwnerRecord account for the message author
    ///   4. `[signer]` Governance Authority (TokenOwner or Governance Delegate)
    ///   5. `[writable, signer]` ChatMessage account
    ///   6. `[signer]` Payer    
    ///   7. `[]` System program    
    ///   8. `[]` ReplyTo Message account (optional)  
    PostMessage {
        #[allow(dead_code)]
        /// Message body (text or reaction)
        body: MessageBody,
    },
}

/// Creates PostMessage instruction
#[allow(clippy::too_many_arguments)]
pub fn post_message(
    program_id: &Pubkey,
    // Accounts
    governance_program_id: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    reply_to: Option<Pubkey>,
    chat_message: &Pubkey,
    payer: &Pubkey,
    // Args
    body: MessageBody,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*governance_program_id, false),
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new_readonly(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(*chat_message, true),
        AccountMeta::new_readonly(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    if let Some(reply_to) = reply_to {
        accounts.push(AccountMeta::new_readonly(reply_to, false));
    }

    let instruction = GovernanceChatInstruction::PostMessage { body };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}
