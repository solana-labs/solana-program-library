use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::pubkey::Pubkey,
};
/// prefix used for PDAs to avoid certain collision attacks (https://en.wikipedia.org/wiki/Collision_attack#Chosen-prefix_collision_attack)
pub const PREFIX: &str = "metadata";

pub const MAX_NAME_LENGTH: usize = 32;

pub const MAX_SYMBOL_LENGTH: usize = 10;

pub const MAX_URI_LENGTH: usize = 200;

pub const MAX_METADATA_LEN: usize = 1 + 32 + MAX_NAME_LENGTH + MAX_SYMBOL_LENGTH + MAX_URI_LENGTH;

pub const MAX_OWNER_LEN: usize = 1 + 32 + 32;

pub const METADATA_KEY: u8 = 0;

pub const NAME_SYMBOL_TUPLE_KEY: u8 = 1;

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
    /// Key for the front end denoting what type of struct this is -
    /// helpful for filtering on getProgramAccounts
    /// Always 0
    pub key: u8,
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
    /// Key for the front end denoting what type of struct this is -
    /// helpful for filtering on getProgramAccounts.
    /// Always 1
    pub key: u8,
    /// The person who can make updates to the metadata after it's made
    pub update_authority: Pubkey,
    /// Address of the current active metadata account
    pub metadata: Pubkey,
}

#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize)]
pub struct Edition {
    /// All Editions should never have a supply greater than 1.
    /// To enforce this, a transfer mint authority instruction will happen when
    /// a normal token is turned into an Edition, and in order for a Metadata update authority
    /// to do this transaction they will also need to sign the transaction as the Mint authority.
    ///
    /// If this is a master record, this is None, if this is not the master record,
    /// this will point back at the master record (Edition).
    master_record: Option<Pubkey>,

    /// Starting at 0 for master record, this is incremented for each edition minted.
    edition_count: u64,

    /// A new mint with supply of 1 is made for each edition. The mint on the master is the "master mint."
    mint: Pubkey,

    /// All editions point at the same Metadata, which presumably is owned by the Artist.
    /// This means if the Artists updates their Metadata or Royalty configs, all Limited or Open Edition holders
    /// Immediately inherit it.
    pub metadata: Pubkey,
}
