use anchor_lang::prelude::*;
use concurrent_merkle_tree::state::Node as TreeNode;

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
