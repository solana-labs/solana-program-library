//! Governance asserts

use solana_program::{account_info::AccountInfo, program_error::ProgramError};

use crate::{error::GovernanceError, id, state::voter_record::VoterRecord};

/// Checks whether the provided vote authority can set new  vote authority
pub fn assert_is_signed_by_owner_or_vote_authority(
    voter_record: &VoterRecord,
    vote_authority_info: &AccountInfo,
) -> Result<(), ProgramError> {
    if vote_authority_info.is_signer {
        if &voter_record.token_owner == vote_authority_info.key {
            return Ok(());
        }

        if let Some(vote_authority) = voter_record.vote_authority {
            if &vote_authority == vote_authority_info.key {
                return Ok(());
            }
        };
    }

    Err(GovernanceError::GoverningTokenOwnerOrVoteAuthrotiyMustSign.into())
}

/// Checks whether realm exists and is owned by Governance
pub fn assert_is_valid_realm(realm_info: &AccountInfo) -> Result<(), ProgramError> {
    if realm_info.data_len() == 0 || realm_info.owner != &id() {
        return Err(GovernanceError::InvalidRealm.into());
    }

    Ok(())
}
