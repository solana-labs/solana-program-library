use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::pubkey::Pubkey,
};
/// prefix used for PDAs to avoid certain collision attacks (https://en.wikipedia.org/wiki/Collision_attack#Chosen-prefix_collision_attack)
pub const PREFIX: &str = "metadata";

/// Used in seeds to make Edition model pda address
pub const EDITION: &str = "edition";

pub const MAX_NAME_LENGTH: usize = 32;

pub const MAX_SYMBOL_LENGTH: usize = 10;

pub const MAX_URI_LENGTH: usize = 200;

pub const MAX_METADATA_LEN: usize = 1 + 32 + MAX_NAME_LENGTH + MAX_SYMBOL_LENGTH + MAX_URI_LENGTH;

pub const MAX_NAME_SYMBOL_LEN: usize = 1 + 32 + 32;

pub const MAX_EDITION_LEN: usize = 1 + 32 + 8;

pub const MAX_MASTER_EDITION_LEN: usize = 1 + 9 + 8 + 32;

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub enum Key {
    MetadataV1,
    NameSymbolTupleV1,
    EditionV1,
    MasterEditionV1,
}
#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
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
    pub key: Key,
    /// This key is only present when the Metadata is used for a name/symbol combo that
    /// can be duplicated. This means this name/symbol combo has no accompanying
    /// UpdateAuthority account, and so it's update_authority is stored here.
    pub non_unique_specific_update_authority: Option<Pubkey>,
    pub mint: Pubkey,
    pub data: Data,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct NameSymbolTuple {
    pub key: Key,
    /// The person who can make updates to the metadata after it's made
    pub update_authority: Pubkey,
    /// Address of the current active metadata account
    pub metadata: Pubkey,
}

/// Make a master struct
/// total_supply and supply
/// single action

#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct MasterEdition {
    pub key: Key,

    pub supply: u64,

    pub max_supply: Option<u64>,

    /// Can be used to mint tokens that give one-time permission to mint a single limited edition.
    pub master_mint: Pubkey,
}

#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshSerialize, BorshDeserialize)]
/// All Editions should never have a supply greater than 1.
/// To enforce this, a transfer mint authority instruction will happen when
/// a normal token is turned into an Edition, and in order for a Metadata update authority
/// to do this transaction they will also need to sign the transaction as the Mint authority.
pub struct Edition {
    pub key: Key,

    /// Points at MasterEdition struct
    pub parent: Pubkey,

    /// Starting at 0 for master record, this is incremented for each edition minted.
    pub edition: u64,
}
