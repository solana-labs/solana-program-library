//! Creates a new auction account. This will verify the start time is valid, and that the resource
//! being bid on exists. The creator of the auction by default has authority to modify the auction
//! state, including setting someone else as the auction authority.
//!
//! Possible methods to store bid data.
//!
//! 1) Store the entire bid history in the auction account itself with a list.
//! 2) Use a counter for total number of bids, and use PDAs to store individual bids.
//! 3) Create a ring buffer the size of the winner list, and throw away cancelled bids.
//!
//! For now going with 1 for ease of implementation, will come back to this to figure out cost
//! and/or efficiency of the optoins.

use crate::{
    errors::AuctionError,
    processor::{AuctionData, AuctionState, Bid, BidState, WinnerLimit, BASE_AUCTION_DATA_SIZE},
    utils::{assert_owned_by, create_or_allocate_account_raw},
    PREFIX,
};

use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::try_from_slice_unchecked,
        clock::Slot,
        entrypoint::ProgramResult,
        msg,
        pubkey::Pubkey,
    },
    std::mem,
};

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct CreateAuctionArgs {
    /// How many winners are allowed for this auction. See AuctionData.
    pub winners: WinnerLimit,
    /// The resource being auctioned. See AuctionData.
    pub resource: Pubkey,
    /// End time is the cut-off point that the auction is forced to end by. See AuctionData.
    pub end_auction_at: Option<Slot>,
    /// Gap time is how much time after the previous bid where the auction ends. See AuctionData.
    pub end_auction_gap: Option<Slot>,
    /// Token mint for the SPL token used for bidding.
    pub token_mint: Pubkey,
    /// Authority
    pub authority: Pubkey,
}

pub fn create_auction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CreateAuctionArgs,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let creator_act = next_account_info(account_iter)?;
    let auction_act = next_account_info(account_iter)?;
    let rent_act = next_account_info(account_iter)?;
    let system_account = next_account_info(account_iter)?;

    let auction_path = [
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &args.resource.to_bytes(),
    ];

    // Derive the address we'll store the auction in, and confirm it matches what we expected the
    // user to provide.
    let (auction_key, bump) = Pubkey::find_program_address(&auction_path, program_id);
    if auction_key != *auction_act.key {
        return Err(AuctionError::InvalidAuctionAccount.into());
    }

    // The data must be large enough to hold at least the number of winners.
    let auction_size = match args.winners {
        WinnerLimit::Capped(n) => mem::size_of::<Bid>() * n + BASE_AUCTION_DATA_SIZE,
        WinnerLimit::Unlimited => BASE_AUCTION_DATA_SIZE,
    };

    let bid_state = match args.winners {
        WinnerLimit::Capped(n) => BidState::new_english(n),
        WinnerLimit::Unlimited => BidState::new_open_edition(),
    };

    // Create auction account with enough space for a winner tracking.
    msg!("Allocating Auction");
    create_or_allocate_account_raw(
        *program_id,
        auction_act,
        rent_act,
        system_account,
        creator_act,
        auction_size,
        &[
            PREFIX.as_bytes(),
            program_id.as_ref(),
            &args.resource.to_bytes(),
            &[bump],
        ],
    )?;

    // Configure Auction.
    AuctionData {
        authority: args.authority,
        resource: args.resource,
        token_mint: args.token_mint,
        state: AuctionState::create(),
        bid_state: bid_state,
        last_bid: None,
        ended_at: None,
        end_auction_at: args.end_auction_at,
        end_auction_gap: args.end_auction_gap,
    }
    .serialize(&mut *auction_act.data.borrow_mut())?;

    Ok(())
}
