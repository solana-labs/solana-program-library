use crate::state::node::Node;
use anchor_lang::{prelude::*, solana_program::keccak::hashv};
use borsh::BorshSerialize;
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

#[derive(Copy, Clone, PartialEq)]
/// Stores proof for a given Merkle root update
#[repr(C)]
pub struct ChangeLog<const MAX_DEPTH: usize> {
    /// Historical root value before Path was applied
    pub root: Node,
    /// Nodes of off-chain merkle tree
    pub path: [Node; MAX_DEPTH],
    /// Bitmap of node parity (used when hashing)
    pub index: u32,
    pub _padding: u32,
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

    pub fn get_leaf(&self) -> Node {
        self.path[0]
    }

    /// Sets all change log values from a leaf and valid proof
    pub fn recompute_path(&mut self, mut start: Node, proof: &[Node]) -> Node {
        self.path[0] = start;
        for (ix, s) in proof.iter().enumerate() {
            if self.index >> ix & 1 == 0 {
                let res = hashv(&[start.as_ref(), s.as_ref()]);
                start.copy_from_slice(res.as_ref());
            } else {
                let res = hashv(&[s.as_ref(), start.as_ref()]);
                start.copy_from_slice(res.as_ref());
            }
            if ix < MAX_DEPTH - 1 {
                self.path[ix + 1] = start;
            }
        }
        self.root = start;
        start
    }
}

#[derive(Copy, Clone, PartialEq)]
#[repr(C)]
pub struct Path<const MAX_DEPTH: usize> {
    pub proof: [Node; MAX_DEPTH],
    pub leaf: Node,
    pub index: u32,
    pub _padding: u32,
}

impl<const MAX_DEPTH: usize> Default for Path<MAX_DEPTH> {
    fn default() -> Self {
        Self {
            proof: [Node::default(); MAX_DEPTH],
            leaf: Node::default(),
            index: 0,
            _padding: 0,
        }
    }
}
