use crate::{
    PREFIX,
    errors::AuctionError,
    utils::{assert_owned_by, create_or_allocate_account_raw},
};
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh::try_from_slice_unchecked,
    clock::UnixTimestamp,
    entrypoint::ProgramResult,
    program::invoke_signed,
    program_error::ProgramError,
    pubkey::Pubkey,
    system_instruction,
    sysvar::{clock::Clock, rent::Rent, Sysvar},
};
use std::mem;

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
struct Bid(Pubkey, u64);

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
enum BidState {
    Capped {
        bids: Vec<Bid>,
        max:  usize,
    },
    Open
}

/// Bidding Implementation, this stores only the current winning bids in the auction, pruning
/// cancelled and lost bids over time. Temporary bad list implementation, to replace with a ring
/// buffer.
impl BidState {
    fn new_capped(n: usize) -> Self {
        BidState::Capped {
            bids: vec![],
            max: n
        }
    }

    /// Push a new bid into the state, this succeeds only if the bid is larger than the current top
    /// winner stored.
    fn place_bid(&mut self, bid: Bid) -> Result<(), ProgramError> {
        match self {
            // In a capped auction, track the limited number of winners.
            BidState::Capped { ref mut bids, .. } => {
                match bids.last() {
                    Some(top) if top.1 < bid.1 => {
                        bids.retain(|b| b.0 != bid.0);
                        bids.push(bid);
                        Ok(())
                    }
                    _ => Err(AuctionError::BidTooSmall.into())
                }
            },

            // In an open auction, bidding simply succeeds.
            BidState::Open => {
                Ok(())
            }
        }
    }

    /// Cancels a bid, if the bid was a winning bid it is removed, if the bid is invalid the
    /// function simple no-ops.
    fn cancel_bid(&mut self, key: Pubkey) -> Result<(), ProgramError> {
        match self {
            BidState::Capped { ref mut bids, max } => {
                bids.retain(|b| b.0 != key);
                Ok(())
            }

            BidState::Open => {
                Ok(())
            }
        }
    }

    /// Check if a pubkey is currently a winner.
    fn is_winner(&self, key: Pubkey) -> bool {
        match self {
            BidState::Capped { ref bids, max } => {
                bids.iter().any(|bid| bid.0 == key)
            }

            BidState::Open => {
                true
            }
        }
    }
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum WinnerLimit {
    Unlimited,
    Capped(usize),
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
struct AuctionData {
    /// Pubkey of the authority with permission to modify this auction.
    authority: Pubkey,
    /// Auction Bids, each user may have one bid open at a time.
    bid_state: BidState,
    /// The time the last bid was placed, used to time auction ending.
    last_bid: Option<UnixTimestamp>,
    /// Pubkey of the resource being bid on.
    resource: Pubkey,
    /// Time the auction starts at, this may be changed only if the auction hasn't started.
    start_time: UnixTimestamp,
    /// End time is the cut-off point that the auction is forced to end by.
    end_time: Option<UnixTimestamp>,
    /// Gap time is the amount of time after the previous bid at which the auction ends. Going
    /// once, going twice, sold!
    gap_time: Option<UnixTimestamp>,
}

/* -------------------------------------------------------------------------------- */

/// Arguments for the CreateAuction instruction discriminant.
#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct CreateAuctionArgs {
    /// How many winners are allowed for this auction. See AuctionData.
    winners: WinnerLimit,
    /// The resource being auctioned. See AuctionData.
    resource: Pubkey,
    /// The start time requested for this auction. See AuctionData.
    start_time: UnixTimestamp,
    /// End time is the cut-off point that the auction is forced to end by. See AuctionData.
    end_time: Option<UnixTimestamp>,
    /// Gap time is how much time after the previous bid where the auction ends. See AuctionData.
    gap_time: Option<UnixTimestamp>,
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

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    match AuctionInstruction::try_from_slice(input)? {
        AuctionInstruction::CreateAuction(args) => create_auction(program_id, accounts, args),
        AuctionInstruction::PlaceBid(args) => place_bid(program_id, accounts, args),
        AuctionInstruction::CancelBid(args) => cancel_bid(program_id, accounts),
    }
}

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
    let rent_act = next_account_info(account_iter)?;
    let system_account = next_account_info(account_iter)?;

    let auction_path = [
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &auction_act.key.to_bytes(),
    ];

    // Derive the address we'll store the auction in, and confirm it matches what we expected the
    // user to provide.
    let (auction_key, bump) = Pubkey::find_program_address(&auction_path, program_id);
    if auction_key != *auction_act.key {
        return Err(AuctionError::InvalidAuctionAccount.into());
    }

    let auction_seeds = [
        PREFIX.as_bytes(),
        program_id.as_ref(),
        &auction_act.key.to_bytes(),
        &[bump],
    ];

