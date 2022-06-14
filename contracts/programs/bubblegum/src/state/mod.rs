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
    pub count: u128,
}

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
    pub nonce: u128,
}
