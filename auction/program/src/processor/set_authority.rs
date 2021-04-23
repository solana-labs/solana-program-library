//! Resets authority on an auction account.

use crate::{
    errors::AuctionError,
    processor::{AuctionData, BASE_AUCTION_DATA_SIZE},
    utils::assert_owned_by,
    PREFIX,
};

use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::try_from_slice_unchecked,
        entrypoint::ProgramResult,
        pubkey::Pubkey,
    },
};

pub fn set_authority(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let auction_act = next_account_info(account_iter)?;
    let current_authority = next_account_info(account_iter)?;
    let new_authority = next_account_info(account_iter)?;

    let mut auction: AuctionData = try_from_slice_unchecked(&auction_act.data.borrow_mut())?;
    assert_owned_by(auction_act, program_id)?;

    if auction.authority != *current_authority.key {
        return Err(AuctionError::InvalidAuthority.into());
    }

    if !current_authority.is_signer {
        return Err(AuctionError::InvalidAuthority.into());
    }

    auction.authority = *new_authority.key;
    auction.serialize(&mut *auction_act.data.borrow_mut())?;
    Ok(())
}
