use anchor_lang::prelude::*;
use borsh::{BorshDeserialize, BorshSerialize};
use concurrent_merkle_tree::state::{ChangeLog, Node};

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]
pub struct PathNode {
    pub node: [u8; 32],
    pub index: u32,
}

impl PathNode {
    pub fn new(tree_node: Node, index: u32) -> Self {
        Self {
            node: tree_node.inner,
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
