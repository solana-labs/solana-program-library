use {solana_zk_sdk::zk_elgamal_proof_program::errors::ProofGenerationError, thiserror::Error};

#[derive(Error, Clone, Debug, Eq, PartialEq)]
pub enum TokenProofGenerationError {
    #[error("inner proof generation failed")]
    ProofGeneration(#[from] ProofGenerationError),
    #[error("not enough funds in account")]
    NotEnoughFunds,
    #[error("illegal amount bit length")]
    IllegalAmountBitLength,
    #[error("fee calculation failed")]
    FeeCalculation,
}
