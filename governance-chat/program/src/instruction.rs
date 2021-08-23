//! Program instructions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

/// Instructions supported by the Governance program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
#[allow(clippy::large_enum_variant)]
pub enum GovernanceChatInstruction {
    /// Post message
    PostMessage {
        #[allow(dead_code)]
        /// UTF-8 encoded Message body
        body: String,
    },
}

/// Creates PostMessage instruction
pub fn post_message(
    program_id: &Pubkey,
    // Accounts
    governance_program: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    reply_to: Option<Pubkey>,
    message: &Pubkey,
    payer: &Pubkey,
    // Args
    body: String,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*governance_program, false),
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new_readonly(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(*message, true),
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
