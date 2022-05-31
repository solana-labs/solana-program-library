use crate::state::node::Node;
use anchor_lang::{prelude::*, solana_program::keccak::hashv};
use borsh::BorshSerialize;
use concurrent_merkle_tree::state::{ChangeLog, Node as TreeNode};
use std::convert::AsRef;

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]
pub struct PathNode {
    pub node: Node,
    pub index: u32,
}

impl PathNode {
    pub fn new(tree_node: TreeNode, index: u32) -> Self {
        Self {
            node: Node::from(tree_node),
            index,
        }
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
//  ChangeLog<MAX_DEPTH>
impl<const MAX_DEPTH: usize> From<(Box<ChangeLog<MAX_DEPTH>>, Pubkey, u128)>
    for Box<ChangeLogEvent>
{
    fn from(log_info: (Box<ChangeLog<MAX_DEPTH>>, Pubkey, u128)) -> Self {
        let (changelog, tree_id, seq) = log_info;
        let path_len = changelog.path.len() as u32;
        let mut path: Vec<PathNode> = changelog
            .path
            .iter()
            .enumerate()
            .map(|(lvl, n)| {
                PathNode::new(
                    *n,
                    (1 << (path_len - lvl as u32)) + (changelog.index >> lvl),
                )
            })
            .collect();
        path.push(PathNode::new(changelog.root, 1));
        Box::new(ChangeLogEvent {
            id: tree_id,
            path,
            seq,
            index: changelog.index,
        })
    }
}
