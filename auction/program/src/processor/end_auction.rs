use crate::{
    errors::AuctionError,
    processor::{AuctionData, AuctionState, Bid, BidState, WinnerLimit},
    utils::{assert_owned_by, assert_derivation, create_or_allocate_account_raw},
    PREFIX,
};

use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::try_from_slice_unchecked,
        clock::Clock,
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
        sysvar::Sysvar,
    },
    std::mem,
};

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct EndAuctionArgs {
    /// The resource being auctioned. See AuctionData.
    pub resource: Pubkey,
}

pub fn end_auction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: EndAuctionArgs,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let auction_act = next_account_info(account_iter)?;
    let clock_sysvar = next_account_info(account_iter)?;
    let clock = Clock::from_account_info(clock_sysvar)?;

    assert_derivation(
        program_id,
        auction_act,
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            &args.resource.as_ref(),
        ],
    )?;

    let mut auction: AuctionData = try_from_slice_unchecked(&auction_act.data.borrow())?;
    let clock_sysvar = next_account_info(account_iter)?;
    let clock = Clock::from_account_info(clock_sysvar)?;
    auction.ended_at = Some(clock.slot);
    auction.serialize(&mut *auction_act.data.borrow_mut())?;

    Ok(())
}
