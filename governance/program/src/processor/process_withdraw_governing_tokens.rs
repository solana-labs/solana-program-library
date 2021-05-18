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
        realm::{deserialize_realm, get_realm_address_seeds},
        voter_record::{deserialize_voter_record, get_voter_record_address_seeds},
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
    let voter_record_info = next_account_info(account_info_iter)?; // 4
    let spl_token_info = next_account_info(account_info_iter)?; // 5

    if !governing_token_owner_info.is_signer {
        return Err(GovernanceError::GoverningTokenOwnerMustSign.into());
    }

    let realm_data = deserialize_realm(realm_info)?;
    let governing_token_mint = get_mint_from_token_account(governing_token_holding_info)?;

    let voter_record_address_seeds = get_voter_record_address_seeds(
        realm_info.key,
        &governing_token_mint,
        governing_token_owner_info.key,
    );

    let mut voter_record_data =
        deserialize_voter_record(voter_record_info, &voter_record_address_seeds)?;

    if voter_record_data.active_votes_count > 0 {
        return Err(GovernanceError::CannotWithdrawGoverningTokensWhenActiveVotesExist.into());
    }

    transfer_spl_tokens_signed(
        &governing_token_holding_info,
        &governing_token_destination_info,
        &realm_info,
        &get_realm_address_seeds(&realm_data.name),
        program_id,
        voter_record_data.token_deposit_amount,
        spl_token_info,
    )?;

    voter_record_data.token_deposit_amount = 0;
    voter_record_data.serialize(&mut *voter_record_info.data.borrow_mut())?;

    Ok(())
}
