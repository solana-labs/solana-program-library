use {
    anchor_lang::{prelude::*, solana_program::keccak},
    gummyroll::Node
};

#[event]
pub struct LeafSchemaEvent {
    pub version: Version,
    pub schema: LeafSchema,
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

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]
pub enum LeafSchema {
    V0 {
        id: Pubkey,
        owner: Pubkey,
        delegate: Pubkey,
        nonce: u64,
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
    },
}

impl Default for LeafSchema {
    fn default() -> Self {
        Self::V0 {
            id: Default::default(),
            owner: Default::default(),
            delegate: Default::default(),
            nonce: 0,
            data_hash: [0; 32],
            creator_hash: [0; 32],
        }
    }
}

impl LeafSchema {
    pub fn new_v0(
        id: Pubkey,
        owner: Pubkey,
        delegate: Pubkey,
        nonce: u64,
        data_hash: [u8; 32],
        creator_hash: [u8; 32],
    ) -> Self {
        Self::V0 {
            id,
            owner,
            delegate,
            nonce,
            data_hash,
            creator_hash,
        }
    }

    pub fn version(&self) -> Version {
        match self {
            LeafSchema::V0 {
                ..
            } => Version::V0
        }
    }

    pub fn nonce(&self) -> u64 {
        match self {
            LeafSchema::V0 {
                nonce,
                ..
            } => *nonce
        }
    }

    pub fn to_event(&self) -> LeafSchemaEvent {
        LeafSchemaEvent {
            version: self.version(),
            schema: *self,
            leaf_hash: self.to_node(),
        }
    }

    pub fn to_node(&self) -> Node {
        let hashed_leaf = match self {
            LeafSchema::V0 {
                id,
                owner,
                delegate,
                nonce,
                data_hash,
                creator_hash,
            } => {
                keccak::hashv(&[
                    &[self.version().to_bytes()],
                    id.as_ref(),
                    owner.as_ref(),
                    delegate.as_ref(),
                    nonce.to_le_bytes().as_ref(),
                    data_hash.as_ref(),
                    creator_hash.as_ref(),
                ])
                    .to_bytes()
            }
        };
        hashed_leaf
    }
}
