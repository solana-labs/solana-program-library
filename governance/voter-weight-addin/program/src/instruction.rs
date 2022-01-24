//! Program instructions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program,
};

/// Instructions supported by the VoterWeightInstruction addin program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
#[allow(clippy::large_enum_variant)]
pub enum VoterWeightAddinInstruction {
    /// Revises voter weight providing up to date voter weight
    ///
    /// 0. `[]` Governance Program Id
    /// 1. `[]` Realm account
    /// 2. `[]` Governing Token mint
    /// 3. `[]` TokenOwnerRecord
    /// 4. `[writable]` VoterWeightRecord
    Revise {},

    /// Deposits governing token
    /// 0. `[]` Governance Program Id
    /// 1. `[]` Realm account
    /// 2. `[]` Governing Token mint
    /// 3. `[]` TokenOwnerRecord
    /// 4. `[writable]` VoterWeightRecord
    /// 5. `[signer]` Payer
    /// 6. `[]` System
    Deposit {
        /// The deposit amount
        #[allow(dead_code)]
        amount: u64,
    },

    /// Withdraws deposited tokens
    /// Note: This instruction should ensure the tokens can be withdrawn form the Realm
    ///       by calling TokenOwnerRecord.assert_can_withdraw_governing_tokens()
    ///
    /// 0. `[]` Governance Program Id
    /// 1. `[]` Realm account
    /// 2. `[]` Governing Token mint
    /// 3. `[]` TokenOwnerRecord
    /// 4. `[writable]` VoterWeightRecord
    Withdraw {},
}

/// Creates Deposit instruction
#[allow(clippy::too_many_arguments)]
pub fn deposit_voter_weight(
    program_id: &Pubkey,
    // Accounts
    governance_program_id: &Pubkey,
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
    token_owner_record: &Pubkey,
    voter_weight_record: &Pubkey,
    payer: &Pubkey,
    // Args
    amount: u64,
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

    let instruction = VoterWeightAddinInstruction::Deposit { amount };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}
