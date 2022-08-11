use crate::utils::hash_to_parent;

#[derive(Copy, Clone, Debug, PartialEq)]
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
    pub fn default() -> Self {
        Self {
            root: EMPTY,
            path: [EMPTY; MAX_DEPTH],
            index: 0,
            _padding: 0,
        }
    }

    pub fn new(root: Node, path: [Node; MAX_DEPTH], index: u32) -> Self {
        Self {
            root,
            path,
            index,
            _padding: 0,
        }
    }

    pub fn get_leaf(&self) -> Node {
        self.path[0]
    }

    /// Sets all change log values from a leaf and valid proof
    pub fn replace_and_recompute_path(
        &mut self,
        index: u32,
        mut node: Node,
        proof: &[Node],
    ) -> Node {
        self.index = index;
        for (i, sibling) in proof.iter().enumerate() {
            self.path[i] = node;
            hash_to_parent(&mut node, sibling, self.index >> i & 1 == 0);
        }
        self.root = node;
        node
    }

    pub fn update_proof_or_leaf(
        &self,
        leaf_index: u32,
        proof: &mut [Node; MAX_DEPTH],
        leaf: &mut Node,
    ) {
        let padding: usize = 32 - MAX_DEPTH;
        if leaf_index != self.index {
            // This bit math is used to identify which node in the proof
            // we need to swap for a corresponding node in a saved change log
            let common_path_len = ((leaf_index ^ self.index) << padding).leading_zeros() as usize;
            let critbit_index = (MAX_DEPTH - 1) - common_path_len;
            proof[critbit_index] = self.path[critbit_index];
        } else {
            *leaf = self.get_leaf();
        }
    }
}

#[derive(Copy, Clone, Debug, PartialEq)]
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

pub type Node = [u8; 32];
pub const EMPTY: Node = [0_u8; 32];
