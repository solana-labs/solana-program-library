//! Program state processor

use {
    crate::{
        error::GovernanceError,
        state::{
            realm::{get_realm_address_seeds, get_realm_data},
            realm_config::get_realm_config_data_for_realm,
            token_owner_record::get_token_owner_record_data_for_realm_and_governing_mint,
        },
        tools::spl_token::{assert_spl_token_mint_authority_is_signer, burn_spl_tokens_signed},
    },
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        entrypoint::ProgramResult,
        pubkey::Pubkey,
    },
};

/// Processes RevokeGoverningTokens instruction
pub fn process_revoke_governing_tokens(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    amount: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0

    let governing_token_holding_info = next_account_info(account_info_iter)?; // 1
    let token_owner_record_info = next_account_info(account_info_iter)?; // 2

    let governing_token_mint_info = next_account_info(account_info_iter)?; // 3
    let revoke_authority_info = next_account_info(account_info_iter)?; // 4

    let realm_config_info = next_account_info(account_info_iter)?; // 5
    let spl_token_info = next_account_info(account_info_iter)?; // 6

    let realm_data = get_realm_data(program_id, realm_info)?;

    realm_data.assert_is_valid_governing_token_mint_and_holding(
        program_id,
        realm_info.key,
        governing_token_mint_info.key,
        governing_token_holding_info.key,
    )?;

    let realm_config_data =
        get_realm_config_data_for_realm(program_id, realm_config_info, realm_info.key)?;

    realm_config_data
        .assert_can_revoke_governing_token(&realm_data, governing_token_mint_info.key)?;

    let mut token_owner_record_data = get_token_owner_record_data_for_realm_and_governing_mint(
        program_id,
        token_owner_record_info,
        realm_info.key,
        governing_token_mint_info.key,
    )?;

    // If the governing token owner voluntarily revokes their own membership then
    // the owner must sign the transaction
    if *revoke_authority_info.key == token_owner_record_data.governing_token_owner {
        if !revoke_authority_info.is_signer {
            return Err(GovernanceError::GoverningTokenOwnerMustSign.into());
        }
    } else {
        // If its a forceful membership revocation then the governing_token_mint
        // authority must sign the transaction
        assert_spl_token_mint_authority_is_signer(
            governing_token_mint_info,
            revoke_authority_info,
        )?;
    }

    token_owner_record_data.governing_token_deposit_amount = token_owner_record_data
        .governing_token_deposit_amount
        .checked_sub(amount)
        .ok_or(GovernanceError::InvalidRevokeAmount)?;

    token_owner_record_data.serialize(&mut token_owner_record_info.data.borrow_mut()[..])?;

    burn_spl_tokens_signed(
        governing_token_holding_info,
        governing_token_mint_info,
        realm_info,
        &get_realm_address_seeds(&realm_data.name),
        program_id,
        amount,
        spl_token_info,
    )?;

    Ok(())
}
