//! Program state processor

use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

use crate::{
    error::GovernanceError,
    state::realm::{assert_valid_realm_config_args, get_realm_data_for_authority, RealmConfigArgs},
};

/// Processes SetRealmConfig instruction
pub fn process_set_realm_config(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    config_args: RealmConfigArgs,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let realm_authority_info = next_account_info(account_info_iter)?; // 1

    let mut realm_data =
        get_realm_data_for_authority(program_id, realm_info, realm_authority_info.key)?;

    if !realm_authority_info.is_signer {
        return Err(GovernanceError::RealmAuthorityMustSign.into());
    }

    assert_valid_realm_config_args(&config_args)?;

    if config_args.use_council_mint {
        let council_token_mint_info = next_account_info(account_info_iter)?;

        // Council mint can only be at present set to none (removed) and changing it to other mint is not supported
        // It might be implemented in future versions but it needs careful planning
        // It can potentially open a can of warms like what happens with existing deposits or pending proposals
        if let Some(council_token_mint) = realm_data.config.council_mint {
            // Council mint can't be changed to different one
            if council_token_mint != *council_token_mint_info.key {
                return Err(GovernanceError::RealmCouncilMintChangeIsNotSupported.into());
            }
        } else {
            // Council mint can't be restored (changed from None)
            return Err(GovernanceError::RealmCouncilMintChangeIsNotSupported.into());
        }
    } else {
        // Remove council mint from realm
        // Note: In the current implementation this also makes it impossible to withdraw council tokens
        realm_data.config.council_mint = None;
    }

    realm_data.config.community_mint_max_vote_weight_source =
        config_args.community_mint_max_vote_weight_source;
    realm_data.config.min_community_tokens_to_create_governance =
        config_args.min_community_tokens_to_create_governance;

    realm_data.serialize(&mut *realm_info.data.borrow_mut())?;

    Ok(())
}
