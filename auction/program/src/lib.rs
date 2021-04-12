use byteorder::{ByteOrder, LittleEndian};
use std::mem;
use {
    crate::utils::assert_owned_by,
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::{
        account_info::{next_account_info, AccountInfo},
        borsh::try_from_slice_unchecked,
        clock::Epoch,
        entrypoint,
        entrypoint::ProgramResult,
        msg,
        program::{invoke, invoke_signed},
        program_error::ProgramError,
        program_pack::{IsInitialized, Pack},
        pubkey::Pubkey,
        system_instruction,
        sysvar::{rent::Rent, Sysvar},
    },
    std::convert::TryInto,
};

mod errors;
mod utils;

/// Declare and export the program's entrypoint
entrypoint!(process_instruction);

/// Prefix used in PDA derivations to avoid collisions.
const PREFIX: &str = "auction";

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
struct Bid(Pubkey, u64);

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
struct BidState {
    bids: Vec<Option<Bid>>,
}

/// Bad bid implementation, just setting up the API.
impl BidState {
    fn new() -> Self {
        BidState { bids: vec![] }
    }

    fn bid(&mut self, bid: Bid) -> Result<(), ProgramError> {
        self.bids.push(Some(bid));
        Ok(())
    }

    fn cancel_bid(&mut self, key: Pubkey) -> Result<(), ProgramError> {
        self.bids
            .retain(|maybe_bid| {
                maybe_bid
                    .as_ref()
                    .map_or(false, |bid| bid.0 != key)
            });
        Ok(())
    }
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
struct AuctionData {
    /// Pubkey of the authority with permission to modify this auction.
    authority: Pubkey,
    /// Auction Bids, each user may have one bid open at a time.
    bids: BidState,
    /// The time the last bid was placed, use to time auction ending.
    last_bid: Option<usize>,
    /// Maximum amount of accounts that may win this bid.
    max_winners: usize,
    /// Pubkey of the resource being bid on.
    resource: Pubkey,
    /// Time the auction starts at, this may be changed only if the auction hasn't started.
    start_time: usize,
}

/* -------------------------------------------------------------------------------- */

/// Creates a new auction account. This will verify the start time is valid, and that the resource
/// being bid on exists. The creator of the auction by default has authority to modify the auction
/// state, including setting someone else as the auction authority.
///
/// Possible methods to store bid data.
///
/// 1) Store the entire bid history in the auction account itself with a list.
/// 2) Use a counter for total number of bids, and use PDAs to store individual bids.
/// 3) Create a ring buffer the size of the winner list, and throw away cancelled bids.
///
/// For now going with 1 for ease of implementation, will come back to this to figure out cost
/// and/or efficiency of the optoins.
fn create_auction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    args: CreateAuctionArgs,
) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let creator_act = next_account_info(account_iter)?;
    let auction_act = next_account_info(account_iter)?;

    let auction_path = [
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &auction_act.key.to_bytes(),
    ];

    // Derive the address we'll store the auction in, and confirm it matches what we expected the
    // user to provide.
    let (auction_key, auction_key_bump) = Pubkey::find_program_address(&auction_path, program_id);
    if auction_key != *auction_act.key {
        return Ok(());
    }

    // Assert all states are valid.
    assert_owned_by(auction_act, program_id)?;

    // The data must be large enough to hold at least the number of winners.
    if auction_act.try_data_len()? < (mem::size_of::<Bid>() * args.max_winners) {
        msg!("Account data to small for auction results.");
        return Err(ProgramError::InvalidAccountData);
    }

    let auction = AuctionData {
        authority: *creator_act.key,
        bids: BidState::new(),
        last_bid: None,
        max_winners: args.max_winners,
        resource: args.resource,
        start_time: args.start_time,
    };

    auction.serialize(&mut *auction_act.data.borrow_mut())?;

    Ok(())
}

/* -------------------------------------------------------------------------------- */

