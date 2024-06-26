use {
    crate::{encryption::PodTransferAmountCiphertext, errors::TokenProofExtractionError},
    solana_zk_sdk::{
        encryption::pod::elgamal::{PodElGamalCiphertext, PodElGamalPubkey},
        zk_elgamal_proof_program::proof_data::{
            BatchedGroupedCiphertext3HandlesValidityProofContext, BatchedRangeProofContext,
            CiphertextCommitmentEqualityProofContext,
        },
    },
};
#[cfg(not(target_os = "solana"))]
use {
    crate::{
        encryption::TransferAmountCiphertext, errors::TokenProofGenerationError,
        try_combine_lo_hi_ciphertexts, try_split_u64, REMAINING_BALANCE_BIT_LENGTH,
        TRANSFER_AMOUNT_HI_BITS, TRANSFER_AMOUNT_LO_BITS,
    },
    solana_zk_sdk::{
        encryption::{
            auth_encryption::{AeCiphertext, AeKey},
            elgamal::{ElGamalCiphertext, ElGamalKeypair, ElGamalPubkey},
            pedersen::Pedersen,
        },
        zk_elgamal_proof_program::proof_data::{
            BatchedGroupedCiphertext3HandlesValidityProofData, BatchedRangeProofU128Data,
            CiphertextCommitmentEqualityProofData,
        },
    },
};

/// The transfer public keys associated with a transfer.
pub struct TransferPubkeys {
    /// Source ElGamal public key
    pub source: PodElGamalPubkey,
    /// Destination ElGamal public key
    pub destination: PodElGamalPubkey,
    /// Auditor ElGamal public key
    pub auditor: PodElGamalPubkey,
}

/// The proof context information needed to process a [Transfer] instruction.
pub struct TransferProofContext {
    /// Ciphertext containing the low 16 bits of the transafer amount
    pub ciphertext_lo: PodTransferAmountCiphertext,
    /// Ciphertext containing the high 32 bits of the transafer amount
    pub ciphertext_hi: PodTransferAmountCiphertext,
    /// The transfer public keys associated with a transfer
    pub transfer_pubkeys: TransferPubkeys,
    /// The new source available balance ciphertext
    pub new_source_ciphertext: PodElGamalCiphertext,
}

/// The padding bit length in range proofs that are used for a confidential
/// token transfer
#[cfg(not(target_os = "solana"))]
const RANGE_PROOF_PADDING_BIT_LENGTH: usize = 16;

#[cfg(not(target_os = "solana"))]
pub fn transfer_split_proof_data(
    current_available_balance: &ElGamalCiphertext,
    current_decryptable_available_balance: &AeCiphertext,
    transfer_amount: u64,
    source_elgamal_keypair: &ElGamalKeypair,
    aes_key: &AeKey,
    destination_elgamal_pubkey: &ElGamalPubkey,
    auditor_elgamal_pubkey: Option<&ElGamalPubkey>,
) -> Result<
    (
        CiphertextCommitmentEqualityProofData,
        BatchedGroupedCiphertext3HandlesValidityProofData,
        BatchedRangeProofU128Data,
    ),
    TokenProofGenerationError,
