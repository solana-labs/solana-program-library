use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::pubkey::Pubkey,
};
/// prefix used for PDAs to avoid certain collision attacks
pub const PREFIX: &str = "metadata";

pub const NAME_LENGTH: usize = 32;

pub const SYMBOL_LENGTH: usize = 10;

pub const URI_LENGTH: usize = 200;

pub const METADATA_LEN: usize = 32 + NAME_LENGTH + SYMBOL_LENGTH + URI_LENGTH + 200;

pub const OWNER_LEN: usize = 32 + 32 + 200;

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

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Owner {
    /// The person who can make updates to the metadata after it's made
    pub owner: Pubkey,
    /// Address of the metadata account
    pub metadata: Pubkey,
}
