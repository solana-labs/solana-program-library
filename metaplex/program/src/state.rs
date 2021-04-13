use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::pubkey::Pubkey,
};
/// prefix used for PDAs to avoid certain collision attacks (https://en.wikipedia.org/wiki/Collision_attack#Chosen-prefix_collision_attack)
pub const PREFIX: &str = "metaplex";

pub const MAX_AUCTION_MANAGER_SIZE: usize = 1 + 32 + 32 + 32 + 1;
#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, PartialEq)]
pub enum Key {
    AuctionManagerV1,
}

/// An Auction Manager can support an auction that is an English auction and limited edition and open edition
/// all at once. Need to support all at once. We use u8 keys to point to safety deposit indices in Vault
/// as opposed to the pubkeys to save on space. Ordering of safety deposits is guaranteed fixed by vault
/// implementation.
#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct AuctionManager {
    pub key: Key,

    pub auction: Pubkey,

    pub vault: Pubkey,

    pub winners_eligible_for_open_edition: bool,

    /// The safety deposit box index in the vault containing the winning items, in order of place
    /// The same index can appear multiple times if that index contains n tokens for n appearances (this will be checked)
    pub winning_keys: Vec<u8>,

    /// The safety deposit box index in the vault containing the template for the limited edition
    pub limited_edition_key: Option<u8>,

    /// The safety deposit box index in the vault containing the template for the open edition
    pub open_edition_key: Option<u8>,

    /// How long open edition (if this auction manager has it) goes for
    pub open_edition_duration_slots: Option<u64>,

    /// How many limited editions will be minted - these go to the nth second, third, x place winners
    /// after winning keys are exhausted, minted off master record in limited edition key.
    pub limited_edition_count: Option<u64>,

    /// The reserve price for a bid to be considered a valid bid for redemption.
    /// The auction may allow bids to be placed underneath this but presenting that ticket to this
    /// manager will not allow any redemptions.
    pub reserve_price: Option<u64>,

    /// Setting this field disconnects the open edition's price from the bid. Any bid you submit, regardless
    /// of amount, charges you the same fixed price. NOTE: This field supersedes open_edition_reserve_price.
    pub open_edition_fixed_price: Option<u64>,

    /// Setting this field disconnects the open edition reserve price from the normal bid reserve price.
    /// This means that while you may need $120 bid at least to have a winning bid for a limited edition or winning item,
    /// if you lose both of those, you can redeem an open edition token for your lower bid amount, even 0$ if the auctioneer so chooses,
    /// as a token of appreciation for coming. This means different people will pay different amounts for the open edition based
    /// on what their bid ticket says, always above this reserve price, and this reserve price is separated from the main reserve price.
    pub open_edition_reserve_price: Option<u64>,
}
