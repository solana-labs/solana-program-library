use crate::errors::AuctionError;
use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo,
    clock::UnixTimestamp,
    entrypoint::ProgramResult,
    program_error::ProgramError,
    pubkey::Pubkey,
};

pub mod cancel_bid;
pub mod create_auction;
pub mod place_bid;
pub mod start_auction;

pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    use crate::instruction::AuctionInstruction;
    use create_auction::create_auction;
    use start_auction::start_auction;
    use place_bid::place_bid;
    use cancel_bid::cancel_bid;

    match AuctionInstruction::try_from_slice(input)? {
        AuctionInstruction::CreateAuction(args) => create_auction(program_id, accounts, args),
        AuctionInstruction::StartAuction(args) => start_auction(program_id, accounts, args),
        AuctionInstruction::PlaceBid(args) => place_bid(program_id, accounts, args),
        AuctionInstruction::CancelBid(args) => cancel_bid(program_id, accounts),
    }
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct AuctionData {
    /// Pubkey of the authority with permission to modify this auction.
    pub authority: Pubkey,
    /// Auction Bids, each user may have one bid open at a time.
    pub bid_state: BidState,
    /// The time the last bid was placed, used to time auction ending.
    pub last_bid: Option<UnixTimestamp>,
    /// Pubkey of the resource being bid on.
    pub resource: Pubkey,
    /// Whether or not the auction has started
    pub started: bool,
    /// End time is the cut-off point that the auction is forced to end by.
    pub end_time: Option<UnixTimestamp>,
    /// Gap time is the amount of time after the previous bid at which the auction ends. Going
    /// once, going twice, sold!
    pub gap_time: Option<UnixTimestamp>,
}


/// Bids associate a bidding key with an amount bid.
#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub struct Bid(Pubkey, u64);

/// BidState tracks the running state of an auction, each variant represents a different kind of
/// auction being run.
#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum BidState {
    EnglishAuction { bids: Vec<Bid>, max: usize },
    OpenEdition,
}

/// Bidding Implementations.
///
/// English Auction: this stores only the current winning bids in the auction, pruning cancelled
/// and lost bids over time.
///
/// Open Edition: All bids are accepted, cancellations return money to the bidder and always
/// succeed.
impl BidState {
    fn new_english(n: usize) -> Self {
        BidState::EnglishAuction {
            bids: vec![],
            max: n,
        }
    }

    fn new_open_edition() -> Self {
        BidState::OpenEdition
    }

    /// Push a new bid into the state, this succeeds only if the bid is larger than the current top
    /// winner stored. Crappy list information to start with.
    fn place_bid(&mut self, bid: Bid) -> Result<(), ProgramError> {
        match self {
            // In a capped auction, track the limited number of winners.
            BidState::EnglishAuction { ref mut bids, max } => match bids.last() {
                Some(top) if top.1 < bid.1 => {
                    bids.retain(|b| b.0 != bid.0);
                    bids.push(bid);
                    if bids.len() > *max {
                        bids.remove(0);
                    }
                    Ok(())
                }
                _ => Err(AuctionError::BidTooSmall.into()),
            },

            // In an open auction, bidding simply succeeds.
            BidState::OpenEdition => Ok(()),
        }
    }

    /// Cancels a bid, if the bid was a winning bid it is removed, if the bid is invalid the
    /// function simple no-ops.
    fn cancel_bid(&mut self, key: Pubkey) -> Result<(), ProgramError> {
        match self {
            BidState::EnglishAuction { ref mut bids, max } => {
                bids.retain(|b| b.0 != key);
                Ok(())
            }

            // In an open auction, cancelling simply succeeds. It's up to the manager of an auction
            // to decide what to do with open edition bids.
            BidState::OpenEdition => Ok(()),
        }
    }

    /// Check if a pubkey is currently a winner.
    fn is_winner(&self, key: Pubkey) -> bool {
        match self {
            // Presense in the winner list is enough to check win state.
            BidState::EnglishAuction { ref bids, max } => bids.iter().any(|bid| bid.0 == key),
            // There are no winners in an open edition, it is up to the auction manager to decide
            // what to do with open edition bids.
            BidState::OpenEdition => false,
        }
    }
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum WinnerLimit {
    Unlimited,
    Capped(usize),
}

/// Models a set of metadata for a bidder, meant to be stored in a PDA. This allows looking up
/// information about a bidder regardless of if they have won, lost or cancelled.
#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
struct BidderMetadata {
    // Relationship with the bidder who's metadata this covers.
    bidder_pubkey: Pubkey,
    // Relationship with the auction this bid was placed on.
    auction_pubkey: Pubkey,
    // Amount that the user bid.
    last_bid: u64,
    // Tracks the last time this user bid.
    last_bid_timestamp: UnixTimestamp,
    // Whether the last bid the user made was cancelled. This should also be enough to know if the
    // user is a winner, as if cancelled it implies previous bids were also cancelled.
    cancelled: bool,
}