> {
    let default_auditor_pubkey = ElGamalPubkey::default();
    let auditor_elgamal_pubkey = auditor_elgamal_pubkey.unwrap_or(&default_auditor_pubkey);

    // Split the transfer amount into the low and high bit components
    let (transfer_amount_lo, transfer_amount_hi) =
        try_split_u64(transfer_amount, TRANSFER_AMOUNT_LO_BITS)
            .ok_or(TokenProofGenerationError::IllegalAmountBitLength)?;

    // Encrypt the `lo` and `hi` transfer amounts
    let (transfer_amount_grouped_ciphertext_lo, transfer_amount_opening_lo) =
        TransferAmountCiphertext::new(
            transfer_amount_lo,
            source_elgamal_keypair.pubkey(),
            destination_elgamal_pubkey,
            auditor_elgamal_pubkey,
        );

    let (transfer_amount_grouped_ciphertext_hi, transfer_amount_opening_hi) =
        TransferAmountCiphertext::new(
            transfer_amount_hi,
            source_elgamal_keypair.pubkey(),
            destination_elgamal_pubkey,
            auditor_elgamal_pubkey,
        );

    // Decrypt the current available balance at the source
    let current_decrypted_available_balance = current_decryptable_available_balance
        .decrypt(aes_key)
        .ok_or(TokenProofGenerationError::IllegalAmountBitLength)?;

    // Compute the remaining balance at the source
    let new_decrypted_available_balance = current_decrypted_available_balance
        .checked_sub(transfer_amount)
        .ok_or(TokenProofGenerationError::NotEnoughFunds)?;

    // Create a new Pedersen commitment for the remaining balance at the source
    let (new_available_balance_commitment, new_source_opening) =
        Pedersen::new(new_decrypted_available_balance);

    // Compute the remaining balance at the source as ElGamal ciphertexts
    let transfer_amount_source_ciphertext_lo = transfer_amount_grouped_ciphertext_lo
        .0
        .to_elgamal_ciphertext(0)
        .unwrap();
    let transfer_amount_source_ciphertext_hi = transfer_amount_grouped_ciphertext_hi
        .0
        .to_elgamal_ciphertext(0)
        .unwrap();

    let new_available_balance_ciphertext = current_available_balance
        - try_combine_lo_hi_ciphertexts(
            &transfer_amount_source_ciphertext_lo,
            &transfer_amount_source_ciphertext_hi,
            TRANSFER_AMOUNT_LO_BITS,
        )
        .ok_or(TokenProofGenerationError::IllegalAmountBitLength)?;

    // generate equality proof data
    let equality_proof_data = CiphertextCommitmentEqualityProofData::new(
        source_elgamal_keypair,
        &new_available_balance_ciphertext,
        &new_available_balance_commitment,
        &new_source_opening,
        new_decrypted_available_balance,
    )
    .map_err(TokenProofGenerationError::from)?;

    // generate ciphertext validity data
    let ciphertext_validity_proof_data = BatchedGroupedCiphertext3HandlesValidityProofData::new(
        source_elgamal_keypair.pubkey(),
        destination_elgamal_pubkey,
        auditor_elgamal_pubkey,
        &transfer_amount_grouped_ciphertext_lo.0,
        &transfer_amount_grouped_ciphertext_hi.0,
        transfer_amount_lo,
        transfer_amount_hi,
        &transfer_amount_opening_lo,
        &transfer_amount_opening_hi,
    )
    .map_err(TokenProofGenerationError::from)?;

    // generate range proof data
    let (padding_commitment, padding_opening) = Pedersen::new(0_u64);
    let range_proof_data = BatchedRangeProofU128Data::new(
        vec![
            &new_available_balance_commitment,
            transfer_amount_grouped_ciphertext_lo.get_commitment(),
            transfer_amount_grouped_ciphertext_hi.get_commitment(),
            &padding_commitment,
        ],
        vec![
            new_decrypted_available_balance,
            transfer_amount_lo,
            transfer_amount_hi,
            0,
        ],
        vec![
            REMAINING_BALANCE_BIT_LENGTH,
            TRANSFER_AMOUNT_LO_BITS,
            TRANSFER_AMOUNT_HI_BITS,
            RANGE_PROOF_PADDING_BIT_LENGTH,
        ],
        vec![
            &new_source_opening,
            &transfer_amount_opening_lo,
            &transfer_amount_opening_hi,
            &padding_opening,
        ],
    )
    .map_err(TokenProofGenerationError::from)?;

    Ok((
        equality_proof_data,
        ciphertext_validity_proof_data,
        range_proof_data,
    ))
}

