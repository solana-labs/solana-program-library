use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::pubkey::Pubkey;
///prefix
pub const PREFIX: &str = "metadata";

/// max name length
pub const NAME_LENGTH: usize = 32;

/// max symbol length
pub const SYMBOL_LENGTH: usize = 10;

/// max uri length
pub const URI_LENGTH: usize = 200;

/// Max len of metadata
pub const METADATA_LEN: usize = 32 + NAME_LENGTH + SYMBOL_LENGTH + URI_LENGTH + 200;

/// Max len of owner
pub const OWNER_LEN: usize = 32 + 32 + 200;

/// Metadata
#[repr(C)]
#[derive(Clone, Default, BorshSerialize, BorshDeserialize)]
pub struct Metadata {
    /// Mint of the token asset
    pub mint: Pubkey,
    /// The name of the asset
    pub name: String,
    /// The symbol for the asset, ie, AAPL or SHOES
    pub symbol: String,
    /// URI pointing to JSON representing the asset
    pub uri: String,
}

/// Metadata
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Owner {
    /// The person who can make updates to the metadata after it's made
    pub owner: Pubkey,
    /// Pointer to the metadata object for verification purposes
    pub metadata: Pubkey,
}
