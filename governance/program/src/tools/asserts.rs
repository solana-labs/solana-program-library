//! Governance asserts

use solana_program::{account_info::AccountInfo, program_error::ProgramError};

use crate::{error::GovernanceError, state::token_owner_record::TokenOwnerRecord};

/// Checks whether the provided Governance Authority signed transaction
pub fn assert_token_owner_or_delegate_is_signer(
    token_owner_record: &TokenOwnerRecord,
    governance_authority_info: &AccountInfo,
) -> Result<(), ProgramError> {
    if governance_authority_info.is_signer {
        if &token_owner_record.governing_token_owner == governance_authority_info.key {
            return Ok(());
        }

        if let Some(governance_delegate) = token_owner_record.governance_delegate {
            if &governance_delegate == governance_authority_info.key {
                return Ok(());
            }
        };
    }

    Err(GovernanceError::GoverningTokenOwnerOrDelegateMustSign.into())
}
