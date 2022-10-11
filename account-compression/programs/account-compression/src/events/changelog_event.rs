use crate::state::PathNode;

use anchor_lang::prelude::*;
use spl_concurrent_merkle_tree::changelog::ChangeLog;

#[derive(AnchorDeserialize, AnchorSerialize)]
#[repr(C)]
pub enum ChangeLogEvent {
    V1(ChangeLogEventV1),
}

#[derive(AnchorDeserialize, AnchorSerialize)]
pub struct ChangeLogEventV1 {
    /// Public key of the ConcurrentMerkleTree
    pub id: Pubkey,

    /// Nodes of off-chain merkle tree needed by indexer
    pub path: Vec<PathNode>,

    /// Index corresponding to the number of successful operations on this tree.
    /// Used by the off-chain indexer to figure out when there are gaps to be backfilled.
    pub seq: u64,

    /// Bitmap of node parity (used when hashing)
    pub index: u32,
}

impl ChangeLogEvent {
    pub fn new(id: Pubkey, path: Vec<PathNode>, seq: u64, index: u32) -> Self {
        Self::V1(ChangeLogEventV1 {
            id,
            path,
            seq,
            index,
        })
    }
}

impl<const MAX_DEPTH: usize> From<(Box<ChangeLog<MAX_DEPTH>>, Pubkey, u64)>
    for Box<ChangeLogEvent>
{
    fn from(log_info: (Box<ChangeLog<MAX_DEPTH>>, Pubkey, u64)) -> Self {
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
        Box::new(ChangeLogEvent::V1(ChangeLogEventV1 {
            id: tree_id,
            path,
            seq,
            index: changelog.index,
        }))
    }
}
