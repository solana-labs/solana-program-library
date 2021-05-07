use {
    borsh::{BorshDeserialize, BorshSerialize},
    solana_program::pubkey::Pubkey,
};

/// prefix used for PDAs to avoid certain collision attacks (https://en.wikipedia.org/wiki/Collision_attack#Chosen-prefix_collision_attack)
pub const METADATA_PREFIX: &str = "metadata";

pub const IDL_PREFIX: &str = "idl";

// Metadata size
pub const MAX_NAME_LENGTH: usize = 32;

pub const MAX_VALUE_LENGTH: usize = 256;

pub const METADATA_ENTRY_SIZE: usize = 1 + 32 + MAX_NAME_LENGTH + MAX_NAME_LENGTH + 32;

// Idl size
pub const MAX_URL_LENGTH: usize = 200;

pub const VERSIONED_IDL_SIZE: usize =
    1 + 32 + 8 + MAX_URL_LENGTH + MAX_URL_LENGTH + 1 + MAX_URL_LENGTH + 32;

#[repr(C)]
#[derive(BorshSerialize, BorshDeserialize, PartialEq, Debug, Clone)]
pub enum AccountType {
    MetadataPairV1,
    VersionedIdlV1,
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub enum SerializationMethod {
    Bincode,
    Borsh,
    Anchor,
    CustomLayoutUrl,
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct MetadataEntry {
    pub account_type: AccountType,
    pub program_id: Pubkey,
    pub name: String,
    pub value: String,
    pub update_authority: Pubkey,
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct VersionedIdl {
    pub account_type: AccountType,
    pub program_id: Pubkey,
    pub effective_slot: u64,
    pub idl_url: String,
    pub source_url: String,
    pub serialization: SerializationMethod,
    pub custom_layout_url: Option<String>,
    pub update_authority: Pubkey,
}
