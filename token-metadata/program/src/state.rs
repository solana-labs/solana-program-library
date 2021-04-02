use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::pubkey::Pubkey,
};
/// prefix used for PDAs to avoid certain collision attacks (https://en.wikipedia.org/wiki/Collision_attack#Chosen-prefix_collision_attack)
pub const PREFIX: &str = "metadata";

pub const MAX_NAME_LENGTH: usize = 32;

pub const MAX_SYMBOL_LENGTH: usize = 10;

pub const MAX_URI_LENGTH: usize = 200;

pub const MAX_METADATA_LEN: usize = 32 + MAX_NAME_LENGTH + MAX_SYMBOL_LENGTH + MAX_URI_LENGTH;

pub const MAX_OWNER_LEN: usize = 32 + 32;

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
/// Args for Create call
pub struct Data {
    /// The name of the asset
    pub name: String,
    /// The symbol for the asset
    pub symbol: String,
    /// URI pointing to JSON representing the asset
    pub uri: String,
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize)]
pub struct Metadata {
    /// This key is only present when the Metadata is used for a name/symbol combo that
    /// can be duplicated. This means this name/symbol combo has no accompanying
    /// UpdateAuthority account, and so it's update_authority is stored here.
    pub non_unique_specific_update_authority: Option<Pubkey>,
    pub mint: Pubkey,
    pub data: Data,
}

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct NameSymbolTuple {
    /// The person who can make updates to the metadata after it's made
    pub update_authority: Pubkey,
    /// Address of the current active metadata account
    pub metadata: Pubkey,
}
