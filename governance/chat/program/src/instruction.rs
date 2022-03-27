//! Program instructions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};
use spl_governance::instruction::with_realm_config_accounts;

use crate::state::MessageBody;

/// Instructions supported by the GovernanceChat program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
#[allow(clippy::large_enum_variant)]
pub enum GovernanceChatInstruction {
    /// Posts a message with a comment for a Proposal
    ///
    ///   0. `[]` Governance program id
    ///   1. `[]` Realm account of the Proposal
    ///   2. `[]` Governance account the Proposal is for    
    ///   3. `[]` Proposal account   
    ///   4. `[]` TokenOwnerRecord account for the message author
    ///   5. `[signer]` Governance Authority (TokenOwner or Governance Delegate)
    ///   6. `[writable, signer]` ChatMessage account
    ///   7. `[signer]` Payer    
    ///   8. `[]` System program    
    ///   9. `[]` ReplyTo Message account (optional)  
    ///    10. `[]` Optional Voter Weight Record
    PostMessage {
        #[allow(dead_code)]
        /// Message body (text or reaction)
        body: MessageBody,

        #[allow(dead_code)]
        /// Indicates whether the message is a reply to another message
        /// If yes then ReplyTo Message account has to be provided
        is_reply: bool,
    },
}

/// Creates PostMessage instruction
#[allow(clippy::too_many_arguments)]
pub fn post_message(
    program_id: &Pubkey,
    // Accounts
    governance_program_id: &Pubkey,
    realm: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    reply_to: Option<Pubkey>,
    chat_message: &Pubkey,
    payer: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    // Args
    body: MessageBody,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*governance_program_id, false),
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new_readonly(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(*chat_message, true),
        AccountMeta::new_readonly(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let is_reply = if let Some(reply_to) = reply_to {
        accounts.push(AccountMeta::new_readonly(reply_to, false));
        true
    } else {
        false
    };

    with_realm_config_accounts(
        governance_program_id,
        &mut accounts,
        realm,
        voter_weight_record,
        None,
    );

    let instruction = GovernanceChatInstruction::PostMessage { body, is_reply };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}