    // The data must be large enough to hold at least the number of winners.
    let auction_size = match args.winners {
        WinnerLimit::Capped(n) => mem::size_of::<Bid>() * n + 128,
        WinnerLimit::Unlimited => 0,
    };

    let bid_state = match args.winners {
        WinnerLimit::Capped(n) => BidState::new_capped(n),
        WinnerLimit::Unlimited => BidState::new_capped(0),
    };

    // Create auction account with enough space for a winner ringbuffer of size n.
    create_or_allocate_account_raw(
        *program_id,
        auction_act,
        rent_act,
        system_account,
        creator_act,
        auction_size,
        &auction_seeds,
    )?;

    let mut auction: AuctionData = try_from_slice_unchecked(
        &auction_act.data.borrow_mut()
    )?;

    // Configure Auction.
    auction.authority = *creator_act.key;
    auction.bid_state = bid_state;
    auction.last_bid = None;
    auction.resource = args.resource;
    auction.start_time = args.start_time;
    auction.end_time = args.end_time;
    auction.gap_time = args.gap_time;
    auction.serialize(&mut *auction_act.data.borrow_mut())?;

    Ok(())
}

/* -------------------------------------------------------------------------------- */

/// Models a set of metadata for a bidder, meant to be stored in a PDA. This allows looking up
/// information about a bidder regardless of if they have won, lost or cancelled.
struct BidderMetadata {
    /// Tracks the last time this user bid.
    last_bid: UnixTimestamp,
    /// Foreign Key. A reference to the auction this bid was placed on.
    auction: Pubkey,
}

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
    let bidder_pot_act = next_account_info(account_iter)?;
    let rent_act = next_account_info(account_iter)?;
    let system_account = next_account_info(account_iter)?;
    let clock_sysvar = next_account_info(account_iter)?;

    // Use the clock sysvar for timing the auction.
    let clock = Clock::from_account_info(clock_sysvar)?;

    // This path references an account to store the users bid SOL in, if the user wins the auction
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
    if pot_key != *bidder_pot_act.key {
        return Err(AuctionError::InvalidBidAccount.into());
    }

    // TODO: deal with rent and balance correctly, do this properly.
    if bidder_act.lamports() - args.amount <= 0 {
        return Err(AuctionError::BalanceTooLow.into());
    }

    // Pot path including the bump for seeds.
    let pot_seeds = [
        PREFIX.as_bytes(),
        program_id.as_ref(),
        auction_act.key.as_ref(),
        bidder_act.key.as_ref(),
        &[bump],
    ];

    // Allocate bid account, a token account to hold the resources.
    if false /* check account doesn't exist already */ {
        create_or_allocate_account_raw(
            *program_id,
            bidder_pot_act,
            rent_act,
            system_account,
            bidder_act,
            0,
            &pot_seeds,
        )?;
    }

    // Transfer SOL from the bidder's SOL account into their pot.
    invoke_signed(
        &system_instruction::transfer(bidder_act.key, &pot_key, args.amount),
        &[bidder_act.clone(), bidder_pot_act.clone()],
        &[&pot_seeds],
    )?;

    // Allocate a metadata account, to track the users state over time.
    if false /* check account doesn't exist already */ {
        create_or_allocate_account_raw(
            *program_id,
            bidder_pot_act,
            rent_act,
            system_account,
            bidder_act,
            mem::size_of::<BidderMetadata>(),
            &pot_seeds,
        )?;
    }

    // Load the auction and verify this bid is valid.
    let mut auction: AuctionData = try_from_slice_unchecked(&auction_act.data.borrow())?;

    // Make sure the auction hasn't ended. Hardcoded to 10 minutes.
    // TODO: Come back and make this configurable.
    let now = clock.unix_timestamp;
    auction.last_bid = match auction.last_bid {
        // Allow updating the time if 10 minutes has not passed.
        Some(time) if time - now < 10 * 60 => Some(now),
        // Allow the first bid when the auction has started.
        None if now < auction.start_time => Some(now),
        // Otherwise fail.
        _ => return Err(AuctionError::InvalidState.into()),
    };

    auction.bid_state.place_bid(Bid(pot_key, args.amount))?;
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
        return Err(AuctionError::InvalidBidAccount.into());
    }

    // Scan and remove the bid (Expensive, need a better datastructure).
    let mut auction: AuctionData = try_from_slice_unchecked(&auction_act.data.borrow())?;
    auction.bid_state.cancel_bid(pot_key)?;

    // Transfer SOL from the bidder's SOL account into their pot.
    invoke_signed(
        &system_instruction::transfer(&pot_key, bidder_act.key, bidder_pot.lamports()),
        &[bidder_pot.clone(), bidder_act.clone()],
        &[&pot_path],
    )?;

    // Write modified AuctionData.
    auction.serialize(&mut *auction_act.data.borrow_mut())?;

    Ok(())
}