impl TransferProofContext {
    pub fn verify_and_extract(
        equality_proof_context: &CiphertextCommitmentEqualityProofContext,
        ciphertext_validity_proof_context: &BatchedGroupedCiphertext3HandlesValidityProofContext,
        range_proof_context: &BatchedRangeProofContext,
    ) -> Result<Self, TokenProofExtractionError> {
        // The equality proof context consists of the source ElGamal public key, the new
        // source available balance ciphertext, and the new source available
        // commitment. The public key and ciphertext should be returned as parts
        // of `TransferProofContextInfo` and the commitment should be checked
        // with range proof for consistency.
        let CiphertextCommitmentEqualityProofContext {
            pubkey: source_pubkey_from_equality_proof,
            ciphertext: new_source_ciphertext,
            commitment: new_source_commitment,
        } = equality_proof_context;

        // The ciphertext validity proof context consists of the destination ElGamal
        // public key, auditor ElGamal public key, and the transfer amount
        // ciphertexts. All of these fields should be returned as part of
        // `TransferProofContextInfo`. In addition, the commitments pertaining
        // to the transfer amount ciphertexts should be checked with range proof for
        // consistency.
        let BatchedGroupedCiphertext3HandlesValidityProofContext {
            first_pubkey: source_pubkey_from_validity_proof,
            second_pubkey: destination_pubkey,
            third_pubkey: auditor_pubkey,
            grouped_ciphertext_lo: transfer_amount_ciphertext_lo,
            grouped_ciphertext_hi: transfer_amount_ciphertext_hi,
        } = ciphertext_validity_proof_context;

        // The range proof context consists of the Pedersen commitments and bit-lengths
        // for which the range proof is proved. The commitments must consist of
        // three commitments pertaining to the new source available balance, the
        // low bits of the transfer amount, and high bits of the transfer
        // amount. These commitments must be checked for bit lengths `64`, `16`,
        // and `32`.
        let BatchedRangeProofContext {
            commitments: range_proof_commitments,
            bit_lengths: range_proof_bit_lengths,
        } = range_proof_context;

        // check that the source pubkey is consistent between equality and ciphertext
        // validity proofs
        if source_pubkey_from_equality_proof != source_pubkey_from_validity_proof {
            return Err(TokenProofExtractionError::ElGamalPubkeyMismatch);
        }

        // check that the range proof was created for the correct set of Pedersen
        // commitments
        let transfer_amount_commitment_lo = transfer_amount_ciphertext_lo.extract_commitment();
        let transfer_amount_commitment_hi = transfer_amount_ciphertext_hi.extract_commitment();

        let expected_commitments = [
            *new_source_commitment,
            transfer_amount_commitment_lo,
            transfer_amount_commitment_hi,
        ];

        if !range_proof_commitments
            .iter()
            .zip(expected_commitments.iter())
            .all(|(proof_commitment, expected_commitment)| proof_commitment == expected_commitment)
        {
            return Err(TokenProofExtractionError::PedersenCommitmentMismatch);
        }

        // check that the range proof was created for the correct number of bits
        const REMAINING_BALANCE_BIT_LENGTH: u8 = 64;
        const TRANSFER_AMOUNT_LO_BIT_LENGTH: u8 = 16;
        const TRANSFER_AMOUNT_HI_BIT_LENGTH: u8 = 32;
        const PADDING_BIT_LENGTH: u8 = 16;
        let expected_bit_lengths = [
            REMAINING_BALANCE_BIT_LENGTH,
            TRANSFER_AMOUNT_LO_BIT_LENGTH,
            TRANSFER_AMOUNT_HI_BIT_LENGTH,
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

        let transfer_pubkeys = TransferPubkeys {
            source: *source_pubkey_from_equality_proof,
            destination: *destination_pubkey,
            auditor: *auditor_pubkey,
        };

        let context_info = TransferProofContext {
            ciphertext_lo: PodTransferAmountCiphertext(*transfer_amount_ciphertext_lo),
            ciphertext_hi: PodTransferAmountCiphertext(*transfer_amount_ciphertext_hi),
            transfer_pubkeys,
            new_source_ciphertext: *new_source_ciphertext,
        };

        Ok(context_info)
    }
}

#[cfg(test)]
mod tests {
    use {super::*, solana_zk_sdk::zk_elgamal_proof_program::proof_data::ZkProofData};

    #[test]
    fn test_transfer_correctness() {
        test_proof_validity(0, 0);
        test_proof_validity(1, 0);
        test_proof_validity(1, 1);
        test_proof_validity(65535, 65535); // 2^16 - 1
        test_proof_validity(65536, 65536); // 2^16
        test_proof_validity(281474976710655, 281474976710655); // 2^48 - 1
    }

    fn test_proof_validity(spendable_balance: u64, transfer_amount: u64) {
        let source_keypair = ElGamalKeypair::new_rand();

        let aes_key = AeKey::new_rand();

        let destination_keypair = ElGamalKeypair::new_rand();
        let destination_pubkey = destination_keypair.pubkey();

        let auditor_keypair = ElGamalKeypair::new_rand();
        let auditor_pubkey = auditor_keypair.pubkey();

        let spendable_ciphertext = source_keypair.pubkey().encrypt(spendable_balance);
        let decryptable_balance = aes_key.encrypt(spendable_balance);

        let (equality_proof_data, validity_proof_data, range_proof_data) =
            transfer_split_proof_data(
                &spendable_ciphertext,
                &decryptable_balance,
                transfer_amount,
                &source_keypair,
                &aes_key,
                destination_pubkey,
                Some(auditor_pubkey),
            )
            .unwrap();

        equality_proof_data.verify_proof().unwrap();
        validity_proof_data.verify_proof().unwrap();
        range_proof_data.verify_proof().unwrap();

        TransferProofContext::verify_and_extract(
            equality_proof_data.context_data(),
            validity_proof_data.context_data(),
            range_proof_data.context_data(),
        )
        .unwrap();
    }
}
