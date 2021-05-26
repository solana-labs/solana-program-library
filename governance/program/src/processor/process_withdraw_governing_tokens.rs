//! Program state processor

use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

use crate::{
    error::GovernanceError,
    state::{
        realm::{deserialize_realm_raw, get_realm_address_seeds},
        token_owner_record::{
            deserialize_token_owner_record, get_token_owner_record_address_seeds,
        },
    },
    tools::token::{get_mint_from_token_account, transfer_spl_tokens_signed},
};

/// Processes WithdrawGoverningTokens instruction
pub fn process_withdraw_governing_tokens(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let governing_token_holding_info = next_account_info(account_info_iter)?; // 1
    let governing_token_destination_info = next_account_info(account_info_iter)?; // 2
    let governing_token_owner_info = next_account_info(account_info_iter)?; // 3
    let token_owner_record_info = next_account_info(account_info_iter)?; // 4
    let spl_token_info = next_account_info(account_info_iter)?; // 5

    if !governing_token_owner_info.is_signer {
        return Err(GovernanceError::GoverningTokenOwnerMustSign.into());
    }

    let realm_data = deserialize_realm_raw(realm_info)?;
    let governing_token_mint = get_mint_from_token_account(governing_token_holding_info)?;

    let token_owner_record_address_seeds = get_token_owner_record_address_seeds(
        realm_info.key,
        &governing_token_mint,
        governing_token_owner_info.key,
    );

    let mut token_owner_record_data =
        deserialize_token_owner_record(token_owner_record_info, &token_owner_record_address_seeds)?;

    if token_owner_record_data.active_votes_count > 0 {
        return Err(GovernanceError::CannotWithdrawGoverningTokensWhenActiveVotesExist.into());
    }

    transfer_spl_tokens_signed(
        &governing_token_holding_info,
        &governing_token_destination_info,
        &realm_info,
        &get_realm_address_seeds(&realm_data.name),
        program_id,
        token_owner_record_data.governing_token_deposit_amount,
        spl_token_info,
    )?;

    token_owner_record_data.governing_token_deposit_amount = 0;
    token_owner_record_data.serialize(&mut *token_owner_record_info.data.borrow_mut())?;

    Ok(())
}
