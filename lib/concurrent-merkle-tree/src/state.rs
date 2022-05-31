use solana_program::keccak::hashv;
use std::ops::Deref;
use std::ops::DerefMut;

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
    #[cfg(test)]
    pub fn default() -> Self {
        Self {
            root: EMPTY,
            path: [EMPTY; MAX_DEPTH],
            index: 0,
            _padding: 0,
        }
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

#[derive(Debug, Copy, Clone, Default, PartialEq)]

pub struct Node {
    pub inner: [u8; 32],
}

impl Node {
    pub fn new(inner: [u8; 32]) -> Self {
        Self { inner }
    }
}

impl Deref for Node {
    type Target = [u8; 32];
    fn deref(&self) -> &Self::Target {
        &self.inner
    }
}

impl DerefMut for Node {
    fn deref_mut(&mut self) -> &mut Self::Target {
        &mut self.inner
    }
}

impl AsRef<[u8; 32]> for Node {
    fn as_ref(&self) -> &[u8; 32] {
        &self.inner
    }
}

impl From<[u8; 32]> for Node {
    fn from(inner: [u8; 32]) -> Self {
        Self { inner }
    }
}

pub const EMPTY: Node = Node {
    inner: [0 as u8; 32],
};
