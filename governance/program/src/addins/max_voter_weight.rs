//! MaxVoterWeight Addin interface

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, clock::Slot, program_error::ProgramError,
    program_pack::IsInitialized, pubkey::Pubkey,
};
use spl_governance_tools::account::{get_account_data, AccountMaxSize};

/// MaxVoterWeight addin account type
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum MaxVoterWeightAccountType {
    /// Default uninitialized account state
    Uninitialized,

    /// Max Voter Weight Record
    MaxVoterWeightRecord,
}

/// MaxVoterWeightRecord account
/// The account is used as an api interface to provide max voting power to the governance program from external addin contracts
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct MaxVoterWeightRecord {
    /// MaxVoterWeightRecord account type
    pub account_type: MaxVoterWeightAccountType,

    /// The Realm the MaxVoterWeightRecord belongs to
    pub realm: Pubkey,

    /// Governing Token Mint the MaxVoterWeightRecord is associated with
    /// Note: The addin can take deposits of any tokens and is not restricted to the community or council tokens only
    // The mint here is to link the record to either community or council mint of the realm
    pub governing_token_mint: Pubkey,

    /// Max voter weight
    /// The max voter weight provided by the addin for the given realm and governing_token_mint
    pub max_voter_weight: u64,

    /// The slot when the max voting weight expires
    /// It should be set to None if the weight never expires
    /// If the max vote weight decays with time, for example for time locked based weights, then the expiry must be set
    /// As a pattern Revise instruction to update the max weight should be invoked before governance instruction within the same transaction
    /// and the expiry set to the current slot to provide up to date weight
    pub max_voter_weight_expiry: Option<Slot>,
}

impl AccountMaxSize for MaxVoterWeightRecord {}

impl IsInitialized for MaxVoterWeightRecord {
    fn is_initialized(&self) -> bool {
        self.account_type == MaxVoterWeightAccountType::MaxVoterWeightRecord
    }
}

/// Deserializes MaxVoterWeightRecord account and checks owner program
pub fn get_max_voter_weight_record_data(
    program_id: &Pubkey,
    max_voter_weight_record_info: &AccountInfo,
) -> Result<MaxVoterWeightRecord, ProgramError> {
    get_account_data::<MaxVoterWeightRecord>(program_id, max_voter_weight_record_info)
}
