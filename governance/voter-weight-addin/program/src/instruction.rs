//! Program instructions

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

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
