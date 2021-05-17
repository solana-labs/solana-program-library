//! Program state processor

use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

use crate::{
    error::GovernanceError,
    state::voter_record::{deserialize_voter_record, get_voter_record_address_seeds},
};

/// Processes SetVoteAuthority instruction
pub fn process_set_vote_authority(
    _program_id: &Pubkey,
    accounts: &[AccountInfo],
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
    vote_authority: &Pubkey,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let governing_token_owner_info = next_account_info(account_info_iter)?; // 0
    let voter_record_info = next_account_info(account_info_iter)?; // 1

    if !governing_token_owner_info.is_signer {
        return Err(GovernanceError::GoverningTokenOwnerMustSign.into());
    }

    let voter_record_address_seeds = get_voter_record_address_seeds(
        realm,
        &governing_token_mint,
        governing_token_owner_info.key,
    );

    let mut voter_record_data =
        deserialize_voter_record(voter_record_info, voter_record_address_seeds)?;

    voter_record_data.vote_authority = *vote_authority;

    voter_record_data.serialize(&mut *voter_record_info.data.borrow_mut())?;

    Ok(())
}
