use thiserror::Error;

/// Concurrent merkle tree operation errors
#[derive(Error, Debug)]
pub enum ConcurrentMerkleTreeError {
    /// Received an index larger than the rightmost index
    #[error("Received an index larger than the rightmost index")]
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

    /// Root passed as argument cannot be found in stored changelog buffer
    #[error("Root not found in changelog buffer")]
    RootNotFound,

    /// Valid proof was passed to a leaf, but its value has changed since the proof was issued
    #[error(
        "Valid proof was passed to a leaf, but its value has changed since the proof was issued"
    )]
    LeafContentsModified,
}
