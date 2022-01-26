//! Program instructions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    clock::Slot,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};
use spl_governance::addins::voter_weight::VoterWeightAction;

/// Instructions supported by the VoterWeight addin program
/// This program is a mock program used by spl-governance for testing and not real addin
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
#[allow(clippy::large_enum_variant)]
pub enum VoterWeightAddinInstruction {
    /// Sets up VoterWeightRecord owned by the program
    ///
    /// 0. `[]` Governance Program Id
    /// 1. `[]` Realm account
    /// 2. `[]` Governing Token mint
    /// 3. `[]` TokenOwnerRecord
    /// 4. `[writable]` VoterWeightRecord
    /// 5. `[signer]` Payer
    /// 6. `[]` System
    SetupVoterWeightRecord {
        /// Voter weight
        #[allow(dead_code)]
        voter_weight: u64,

        /// Voter weight expiry
        #[allow(dead_code)]
        voter_weight_expiry: Option<Slot>,

        /// Voter weight action
        #[allow(dead_code)]
        weight_action: Option<VoterWeightAction>,

        /// Voter weight action target
        #[allow(dead_code)]
        weight_action_target: Option<Pubkey>,
    },
}

/// Creates SetupVoterWeightRecord instruction
#[allow(clippy::too_many_arguments)]
pub fn setup_voter_weight_record(
    program_id: &Pubkey,
    // Accounts
    governance_program_id: &Pubkey,
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
    token_owner_record: &Pubkey,
    voter_weight_record: &Pubkey,
    payer: &Pubkey,
    // Args
    voter_weight: u64,
    voter_weight_expiry: Option<Slot>,
    weight_action: Option<VoterWeightAction>,
    weight_action_target: Option<Pubkey>,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*governance_program_id, false),
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(*governing_token_mint, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new(*voter_weight_record, true),
        AccountMeta::new_readonly(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let instruction = VoterWeightAddinInstruction::SetupVoterWeightRecord {
        voter_weight,
        voter_weight_expiry,
        weight_action,
        weight_action_target,
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}
