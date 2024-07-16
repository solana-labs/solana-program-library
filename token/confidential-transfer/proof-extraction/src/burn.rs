use {
    crate::{encryption::PodBurnAmountCiphertext, errors::TokenProofExtractionError},
    solana_zk_sdk::{
        encryption::pod::elgamal::{PodElGamalCiphertext, PodElGamalPubkey},
        zk_elgamal_proof_program::proof_data::{
            BatchedRangeProofContext, CiphertextCommitmentEqualityProofContext,
            GroupedCiphertext2HandlesValidityProofContext,
        },
    },
};

/// The public keys associated with a confidential burn
pub struct BurnPubkeys {
    pub source: PodElGamalPubkey,
    pub auditor: PodElGamalPubkey,
}

/// The proof context information needed to process a confidential burn instruction
pub struct BurnProofContext {
    pub burn_amount_ciphertext: PodBurnAmountCiphertext,
    pub burn_pubkeys: BurnPubkeys,
    pub remaining_balance_ciphertext: PodElGamalCiphertext,
}

impl BurnProofContext {
    pub fn verify_and_extract(
        equality_proof_context: &CiphertextCommitmentEqualityProofContext,
        ciphertext_validity_proof_context: &GroupedCiphertext2HandlesValidityProofContext,
        range_proof_context: &BatchedRangeProofContext,
    ) -> Result<Self, TokenProofExtractionError> {
        let CiphertextCommitmentEqualityProofContext {
            pubkey: source_elgamal_pubkey_from_equality_proof,
            ciphertext: remaining_balance_ciphertext,
            commitment: remaining_balance_commitment,
        } = equality_proof_context;

        let GroupedCiphertext2HandlesValidityProofContext {
            first_pubkey: source_elgamal_pubkey_from_validity_proof,
            second_pubkey: auditor_elgamal_pubkey,
            grouped_ciphertext: burn_amount_ciphertext,
        } = ciphertext_validity_proof_context;

        let BatchedRangeProofContext {
            commitments,
            bit_lengths,
        } = range_proof_context;

        if source_elgamal_pubkey_from_equality_proof == source_elgamal_pubkey_from_validity_proof {
            return Err(TokenProofExtractionError::ElGamalPubkeyMismatch);
        }

        if commitments.is_empty() || commitments[0] != *remaining_balance_commitment {
            return Err(TokenProofExtractionError::PedersenCommitmentMismatch);
        }

        if bit_lengths.is_empty() || bit_lengths[0] != 64 {
            return Err(TokenProofExtractionError::RangeProofLengthMismatch);
        }

        let burn_pubkeys = BurnPubkeys {
            source: *source_elgamal_pubkey_from_equality_proof,
            auditor: *auditor_elgamal_pubkey,
        };

        Ok(BurnProofContext {
            burn_amount_ciphertext: PodBurnAmountCiphertext(*burn_amount_ciphertext),
            burn_pubkeys,
            remaining_balance_ciphertext: *remaining_balance_ciphertext,
        })
    }
}
