#[cfg(not(target_os = "solana"))]
use solana_zk_sdk::zk_elgamal_proof_program::errors::ProofGenerationError;
use thiserror::Error;

#[cfg(not(target_os = "solana"))]
#[derive(Error, Clone, Debug, Eq, PartialEq)]
pub enum TokenProofGenerationError {
    #[error("inner proof generation failed")]
    ProofGeneration(#[from] ProofGenerationError),
    #[error("not enough funds in account")]
    NotEnoughFunds,
    #[error("illegal amount bit length")]
    IllegalAmountBitLength,
}

#[derive(Error, Clone, Debug, Eq, PartialEq)]
pub enum TokenProofExtractionError {
    #[error("ElGamal pubkey mismatch")]
    ElGamalPubkeyMismatch,
    #[error("Pedersen commitment mismatch")]
    PedersenCommitmentMismatch,
    #[error("Range proof length mismatch")]
    RangeProofLengthMismatch,
}
