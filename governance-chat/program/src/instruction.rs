//! Program instructions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
};

/// Instructions supported by the Governance program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
#[allow(clippy::large_enum_variant)]
pub enum GovernanceChatInstruction {
    /// Post message
    PostMessage,
}

/// Creates PostMessage instruction
pub fn post_message(
    program_id: &Pubkey,
    // Accounts
    proposal: &Pubkey,

    payer: &Pubkey,
    // Args
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*proposal, false),
        AccountMeta::new_readonly(*payer, true),
    ];

    let instruction = GovernanceChatInstruction::PostMessage {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}
