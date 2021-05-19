//! Governance asserts

use solana_program::{account_info::AccountInfo, program_error::ProgramError};

use crate::{error::GovernanceError, state::voter_record::VoterRecord};

/// Checks wether the provided vote authority can set new  vote authority
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
