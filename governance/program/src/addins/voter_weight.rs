//! VoterWeight Addin interface

use solana_program::{
    account_info::AccountInfo, clock::Clock, program_error::ProgramError, pubkey::Pubkey,
    sysvar::Sysvar,
};
use spl_governance_addin_api::voter_weight::{VoterWeightAction, VoterWeightRecord};
use spl_governance_tools::account::get_account_data;

use crate::{error::GovernanceError, state::token_owner_record::TokenOwnerRecordV2};

/// Asserts the VoterWeightRecord hasn't expired and matches the given action and target if specified
pub fn assert_is_valid_voter_weight(
    voter_weight_record: &VoterWeightRecord,
    weight_action: VoterWeightAction,
    weight_action_target: &Pubkey,
) -> Result<(), ProgramError> {
    // Assert the weight is not stale
    if let Some(voter_weight_expiry) = voter_weight_record.voter_weight_expiry {
        let slot = Clock::get()?.slot;

        if slot > voter_weight_expiry {
            return Err(GovernanceError::VoterWeightRecordExpired.into());
        }
    }

    // Assert the weight is for the action specified by the addin
    if let Some(voter_weight_action) = &voter_weight_record.weight_action {
        if voter_weight_action != &weight_action {
            return Err(GovernanceError::VoterWeightRecordInvalidAction.into());
        }
    }

    // Assert the weight is for the action target specified by the addin
    if let Some(voter_weight_action_target) = &voter_weight_record.weight_action_target {
        if voter_weight_action_target != weight_action_target {
            return Err(GovernanceError::VoterWeightRecordInvalidActionTarget.into());
        }
    }

    Ok(())
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
    token_owner_record: &TokenOwnerRecordV2,
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
