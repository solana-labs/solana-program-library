use anchor_lang::prelude::*;
use spl_concurrent_merkle_tree::node::Node;

#[derive(AnchorDeserialize, AnchorSerialize, Clone, Copy, Debug)]
pub struct PathNode {
    pub node: [u8; 32],
    pub index: u32,
}

impl PathNode {
    pub fn new(tree_node: Node, index: u32) -> Self {
        Self {
            node: tree_node,
            index,
        }
    }
}
