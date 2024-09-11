use {
    crate::{encryption::PodMintAmountCiphertext, errors::TokenProofExtractionError},
    solana_zk_sdk::{
        encryption::pod::elgamal::{PodElGamalCiphertext, PodElGamalPubkey},
        zk_elgamal_proof_program::proof_data::{
            BatchedGroupedCiphertext3HandlesValidityProofContext, BatchedRangeProofContext,
            CiphertextCommitmentEqualityProofContext,
        },
    },
};

/// The public keys associated with a confidential mint
pub struct MintPubkeys {
    pub destination: PodElGamalPubkey,
    pub auditor: PodElGamalPubkey,
    pub supply: PodElGamalPubkey,
}

/// The proof context information needed to process a confidential mint
/// instruction
pub struct MintProofContext {
    pub mint_amount_ciphertext_lo: PodMintAmountCiphertext,
    pub mint_amount_ciphertext_hi: PodMintAmountCiphertext,
    pub mint_pubkeys: MintPubkeys,
    pub new_supply_ciphertext: PodElGamalCiphertext,
}

impl MintProofContext {
    pub fn verify_and_extract(
        equality_proof_context: &CiphertextCommitmentEqualityProofContext,
        ciphertext_validity_proof_context: &BatchedGroupedCiphertext3HandlesValidityProofContext,
        range_proof_context: &BatchedRangeProofContext,
    ) -> Result<Self, TokenProofExtractionError> {
        // The equality proof context consists of the supply ElGamal public key, the new
        // supply ciphertext, and the new supply commitment. The supply ElGamal
        // public key should be checked with ciphertext validity proof for
        // consistency and the new supply commitment should be checked with
        // range proof for consistency. The new supply ciphertext should be
        // returned as part of `MintProofContext`.
        let CiphertextCommitmentEqualityProofContext {
            pubkey: supply_elgamal_pubkey_from_equality_proof,
            ciphertext: new_supply_ciphertext,
            commitment: new_supply_commitment,
        } = equality_proof_context;

        // The ciphertext validity proof context consists of the destination ElGamal
        // public key, the auditor ElGamal public key, and the grouped ElGamal
        // ciphertexts for the low and high bits of the mint amount. These
        // fields should be returned as part of `MintProofContext`.
        let BatchedGroupedCiphertext3HandlesValidityProofContext {
            first_pubkey: destination_elgamal_pubkey,
            second_pubkey: auditor_elgamal_pubkey,
            third_pubkey: supply_elgamal_pubkey_from_ciphertext_validity_proof,
            grouped_ciphertext_lo: mint_amount_ciphertext_lo,
            grouped_ciphertext_hi: mint_amount_ciphertext_hi,
        } = ciphertext_validity_proof_context;

        // The range proof context consists of the Pedersen commitments and bit-lengths
        // for which the range proof is proved. The commitments must consist of
        // two commitments pertaining to the the
        // low bits of the mint amount, and high bits of the mint
        // amount. These commitments must be checked for bit lengths `16` and
        // and `32`.
        let BatchedRangeProofContext {
            commitments: range_proof_commitments,
            bit_lengths: range_proof_bit_lengths,
        } = range_proof_context;

        // check that the supply pubkey is consistent between equality and ciphertext
        // validity proofs
        if supply_elgamal_pubkey_from_equality_proof
            != supply_elgamal_pubkey_from_ciphertext_validity_proof
        {
            return Err(TokenProofExtractionError::ElGamalPubkeyMismatch);
        }

        // check that the range proof was created for the correct set of Pedersen
        // commitments
        let mint_amount_commitment_lo = mint_amount_ciphertext_lo.extract_commitment();
        let mint_amount_commitment_hi = mint_amount_ciphertext_hi.extract_commitment();

        let expected_commitments = [
            *new_supply_commitment,
            mint_amount_commitment_lo,
            mint_amount_commitment_hi,
        ];

        if !range_proof_commitments
            .iter()
            .zip(expected_commitments.iter())
            .all(|(proof_commitment, expected_commitment)| proof_commitment == expected_commitment)
        {
            return Err(TokenProofExtractionError::PedersenCommitmentMismatch);
        }

        // check that the range proof was created for the correct number of bits
        const NEW_SUPPLY_BIT_LENGTH: u8 = 64;
        const MINT_AMOUNT_LO_BIT_LENGTH: u8 = 16;
        const MINT_AMOUNT_HI_BIT_LENGTH: u8 = 32;
        const PADDING_BIT_LENGTH: u8 = 16;
        let expected_bit_lengths = [
            NEW_SUPPLY_BIT_LENGTH,
            MINT_AMOUNT_LO_BIT_LENGTH,
            MINT_AMOUNT_HI_BIT_LENGTH,
            PADDING_BIT_LENGTH,
        ]
        .iter();

        if !range_proof_bit_lengths
            .iter()
            .zip(expected_bit_lengths)
            .all(|(proof_len, expected_len)| proof_len == expected_len)
        {
            return Err(TokenProofExtractionError::RangeProofLengthMismatch);
        }

        let mint_pubkeys = MintPubkeys {
            destination: *destination_elgamal_pubkey,
            auditor: *auditor_elgamal_pubkey,
            supply: *supply_elgamal_pubkey_from_equality_proof,
        };

        Ok(MintProofContext {
            mint_amount_ciphertext_lo: PodMintAmountCiphertext(*mint_amount_ciphertext_lo),
            mint_amount_ciphertext_hi: PodMintAmountCiphertext(*mint_amount_ciphertext_hi),
            mint_pubkeys,
            new_supply_ciphertext: *new_supply_ciphertext,
        })
    }
}
