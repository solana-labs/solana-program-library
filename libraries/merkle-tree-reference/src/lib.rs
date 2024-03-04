#![allow(clippy::arithmetic_side_effects)]
use {
    solana_program::keccak::hashv,
    std::{cell::RefCell, collections::VecDeque, iter::FromIterator, rc::Rc},
};

pub type Node = [u8; 32];
pub const EMPTY: Node = [0; 32];

/// Max number of concurrent changes to tree supported before having to
/// regenerate proofs
pub const MAX_SIZE: usize = 64;

/// Max depth of the Merkle tree
pub const MAX_DEPTH: usize = 14;

/// Used for node parity when hashing
pub const MASK: usize = MAX_SIZE - 1;

/// Recomputes root of the Merkle tree from Node & proof
pub fn recompute(mut leaf: Node, proof: &[Node], index: u32) -> Node {
    for (i, s) in proof.iter().enumerate() {
        if index >> i & 1 == 0 {
            let res = hashv(&[&leaf, s.as_ref()]);
            leaf.copy_from_slice(res.as_ref());
        } else {
            let res = hashv(&[s.as_ref(), &leaf]);
            leaf.copy_from_slice(res.as_ref());
        }
    }
    leaf
}

// Off-chain implementation to keep track of nodes
pub struct MerkleTree {
    pub leaf_nodes: Vec<Rc<RefCell<TreeNode>>>,
    pub root: Node,
}

impl MerkleTree {
    /// Calculates updated root from the passed leaves
    pub fn new(leaves: &[Node]) -> Self {
        let mut leaf_nodes = vec![];
        for (i, node) in leaves.iter().enumerate() {
            let mut tree_node = TreeNode::new_empty(0, i as u128);
            tree_node.node = *node;
            leaf_nodes.push(Rc::new(RefCell::new(tree_node)));
        }
        let root = MerkleTree::build_root(leaf_nodes.as_slice());
        Self { leaf_nodes, root }
    }

    /// Builds root from stack of leaves
    pub fn build_root(leaves: &[Rc<RefCell<TreeNode>>]) -> Node {
        let mut tree = VecDeque::from_iter(leaves.iter().map(Rc::clone));
        let mut seq_num = leaves.len() as u128;
        while tree.len() > 1 {
            let left = tree.pop_front().unwrap();
            let level = left.borrow().level;
            let right = if level != tree[0].borrow().level {
                let node = Rc::new(RefCell::new(TreeNode::new_empty(level, seq_num)));
                seq_num += 1;
                node
            } else {
                tree.pop_front().unwrap()
            };
            let mut hashed_parent = EMPTY;

            hashed_parent
                .copy_from_slice(hashv(&[&left.borrow().node, &right.borrow().node]).as_ref());
            let parent = Rc::new(RefCell::new(TreeNode::new(
                hashed_parent,
                left.clone(),
                right.clone(),
                level + 1,
                seq_num,
            )));
            left.borrow_mut().assign_parent(parent.clone());
            right.borrow_mut().assign_parent(parent.clone());
            tree.push_back(parent);
            seq_num += 1;
        }

        let root = tree[0].borrow().node;
        root
    }

    /// Traverses TreeNodes upwards to root from a Leaf TreeNode
    /// hashing along the way
    pub fn get_proof_of_leaf(&self, idx: usize) -> Vec<Node> {
        let mut proof = vec![];
        let mut node = self.leaf_nodes[idx].clone();
        loop {
            let ref_node = node.clone();
            if ref_node.borrow().parent.is_none() {
                break;
            }
            let parent = ref_node.borrow().parent.as_ref().unwrap().clone();
            if parent.borrow().left.as_ref().unwrap().borrow().id == ref_node.borrow().id {
                proof.push(parent.borrow().right.as_ref().unwrap().borrow().node);
            } else {
                proof.push(parent.borrow().left.as_ref().unwrap().borrow().node);
            }
            node = parent;
        }
        proof
    }

    /// Updates root from an updated leaf node set at index: `idx`
    fn update_root_from_leaf(&mut self, leaf_idx: usize) {
        let mut node = self.leaf_nodes[leaf_idx].clone();
        loop {
            let ref_node = node.clone();
            if ref_node.borrow().parent.is_none() {
                self.root = ref_node.borrow().node;
                break;
            }
            let parent = ref_node.borrow().parent.as_ref().unwrap().clone();
            let hash = if parent.borrow().left.as_ref().unwrap().borrow().id == ref_node.borrow().id
            {
                hashv(&[
                    &ref_node.borrow().node,
                    &parent.borrow().right.as_ref().unwrap().borrow().node,
                ])
            } else {
                hashv(&[
                    &parent.borrow().left.as_ref().unwrap().borrow().node,
                    &ref_node.borrow().node,
                ])
            };
            node = parent;
            node.borrow_mut().node.copy_from_slice(hash.as_ref());
        }
    }

    pub fn get_node(&self, idx: usize) -> Node {
        self.leaf_nodes[idx].borrow().node
    }

    pub fn get_root(&self) -> Node {
        self.root
    }

    pub fn add_leaf(&mut self, leaf: Node, leaf_idx: usize) {
        self.leaf_nodes[leaf_idx].borrow_mut().node = leaf;
        self.update_root_from_leaf(leaf_idx)
    }

    pub fn remove_leaf(&mut self, leaf_idx: usize) {
        self.leaf_nodes[leaf_idx].borrow_mut().node = EMPTY;
        self.update_root_from_leaf(leaf_idx)
    }

    pub fn get_leaf(&self, leaf_idx: usize) -> Node {
        self.leaf_nodes[leaf_idx].borrow().node
    }
}

#[derive(Clone)]
pub struct TreeNode {
    pub node: Node,
    left: Option<Rc<RefCell<TreeNode>>>,
    right: Option<Rc<RefCell<TreeNode>>>,
    parent: Option<Rc<RefCell<TreeNode>>>,
    level: u32,
    /// ID needed to figure out whether we came from left or right child node
    /// when hashing path upwards
    id: u128,
}

impl TreeNode {
    pub fn new(
        node: Node,
        left: Rc<RefCell<TreeNode>>,
        right: Rc<RefCell<TreeNode>>,
        level: u32,
        id: u128,
    ) -> Self {
        Self {
            node,
            left: Some(left),
            right: Some(right),
            parent: None,
            level,
            id,
        }
    }

    pub fn new_empty(level: u32, id: u128) -> Self {
        Self {
            node: empty_node(level),
            left: None,
            right: None,
            parent: None,
            level,
            id,
        }
    }

    /// Allows to propagate parent assignment
    pub fn assign_parent(&mut self, parent: Rc<RefCell<TreeNode>>) {
        self.parent = Some(parent);
    }
}

/// Calculates hash of empty nodes up to level i
/// TODO: cache this
pub fn empty_node(level: u32) -> Node {
    let mut data = EMPTY;
    if level != 0 {
        let lower_empty = empty_node(level - 1);
        let hash = hashv(&[&lower_empty, &lower_empty]);
        data.copy_from_slice(hash.as_ref());
    }
    data
}