/// Places a bid on a running auction, the logic here implements a standard English auction
/// mechanism, once the auction starts, new bids can be made until 10 minutes has passed with no
/// new bid. At this point the auction ends.
///
/// Possible Attacks to Consider:
///
/// 1) A user bids many many small bids to fill up the buffer, so that his max bid wins.
/// 2) A user bids a large amount repeatedly to indefinitely delay the auction finishing.
///
/// A few solutions come to mind: don't allow cancelling bids, and simply prune all bids that
/// are not winning bids from the state.
fn place_bid(program_id: &Pubkey, accounts: &[AccountInfo], args: PlaceBidArgs) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let bidder_act = next_account_info(account_iter)?;
    let auction_act = next_account_info(account_iter)?;
    let bidder_pot = next_account_info(account_iter)?;

    // This  path references an account to store the users bid SOL in, if the user wins the auction
    // this is claimed by the auction authority, otherwise the user can request to have the SOL
    // sent back.
    let pot_path = [
        PREFIX.as_bytes(),
        program_id.as_ref(),
        auction_act.key.as_ref(),
        bidder_act.key.as_ref(),
    ];

    // Derive pot key, confirm it matches the users sent pot address.
    let (pot_key, bump) = Pubkey::find_program_address(&pot_path, program_id);
    if pot_key != *bidder_pot.key {
        return Ok(());
    }

    // TODO: deal with rent and balance correctly, do this properly.
    if bidder_act.lamports() - args.amount <= 0 {
        return Ok(());
    }

    // Transfer SOL from the bidder's SOL account into their pot.
    invoke_signed(
        &system_instruction::transfer(bidder_act.key, &pot_key, args.amount),
        &[bidder_act.clone(), bidder_pot.clone()],
        &[&pot_path],
    );

    // Load the auction and verify this bid is valid.
    let mut auction: AuctionData = try_from_slice_unchecked(&auction_act.data.borrow())?;

    // Make sure the auction hasn't ended. Hardcoded to 10 minutes.
    // TODO: Come back and make this configurable.
    let now = 0;
    auction.last_bid = match auction.last_bid {
        // Allow updating the time if 10 minutes has not passed.
        Some(time) if time - now < 10 * 60 => Some(now),
        // Allow the first bid when the auction has started.
        None if now < auction.start_time => Some(now),
        // Otherwise fail.
        _ => return Ok(()),
    };

    auction.bids.bid(Bid(pot_key, args.amount));
    auction.serialize(&mut *auction_act.data.borrow_mut())?;

    Ok(())
}

/// Cancels an existing bid. This only works in two cases:
///
/// 1) The auction is still going on, in which case it is possible to cancel a bid at any time.
/// 2) The auction has finished, but the bid did not win. This allows users to claim back their
///    funds from bid accounts.
fn cancel_bid(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_iter = &mut accounts.iter();
    let bidder_act = next_account_info(account_iter)?;
    let auction_act = next_account_info(account_iter)?;
    let bidder_pot = next_account_info(account_iter)?;

    // This  path references an account to store the users bid SOL in, if the user wins the auction
    // this is claimed by the auction authority, otherwise the user can request to have the SOL
    // sent back.
    let pot_path = [
        PREFIX.as_bytes(),
        program_id.as_ref(),
        auction_act.key.as_ref(),
        bidder_act.key.as_ref(),
    ];

    // Derive pot key, confirm it matches the users sent pot address.
    let (pot_key, bump) = Pubkey::find_program_address(&pot_path, program_id);
    if pot_key != *bidder_pot.key {
        return Ok(());
    }

    // Scan and remove the bid (Expensive, need a better datastructure).
    let mut auction: AuctionData = try_from_slice_unchecked(&auction_act.data.borrow())?;
    auction.bids.cancel_bid(pot_key);

    // Transfer SOL from the bidder's SOL account into their pot.
    invoke_signed(
        &system_instruction::transfer(&pot_key, bidder_act.key, bidder_pot.lamports()),
        &[bidder_pot.clone(), bidder_act.clone()],
        &[&pot_path],
    );

    // Write modified AuctionData.
    auction.serialize(&mut *auction_act.data.borrow_mut())?;

    Ok(())
}

/* -------------------------------------------------------------------------------- */

/// Arguments for the CreateAuction instruction discriminant.
#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct CreateAuctionArgs {
    /// The start time requested for this auction. See AuctionData.
    start_time: usize,
    /// How many winners are allowed for this auction. See AuctionData.
    max_winners: usize,
    /// The resource being auctioned. See AuctionData.
    resource: Pubkey,
}

/// Arguments for the PlaceBid instruction discriminant .
#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct PlaceBidArgs {
    /// Size of the bid being placed. The user must have enough SOL to satisfy this amount.
    pub amount: u64,
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct CancelBidArgs {}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum AuctionInstruction {
    CreateAuction(CreateAuctionArgs),
    PlaceBid(PlaceBidArgs),
    CancelBid(CancelBidArgs),
}

fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let account_iter = &mut accounts.iter();

    match AuctionInstruction::try_from_slice(input)? {
        AuctionInstruction::CreateAuction(args) => create_auction(program_id, accounts, args),
        AuctionInstruction::PlaceBid(args) => place_bid(program_id, accounts, args),
        AuctionInstruction::CancelBid(args) => cancel_bid(program_id, accounts),
    };

    Ok(())
}
