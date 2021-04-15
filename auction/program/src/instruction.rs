use borsh::{BorshDeserialize, BorshSerialize};
use crate::processor::{
    cancel_bid::CancelBidArgs,
    create_auction::CreateAuctionArgs,
    place_bid::PlaceBidArgs,
};

#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum AuctionInstruction {
    CreateAuction(CreateAuctionArgs),
    PlaceBid(PlaceBidArgs),
    CancelBid(CancelBidArgs),
}
