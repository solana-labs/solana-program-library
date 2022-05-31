use crate::state::node::Node;
use anchor_lang::{prelude::*, solana_program::keccak::hashv};
use borsh::BorshSerialize;
use concurrent_merkle_roll::state::ChangeLog;
use std::convert::AsRef;

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]
pub struct PathNode {
    pub node: Node,
    pub index: u32,
}

impl PathNode {
    pub fn new(node: Node, index: u32) -> Self {
        Self { node, index }
    }
}

#[event]
pub struct ChangeLogEvent {
    /// Public key of the Merkle Roll
    pub id: Pubkey,
    /// Nodes of off-chain merkle tree
    pub path: Vec<PathNode>,
    pub seq: u128,
    /// Bitmap of node parity (used when hashing)
    pub index: u32,
}

impl<const MAX_DEPTH: usize> ChangeLog<MAX_DEPTH> {
    pub fn to_event(&self, id: Pubkey, seq: u128) -> Box<ChangeLogEvent> {
        let path_len = self.path.len() as u32;
        let mut path: Vec<PathNode> = self
            .path
            .iter()
            .enumerate()
            .map(|(lvl, n)| PathNode::new(*n, (1 << (path_len - lvl as u32)) + (self.index >> lvl)))
            .collect();
        path.push(PathNode::new(self.root, 1));
        Box::new(ChangeLogEvent {
            id,
            path,
            seq,
            index: self.index,
        })
    }
}
