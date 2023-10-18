//! Utility function for checking an update authority

use {
    solana_program::{
        account_info::AccountInfo,
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
    spl_token_metadata_interface::error::TokenMetadataError,
};

pub fn check_update_authority(
    update_authority_info: &AccountInfo,
    expected_update_authority: &OptionalNonZeroPubkey,
) -> Result<(), ProgramError> {
    if !update_authority_info.is_signer {
        return Err(ProgramError::MissingRequiredSignature);
    }
    let update_authority = Option::<Pubkey>::from(*expected_update_authority)
        .ok_or(TokenMetadataError::ImmutableMetadata)?;
    if update_authority != *update_authority_info.key {
        return Err(TokenMetadataError::IncorrectUpdateAuthority.into());
    }
    Ok(())
}