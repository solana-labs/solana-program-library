pub mod leaf_schema;
pub mod metaplex_adapter;
pub mod metaplex_anchor;

use anchor_lang::prelude::*;
use leaf_schema::LeafSchema;
use leaf_schema::Version;
use metaplex_adapter::MetadataArgs;

#[account]
#[derive(Copy)]
pub struct Nonce {
    pub count: u64,
}

pub const NONCE_SIZE: usize = 8 + 8;
pub const VOUCHER_SIZE: usize = 8 + 1 + 32 + 32 + 32 + 8 + 32 + 32 + 4 + 32;
pub const VOUCHER_PREFIX: &str = "voucher";
pub const ASSET_PREFIX: &str = "asset";

#[account]
#[derive(Copy)]
pub struct Voucher {
    pub leaf_schema: LeafSchema,
    pub index: u32,
    pub merkle_slab: Pubkey,
}

impl Voucher {
    pub fn new(leaf_schema: LeafSchema, index: u32, merkle_slab: Pubkey) -> Self {
        Self {
            leaf_schema,
            index,
            merkle_slab,
        }
    }
}

#[event]
pub struct NewNFTEvent {
    pub version: Version,
    pub metadata: MetadataArgs,
    pub nonce: u64,
}

#[event]
pub struct NFTDecompressionEvent {
    pub version: Version,
    pub id: Pubkey,
    pub tree_id: Pubkey,
    pub nonce: u64,
}
