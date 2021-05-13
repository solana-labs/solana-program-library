use borsh::{BorshDeserialize, BorshSerialize};

/// prefix used for PDAs to avoid certain collision attacks (https://en.wikipedia.org/wiki/Collision_attack#Chosen-prefix_collision_attack)
pub const CLASS_PREFIX: &str = "program_metadata";

// Metadata size
pub const MAX_NAME_LENGTH: usize = 36;

pub const MAX_VALUE_LENGTH: usize = 256;

pub const METADATA_ENTRY_SIZE: usize = 1 + MAX_NAME_LENGTH + MAX_VALUE_LENGTH;

// Idl size
pub const MAX_URL_LENGTH: usize = 200;

pub const IDL_HASH_SIZE: usize = 32;

pub const VERSIONED_IDL_SIZE: usize =
    1 + 32 + MAX_URL_LENGTH + IDL_HASH_SIZE + MAX_URL_LENGTH + 1 + MAX_URL_LENGTH;

// sha256("SPL Name Service" + "_idl")
pub const IDL_HASHED_NAME: [u8; 32] = [
    57, 222, 41, 139, 11, 207, 178, 48, 116, 99, 94, 46, 189, 24, 76, 79, 93, 3, 125, 157, 240,
    173, 14, 162, 89, 247, 248, 16, 251, 82, 91, 136,
];

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
    pub name: String,
    pub value: String,
}

#[repr(C)]
#[derive(Clone, BorshSerialize, BorshDeserialize, Debug)]
pub struct VersionedIdl {
    pub account_type: AccountType,
    pub effective_slot: u64,
    pub idl_url: String,
    pub idl_hash: [u8; 32],
    pub source_url: String,
    pub serialization: SerializationMethod,
    pub custom_layout_url: Option<String>,
}
