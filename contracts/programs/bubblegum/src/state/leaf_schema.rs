use anchor_lang::{prelude::*, solana_program::keccak};
use gummyroll::state::node::Node;

#[event]
pub struct LeafSchemaEvent {
    pub version: Version,
    pub owner: Pubkey,
    pub delegate: Pubkey, // Defaults to owner
    pub nonce: u128,
    pub data_hash: [u8; 32],
    pub creator_hash: [u8; 32],
    pub leaf_hash: [u8; 32],
}
#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]

pub enum Version {
    V0,
}

impl Default for Version {
    fn default() -> Self {
        Version::V0
    }
}

impl Version {
    pub fn to_bytes(&self) -> u8 {
        match self {
            Version::V0 => 0,
        }
    }
}

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Default, Debug)]
pub struct LeafSchema {
    pub version: Version,
    pub owner: Pubkey,
    pub delegate: Pubkey, // Defaults to owner
    pub nonce: u128,
    pub data_hash: [u8; 32],
    pub creator_hash: [u8; 32],
}

impl LeafSchema {
    pub fn new(
        version: Version,
        owner: Pubkey,
        delegate: Pubkey,
        nonce: u128,
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
    ) -> Self {
        Self {
            version,
            owner,
            delegate,
            nonce,
            data_hash,
            creator_hash,
        }
    }

    pub fn to_event(&self) -> LeafSchemaEvent {
        LeafSchemaEvent {
            version: self.version,
            owner: self.owner,
            delegate: self.delegate,
            nonce: self.nonce,
            data_hash: self.data_hash,
            creator_hash: self.creator_hash,
            leaf_hash: self.to_node().inner,
        }
    }

    pub fn to_node(&self) -> Node {
        let hashed_leaf = keccak::hashv(&[
            &[self.version.to_bytes()],
            self.owner.as_ref(),
            self.delegate.as_ref(),
            self.nonce.to_le_bytes().as_ref(),
            self.data_hash.as_ref(),
            self.creator_hash.as_ref(),
        ])
        .to_bytes();
        Node::new(hashed_leaf)
    }
}
