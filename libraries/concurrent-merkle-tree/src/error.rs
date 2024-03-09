use thiserror::Error;

/// Concurrent merkle tree operation errors
#[derive(Error, Debug, PartialEq, Eq)]
pub enum ConcurrentMerkleTreeError {
    /// Received an index larger than the rightmost index
    #[error("Received an index larger than the rightmost index, or greater than (1 << max_depth)")]
    LeafIndexOutOfBounds,

    /// Invalid root recomputed from proof
    #[error("Invalid root recomputed from proof")]
    InvalidProof,

    /// Node to append cannot be empty
    #[error("Cannot append an empty node")]
    CannotAppendEmptyNode,

    /// The tree is at capacity
    #[error("Tree is full, cannot append")]
    TreeFull,

    /// This tree has already been initialized
    #[error("Tree already initialized")]
    TreeAlreadyInitialized,

    /// This tree has not yet been initialized
    #[error("Tree needs to be initialized before using")]
    TreeNotInitialized,

    /// Root passed as argument cannot be found in stored changelog buffer
    #[error("Root not found in changelog buffer")]
    RootNotFound,

    /// The tree's current leaf value does not match the supplied proof's leaf
    /// value
    #[error("This tree's current leaf value does not match the supplied proof's leaf value")]
    LeafContentsModified,

    /// Tree has at least 1 non-EMPTY leaf
    #[error("Tree is not empty")]
    TreeNonEmpty,
}
