use thiserror::Error;

#[derive(Error, Debug)]
pub enum CMTError {
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

    /// Invalid number of bytes passed for node (expected 32 bytes)
    #[error("Invalid number of bytes passed for node (expected 32 bytes)")]
    InvalidNodeByteLength,

    /// Fast forward error: we cannot find a valid point to fast-forward the current proof from
    #[error("Root not found in changelog buffer")]
    RootNotFound,

    /// Valid proof was passed to a leaf, but it's value has changed since the proof was issued
    #[error(
        "Valid proof was passed to a leaf, but it's value has changed since the proof was issued"
    )]
    LeafContentsModified,
}
