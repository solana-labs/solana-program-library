//! VoterWeight Addin interface

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    clock::{Clock, Slot},
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
use spl_governance_tools::account::{get_account_data, AccountMaxSize};

use crate::{error::GovernanceError, state::token_owner_record::TokenOwnerRecord};

/// VoterWeight account type
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum VoterWeightAccountType {
    /// Default uninitialized account state
    Uninitialized,

    /// Voter Weight Record
    VoterWeightRecord,
}

/// VoterWeight Record account
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct VoterWeightRecord {
    /// VoterWeightRecord account type
    pub account_type: VoterWeightAccountType,

    /// The Realm the VoterWeightRecord belongs to
    pub realm: Pubkey,

    /// Governing Token Mint the VoterWeightRecord is associated with
    /// Note: The addin can take deposits of any tokens and is not restricted to the community or council tokens only
    // The mint here is to link the record to either community or council mint of the realm
    pub governing_token_mint: Pubkey,

    /// The owner of the governing token and voter
    pub governing_token_owner: Pubkey,

    /// Voter's weight
    pub voter_weight: u64,

    /// The slot when the voting weight expires
    /// It should be set to None if the weight never expires
    /// If the voter weight decays with time, for example for time locked based weights, then the expiry must be set
    /// As a common pattern Revise instruction to update the weight should be invoked before governance instruction within the same transaction
    /// and the expiry set to the current slot to provide up to date weight
    pub voter_weight_expiry: Option<Slot>,
}

impl AccountMaxSize for VoterWeightRecord {}

impl IsInitialized for VoterWeightRecord {
    fn is_initialized(&self) -> bool {
        self.account_type == VoterWeightAccountType::VoterWeightRecord
    }
}

impl VoterWeightRecord {
    /// Asserts the VoterWeightRecord hasn't expired
    pub fn assert_is_up_to_date(&self) -> Result<(), ProgramError> {
        if let Some(voter_weight_expiry) = self.voter_weight_expiry {
            let slot = Clock::get()?.slot;

            if slot > voter_weight_expiry {
                return Err(GovernanceError::VoterWeightRecordExpired.into());
            }
        }

        Ok(())
    }
}

/// Deserializes VoterWeightRecord account and checks owner program
pub fn get_voter_weight_record_data(
    program_id: &Pubkey,
    voter_weight_record_info: &AccountInfo,
) -> Result<VoterWeightRecord, ProgramError> {
    get_account_data::<VoterWeightRecord>(program_id, voter_weight_record_info)
}

/// Deserializes VoterWeightRecord account, checks owner program and asserts it's for the same realm, mint and token owner as the provided TokenOwnerRecord
pub fn get_voter_weight_record_data_for_token_owner_record(
    program_id: &Pubkey,
    voter_weight_record_info: &AccountInfo,
    token_owner_record: &TokenOwnerRecord,
) -> Result<VoterWeightRecord, ProgramError> {
    let voter_weight_record_data =
        get_voter_weight_record_data(program_id, voter_weight_record_info)?;

    if voter_weight_record_data.realm != token_owner_record.realm {
        return Err(GovernanceError::InvalidVoterWeightRecordForRealm.into());
    }

    if voter_weight_record_data.governing_token_mint != token_owner_record.governing_token_mint {
        return Err(GovernanceError::InvalidVoterWeightRecordForGoverningTokenMint.into());
    }

    if voter_weight_record_data.governing_token_owner != token_owner_record.governing_token_owner {
        return Err(GovernanceError::InvalidVoterWeightRecordForTokenOwner.into());
    }

    Ok(voter_weight_record_data)
}
