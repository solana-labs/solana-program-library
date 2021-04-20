use crate::{
    errors::AuctionError,
    processor::{AuctionData, AuctionState, Bid, BidState, WinnerLimit},
    utils::{assert_owned_by, create_or_allocate_account_raw},
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
pub struct StartAuctionArgs {
    /// The resource being auctioned. See AuctionData.
    pub resource: Pubkey,
}

pub fn start_auction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: StartAuctionArgs,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let creator_act = next_account_info(account_iter)?;
    let auction_act = next_account_info(account_iter)?;
    let clock_sysvar = next_account_info(account_iter)?;
    let clock = Clock::from_account_info(clock_sysvar)?;

    let auction_path = [
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &args.resource.as_ref(),
    ];

    // Derive auction address so we can make the modifications necessary to start it.
    let (auction_key, bump) = Pubkey::find_program_address(&auction_path, program_id);
    if auction_key != *auction_act.key {
        return Err(AuctionError::InvalidAuctionAccount.into());
    }

    let mut auction: AuctionData = try_from_slice_unchecked(&auction_act.data.borrow())?;
    auction.state = AuctionState::create();
    if let Some(end_auction_at) = auction.end_auction_at {
        auction.ended_at = match clock.slot.checked_add(end_auction_at) {
            Some(val) => Some(val),
            None => return Err(AuctionError::NumericalOverflowError.into()),
        };
    }
    auction.serialize(&mut *auction_act.data.borrow_mut())?;

    Ok(())
}
