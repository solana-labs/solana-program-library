use {
    crate::errors::TokenProofExtractionError,
    solana_zk_sdk::{
        encryption::pod::elgamal::{PodElGamalCiphertext, PodElGamalPubkey},
        zk_elgamal_proof_program::proof_data::{
            BatchedRangeProofContext, CiphertextCommitmentEqualityProofContext,
        },
    },
};

const REMAINING_BALANCE_BIT_LENGTH: u8 = 64;

pub struct WithdrawProofContext {
    pub source_pubkey: PodElGamalPubkey,
    pub remaining_balance_ciphertext: PodElGamalCiphertext,
}

impl WithdrawProofContext {
    pub fn verify_and_extract(
        equality_proof_context: &CiphertextCommitmentEqualityProofContext,
        range_proof_context: &BatchedRangeProofContext,
    ) -> Result<Self, TokenProofExtractionError> {
        let CiphertextCommitmentEqualityProofContext {
            pubkey: source_pubkey,
            ciphertext: remaining_balance_ciphertext,
            commitment: remaining_balance_commitment,
        } = equality_proof_context;

        let BatchedRangeProofContext {
            commitments: range_proof_commitments,
            bit_lengths: range_proof_bit_lengths,
        } = range_proof_context;

        if range_proof_commitments.is_empty()
            || range_proof_commitments[0] != *remaining_balance_commitment
        {
            return Err(TokenProofExtractionError::PedersenCommitmentMismatch);
        }

        if range_proof_bit_lengths.is_empty()
            || range_proof_bit_lengths[0] != REMAINING_BALANCE_BIT_LENGTH
        {
            return Err(TokenProofExtractionError::RangeProofLengthMismatch);
        }

        let context_info = WithdrawProofContext {
            source_pubkey: *source_pubkey,
            remaining_balance_ciphertext: *remaining_balance_ciphertext,
        };

        Ok(context_info)
    }
}
