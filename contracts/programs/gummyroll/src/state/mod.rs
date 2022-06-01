pub mod change_log;
pub mod merkle_roll;
pub mod node;

use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use concurrent_merkle_tree::state::{ChangeLog, Node as TreeNode};

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

#[derive(Debug, Copy, Clone, AnchorDeserialize, AnchorSerialize, Default, PartialEq)]
pub struct Node {
    pub inner: [u8; 32],
}
impl Node {
    pub fn new(inner: [u8; 32]) -> Self {
        Self { inner }
    }
}
impl From<TreeNode> for Node {
    fn from(tree_node: TreeNode) -> Self {
        Self {
            inner: tree_node.inner,
        }
    }
}
impl Into<TreeNode> for Node {
    fn into(self) -> TreeNode {
        TreeNode::new(self.inner)
    }
}

#[derive(BorshDeserialize, BorshSerialize)]
#[repr(C)]
pub struct MerkleRollHeader {
    pub max_buffer_size: u32,
    pub max_depth: u32,
    pub authority: Pubkey,
    pub append_authority: Pubkey,
}

impl MerkleRollHeader {
    pub fn initialize(
        &mut self,
        max_depth: u32,
        max_buffer_size: u32,
        authority: &Pubkey,
        append_authority: &Pubkey,
    ) {
        // Check header is empty
        assert_eq!(self.max_buffer_size, 0);
        assert_eq!(self.max_depth, 0);
        self.max_buffer_size = max_buffer_size;
        self.max_depth = max_depth;
        self.authority = *authority;
        self.append_authority = *append_authority;
    }
}
