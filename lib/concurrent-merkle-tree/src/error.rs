use thiserror::Error;

#[derive(Error, Debug)]
pub enum CMTError {
    #[error("Received an index larger than the rightmost index")]
    LeafIndexOutOfBounds,
    #[error("Invalid root recomputed from proof")]
    InvalidProof,
    #[error("Cannot append an empty node")]
    CannotAppendEmptyNode,
    #[error("Tree is full, cannot append")]
    TreeFull,
    #[error("Tree already initialized")]
    TreeAlreadyInitialized,
    #[error("Leaf was updated since proof was issued")]
    LeafAlreadyUpdated,
    #[error("Invalid number of bytes passed for node (expected 32 bytes)")]
    InvalidNodeByteLength,
    #[error("Root not found in changelog buffer")]
    RootNotFound,
}
