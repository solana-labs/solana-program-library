use {
    crate::{
        encryption::MintAmountCiphertext, errors::TokenProofGenerationError,
        try_combine_lo_hi_ciphertexts, try_split_u64, CiphertextValidityProofWithAuditorCiphertext,
    },
    solana_zk_sdk::{
        encryption::{
            elgamal::{ElGamalCiphertext, ElGamalKeypair, ElGamalPubkey},
            pedersen::Pedersen,
        },
        zk_elgamal_proof_program::proof_data::{
            BatchedGroupedCiphertext3HandlesValidityProofData, BatchedRangeProofU128Data,
            CiphertextCommitmentEqualityProofData, ZkProofData,
        },
    },
};

const NEW_SUPPLY_BIT_LENGTH: usize = 64;
const MINT_AMOUNT_LO_BIT_LENGTH: usize = 16;
const MINT_AMOUNT_HI_BIT_LENGTH: usize = 32;
/// The padding bit length in range proofs to make the bit-length power-of-2
const RANGE_PROOF_PADDING_BIT_LENGTH: usize = 16;

/// The proof data required for a confidential mint instruction
pub struct MintProofData {
    pub equality_proof_data: CiphertextCommitmentEqualityProofData,
    pub ciphertext_validity_proof_data_with_ciphertext:
        CiphertextValidityProofWithAuditorCiphertext,
    pub range_proof_data: BatchedRangeProofU128Data,
}

pub fn mint_split_proof_data(
    current_supply_ciphertext: &ElGamalCiphertext,
    mint_amount: u64,
    current_supply: u64,
    supply_elgamal_keypair: &ElGamalKeypair,
    destination_elgamal_pubkey: &ElGamalPubkey,
    auditor_elgamal_pubkey: Option<&ElGamalPubkey>,
) -> Result<MintProofData, TokenProofGenerationError> {
    let default_auditor_pubkey = ElGamalPubkey::default();
    let auditor_elgamal_pubkey = auditor_elgamal_pubkey.unwrap_or(&default_auditor_pubkey);

    // split the mint amount into low and high bits
    let (mint_amount_lo, mint_amount_hi) = try_split_u64(mint_amount, MINT_AMOUNT_LO_BIT_LENGTH)
        .ok_or(TokenProofGenerationError::IllegalAmountBitLength)?;

    // encrypt the mint amount under the destination and auditor's ElGamal public
    // keys
    let (mint_amount_grouped_ciphertext_lo, mint_amount_opening_lo) = MintAmountCiphertext::new(
        mint_amount_lo,
        destination_elgamal_pubkey,
        auditor_elgamal_pubkey,
        supply_elgamal_keypair.pubkey(),
    );

    let (mint_amount_grouped_ciphertext_hi, mint_amount_opening_hi) = MintAmountCiphertext::new(
        mint_amount_hi,
        destination_elgamal_pubkey,
        auditor_elgamal_pubkey,
        supply_elgamal_keypair.pubkey(),
    );

    // compute the new supply ciphertext
    let mint_amount_ciphertext_supply_lo = mint_amount_grouped_ciphertext_lo
        .0
        .to_elgamal_ciphertext(2)
        .unwrap();
    let mint_amount_ciphertext_supply_hi = mint_amount_grouped_ciphertext_hi
        .0
        .to_elgamal_ciphertext(2)
        .unwrap();

    #[allow(clippy::arithmetic_side_effects)]
    let new_supply_ciphertext = current_supply_ciphertext
        + try_combine_lo_hi_ciphertexts(
            &mint_amount_ciphertext_supply_lo,
            &mint_amount_ciphertext_supply_hi,
            MINT_AMOUNT_LO_BIT_LENGTH,
        )
        .ok_or(TokenProofGenerationError::IllegalAmountBitLength)?;

    // compute the new supply
    let new_supply = current_supply
        .checked_add(mint_amount)
        .ok_or(TokenProofGenerationError::IllegalAmountBitLength)?;

    let (new_supply_commitment, new_supply_opening) = Pedersen::new(new_supply);

    // generate equality proof data
    let equality_proof_data = CiphertextCommitmentEqualityProofData::new(
        supply_elgamal_keypair,
        &new_supply_ciphertext,
        &new_supply_commitment,
        &new_supply_opening,
        new_supply,
    )
    .map_err(TokenProofGenerationError::from)?;

    // generate ciphertext validity proof data
    let ciphertext_validity_proof_data = BatchedGroupedCiphertext3HandlesValidityProofData::new(
        destination_elgamal_pubkey,
        auditor_elgamal_pubkey,
        supply_elgamal_keypair.pubkey(),
        &mint_amount_grouped_ciphertext_lo.0,
        &mint_amount_grouped_ciphertext_hi.0,
        mint_amount_lo,
        mint_amount_hi,
        &mint_amount_opening_lo,
        &mint_amount_opening_hi,
    )
    .map_err(TokenProofGenerationError::from)?;

    let mint_amount_auditor_ciphertext_lo = ciphertext_validity_proof_data
        .context_data()
        .grouped_ciphertext_lo
        .try_extract_ciphertext(2)
        .map_err(|_| TokenProofGenerationError::CiphertextExtraction)?;

    let mint_amount_auditor_ciphertext_hi = ciphertext_validity_proof_data
        .context_data()
        .grouped_ciphertext_hi
        .try_extract_ciphertext(2)
        .map_err(|_| TokenProofGenerationError::CiphertextExtraction)?;

    let ciphertext_validity_proof_data_with_ciphertext =
        CiphertextValidityProofWithAuditorCiphertext {
            proof_data: ciphertext_validity_proof_data,
            ciphertext_lo: mint_amount_auditor_ciphertext_lo,
            ciphertext_hi: mint_amount_auditor_ciphertext_hi,
        };

    // generate range proof data
    let (padding_commitment, padding_opening) = Pedersen::new(0_u64);
    let range_proof_data = BatchedRangeProofU128Data::new(
        vec![
            &new_supply_commitment,
            mint_amount_grouped_ciphertext_lo.get_commitment(),
            mint_amount_grouped_ciphertext_hi.get_commitment(),
            &padding_commitment,
        ],
        vec![new_supply, mint_amount_lo, mint_amount_hi, 0],
        vec![
            NEW_SUPPLY_BIT_LENGTH,
            MINT_AMOUNT_LO_BIT_LENGTH,
            MINT_AMOUNT_HI_BIT_LENGTH,
            RANGE_PROOF_PADDING_BIT_LENGTH,
        ],
        vec![
            &new_supply_opening,
            &mint_amount_opening_lo,
            &mint_amount_opening_hi,
            &padding_opening,
        ],
    )
    .map_err(TokenProofGenerationError::from)?;

    Ok(MintProofData {
        equality_proof_data,
        ciphertext_validity_proof_data_with_ciphertext,
        range_proof_data,
    })
}
