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

/// The governance action VoterWeight is evaluated for
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum VoterWeightAction {
    /// Cast vote for a proposal. Target: Proposal
    CastVote,

    /// Comment a proposal. Target: Proposal
    CommentProposal,

    /// Create Governance within a realm. Target: Realm
    CreateGovernance,

    /// Create a proposal for a governance. Target: Governance
    CreateProposal,
}

/// VoterWeightRecord account
/// The account is used as an api interface used to provide voting power to the governance program from external addin contracts
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
    /// This is the actual owner (voter) and corresponds to TokenOwnerRecord.governing_token_owner
    pub governing_token_owner: Pubkey,

    /// Voter's weight
    /// The weight of the voter provided by the addin for the given realm, governing_token_mint and governing_token_owner (voter)
    pub voter_weight: u64,

    /// The slot when the voting weight expires
    /// It should be set to None if the weight never expires
    /// If the voter weight decays with time, for example for time locked based weights, then the expiry must be set
    /// As a common pattern Revise instruction to update the weight should be invoked before governance instruction within the same transaction
    /// and the expiry set to the current slot to provide up to date weight
    pub voter_weight_expiry: Option<Slot>,

    /// The governance action the voter's weight pertains to
    /// It allows to provided voter's weight specific to the particular action the weight is evaluated for
    /// When the action is provided then the governance program asserts the executing action is the same as specified by the addin
    pub weight_action: Option<VoterWeightAction>,

    /// The target the voter's weight  action pertains to
    /// It allows to provided voter's weight specific to the target the weight is evaluated for
    /// For example when addin supplies weight to vote on a particular proposal then it must specify the proposal as the action target
    /// When the target is provided then the governance program asserts the target is the same as specified by the addin
    pub weight_action_target: Option<Pubkey>,
}

impl AccountMaxSize for VoterWeightRecord {}

impl IsInitialized for VoterWeightRecord {
    fn is_initialized(&self) -> bool {
        self.account_type == VoterWeightAccountType::VoterWeightRecord
    }
}

impl VoterWeightRecord {
    /// Asserts the VoterWeightRecord hasn't expired and matches the given action and target if specified
    pub fn assert_is_valid_voter_weight(
        &self,
        weight_action: VoterWeightAction,
        weight_action_target: &Pubkey,
    ) -> Result<(), ProgramError> {
        // Assert the weight is not stale
        if let Some(voter_weight_expiry) = self.voter_weight_expiry {
            let slot = Clock::get()?.slot;

            if slot > voter_weight_expiry {
                return Err(GovernanceError::VoterWeightRecordExpired.into());
            }
        }

        // Assert the weight is for the action specified by the addin
        if let Some(voter_weight_action) = &self.weight_action {
            if voter_weight_action != &weight_action {
                return Err(GovernanceError::VoterWeightRecordInvalidAction.into());
            }
        }

        // Assert the weight is for the action target specified by the addin
        if let Some(voter_weight_action_target) = &self.weight_action_target {
            if voter_weight_action_target != weight_action_target {
                return Err(GovernanceError::VoterWeightRecordInvalidActionTarget.into());
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
