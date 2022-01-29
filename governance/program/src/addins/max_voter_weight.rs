//! MaxVoterWeight Addin interface

use solana_program::{
    account_info::AccountInfo, clock::Clock, program_error::ProgramError, pubkey::Pubkey,
    sysvar::Sysvar,
};
use spl_governance_addin_api::max_voter_weight::MaxVoterWeightRecord;
use spl_governance_tools::account::get_account_data;

use crate::error::GovernanceError;

/// Asserts MaxVoterWeightRecord hasn't expired
pub fn assert_is_valid_max_voter_weight(
    max_voter_weight_record: &MaxVoterWeightRecord,
) -> Result<(), ProgramError> {
    // Assert max voter weight is not stale
    if let Some(max_voter_weight_expiry) = max_voter_weight_record.max_voter_weight_expiry {
        let slot = Clock::get()?.slot;

        if slot > max_voter_weight_expiry {
            return Err(GovernanceError::MaxVoterWeightRecordExpired.into());
        }
    }

    Ok(())
}

/// Deserializes MaxVoterWeightRecord account and checks owner program
pub fn get_max_voter_weight_record_data(
    program_id: &Pubkey,
    max_voter_weight_record_info: &AccountInfo,
) -> Result<MaxVoterWeightRecord, ProgramError> {
    get_account_data::<MaxVoterWeightRecord>(program_id, max_voter_weight_record_info)
}

/// Deserializes MaxVoterWeightRecord account, checks owner program and asserts it's for the given realm and governing_token_mint
pub fn get_max_voter_weight_record_data_for_realm_and_governing_token_mint(
    program_id: &Pubkey,
    max_voter_weight_record_info: &AccountInfo,
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
) -> Result<MaxVoterWeightRecord, ProgramError> {
    let max_voter_weight_record_data =
        get_max_voter_weight_record_data(program_id, max_voter_weight_record_info)?;

    if max_voter_weight_record_data.realm != *realm {
        return Err(GovernanceError::InvalidMaxVoterWeightRecordForRealm.into());
    }

    if max_voter_weight_record_data.governing_token_mint != *governing_token_mint {
        return Err(GovernanceError::InvalidMaxVoterWeightRecordForGoverningTokenMint.into());
    }

    Ok(max_voter_weight_record_data)
}
