//! State related to storing a buffer of Merkle tree roots on-chain.
//!
mod concurrent_merkle_tree_header;
mod path_node;

pub use concurrent_merkle_tree_header::ConcurrentMerkleTreeHeader;
pub use path_node::PathNode;
