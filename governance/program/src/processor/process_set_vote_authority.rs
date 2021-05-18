//! Program state processor

use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

use crate::{
    state::voter_record::{deserialize_voter_record, get_voter_record_address_seeds},
    tools::asserts::assert_is_signed_by_owner_or_vote_authority,
};

/// Processes SetVoteAuthority instruction
pub fn process_set_vote_authority(
    accounts: &[AccountInfo],
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
    governing_token_owner: &Pubkey,
    new_vote_authority: &Option<Pubkey>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let vote_authority_info = next_account_info(account_info_iter)?; // 0
    let voter_record_info = next_account_info(account_info_iter)?; // 1

    let mut voter_record_data = deserialize_voter_record(
        voter_record_info,
        &get_voter_record_address_seeds(realm, &governing_token_mint, governing_token_owner),
    )?;

    assert_is_signed_by_owner_or_vote_authority(&voter_record_data, &vote_authority_info)?;

    voter_record_data.vote_authority = *new_vote_authority;
    voter_record_data.serialize(&mut *voter_record_info.data.borrow_mut())?;

    Ok(())
}
