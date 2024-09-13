use {
    crate::{
        encryption::BurnAmountCiphertext, errors::TokenProofGenerationError,
        try_combine_lo_hi_ciphertexts, try_split_u64,
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

const REMAINING_BALANCE_BIT_LENGTH: usize = 64;
const BURN_AMOUNT_LO_BIT_LENGTH: usize = 16;
const BURN_AMOUNT_HI_BIT_LENGTH: usize = 32;
/// The padding bit length in range proofs to make the bit-length power-of-2
const RANGE_PROOF_PADDING_BIT_LENGTH: usize = 16;

/// The proof data required for a confidential burn instruction
pub struct BurnProofData {
    pub equality_proof_data: CiphertextCommitmentEqualityProofData,
    pub ciphertext_validity_proof_data: BatchedGroupedCiphertext3HandlesValidityProofData,
    pub range_proof_data: BatchedRangeProofU128Data,
}

pub fn burn_split_proof_data(
    current_available_balance_ciphertext: &ElGamalCiphertext,
    current_decryptable_available_balance: &AeCiphertext,
    burn_amount: u64,
    source_elgamal_keypair: &ElGamalKeypair,
    source_aes_key: &AeKey,
    auditor_elgamal_pubkey: &ElGamalPubkey,
    supply_elgamal_pubkey: &ElGamalPubkey,
) -> Result<BurnProofData, TokenProofGenerationError> {
    // split the burn amount into low and high bits
    let (burn_amount_lo, burn_amount_hi) = try_split_u64(burn_amount, BURN_AMOUNT_LO_BIT_LENGTH)
        .ok_or(TokenProofGenerationError::IllegalAmountBitLength)?;

    // encrypt the burn amount under the source and auditor's ElGamal public key
    let (burn_amount_ciphertext_lo, burn_amount_opening_lo) = BurnAmountCiphertext::new(
        burn_amount_lo,
        source_elgamal_keypair.pubkey(),
        auditor_elgamal_pubkey,
        supply_elgamal_pubkey,
    );

    let (burn_amount_ciphertext_hi, burn_amount_opening_hi) = BurnAmountCiphertext::new(
        burn_amount_hi,
        source_elgamal_keypair.pubkey(),
        auditor_elgamal_pubkey,
        supply_elgamal_pubkey,
    );

    // decrypt the current available balance at the source
    let current_decrypted_available_balance = current_decryptable_available_balance
        .decrypt(source_aes_key)
        .ok_or(TokenProofGenerationError::IllegalAmountBitLength)?;

    // compute the remaining balance ciphertext
    let burn_amount_ciphertext_source_lo = burn_amount_ciphertext_lo
        .0
        .to_elgamal_ciphertext(0)
        .unwrap();
    let burn_amount_ciphertext_source_hi = burn_amount_ciphertext_hi
        .0
        .to_elgamal_ciphertext(0)
        .unwrap();

    #[allow(clippy::arithmetic_side_effects)]
    let new_available_balance_ciphertext = current_available_balance_ciphertext
        - try_combine_lo_hi_ciphertexts(
            &burn_amount_ciphertext_source_lo,
            &burn_amount_ciphertext_source_hi,
            BURN_AMOUNT_LO_BIT_LENGTH,
        )
        .ok_or(TokenProofGenerationError::IllegalAmountBitLength)?;

    // compute the remaining balance at the source
    let remaining_balance = current_decrypted_available_balance
        .checked_sub(burn_amount)
        .ok_or(TokenProofGenerationError::NotEnoughFunds)?;

    let (new_available_balance_commitment, new_available_balance_opening) =
        Pedersen::new(remaining_balance);

    // generate equality proof data
    let equality_proof_data = CiphertextCommitmentEqualityProofData::new(
        source_elgamal_keypair,
        &new_available_balance_ciphertext,
        &new_available_balance_commitment,
        &new_available_balance_opening,
        remaining_balance,
    )
    .map_err(TokenProofGenerationError::from)?;

    // generate ciphertext validity data
    let ciphertext_validity_proof_data = BatchedGroupedCiphertext3HandlesValidityProofData::new(
        source_elgamal_keypair.pubkey(),
        auditor_elgamal_pubkey,
        supply_elgamal_pubkey,
        &burn_amount_ciphertext_lo.0,
        &burn_amount_ciphertext_hi.0,
        burn_amount_lo,
        burn_amount_hi,
        &burn_amount_opening_lo,
        &burn_amount_opening_hi,
    )
    .map_err(TokenProofGenerationError::from)?;

    // generate range proof data
    let (padding_commitment, padding_opening) = Pedersen::new(0_u64);
    let range_proof_data = BatchedRangeProofU128Data::new(
        vec![
            &new_available_balance_commitment,
            burn_amount_ciphertext_lo.get_commitment(),
            burn_amount_ciphertext_hi.get_commitment(),
            &padding_commitment,
        ],
        vec![remaining_balance, burn_amount_lo, burn_amount_hi, 0],
        vec![
            REMAINING_BALANCE_BIT_LENGTH,
            BURN_AMOUNT_LO_BIT_LENGTH,
            BURN_AMOUNT_HI_BIT_LENGTH,
            RANGE_PROOF_PADDING_BIT_LENGTH,
        ],
        vec![
            &new_available_balance_opening,
            &burn_amount_opening_lo,
            &burn_amount_opening_hi,
            &padding_opening,
        ],
    )
    .map_err(TokenProofGenerationError::from)?;

    Ok(BurnProofData {
        equality_proof_data,
        ciphertext_validity_proof_data,
        range_proof_data,
    })
}
