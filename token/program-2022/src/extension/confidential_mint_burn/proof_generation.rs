#[cfg(not(target_os = "solana"))]
use crate::{
    error::TokenError, extension::confidential_transfer::processor::verify_and_split_deposit_amount,
};
#[cfg(not(target_os = "solana"))]
use solana_zk_token_sdk::{
    encryption::{grouped_elgamal::GroupedElGamal, pedersen::Pedersen},
    instruction::{BatchedGroupedCiphertext2HandlesValidityProofData, BatchedRangeProofU64Data},
};

#[cfg(not(target_os = "solana"))]
use solana_program::program_error::ProgramError;
#[cfg(not(target_os = "solana"))]
use solana_zk_token_sdk::encryption::{elgamal::ElGamalPubkey, pedersen::PedersenOpening};

/// Generates range proof for mint instruction
#[cfg(not(target_os = "solana"))]
pub fn generate_mint_proofs(
    amount: u64,
    destination_elgamal_pubkey: &ElGamalPubkey,
    auditor_elgamal_pubkey: &Option<ElGamalPubkey>,
) -> Result<
    (
        BatchedRangeProofU64Data,
        BatchedGroupedCiphertext2HandlesValidityProofData,
        (PedersenOpening, PedersenOpening),
    ),
    ProgramError,
> {
    let (amount_lo, amount_hi) = verify_and_split_deposit_amount(amount)?;
    let auditor_elgamal_pubkey = auditor_elgamal_pubkey.unwrap_or_default();

    const MINT_AMOUNT_LO_BIT_LENGTH: usize = 16;
    const MINT_AMOUNT_HI_BIT_LENGTH: usize = 32;
    const PADDING_BIT_LENGTH: usize = 16;

    // Encrypt the `lo` and `hi` transfer amounts.
    let mint_amount_opening_lo = PedersenOpening::new_rand();
    let mint_amount_grouped_ciphertext_lo = GroupedElGamal::<2>::encrypt_with(
        [destination_elgamal_pubkey, &auditor_elgamal_pubkey],
        amount_lo,
        &mint_amount_opening_lo,
    );

    let mint_amount_opening_hi = PedersenOpening::new_rand();
    let mint_amount_grouped_ciphertext_hi = GroupedElGamal::<2>::encrypt_with(
        [destination_elgamal_pubkey, &auditor_elgamal_pubkey],
        amount_hi,
        &mint_amount_opening_hi,
    );

    let (padding_commitment, padding_opening) = Pedersen::new(0_u64);

    Ok((
        BatchedRangeProofU64Data::new(
            vec![
                &mint_amount_grouped_ciphertext_lo.commitment,
                &mint_amount_grouped_ciphertext_hi.commitment,
                &padding_commitment,
            ],
            vec![amount_lo, amount_hi, 0],
            vec![
                MINT_AMOUNT_LO_BIT_LENGTH,
                MINT_AMOUNT_HI_BIT_LENGTH,
                PADDING_BIT_LENGTH,
            ],
            vec![
                &mint_amount_opening_lo,
                &mint_amount_opening_hi,
                &padding_opening,
            ],
        )
        .map_err(|_| TokenError::ProofGeneration)?,
        BatchedGroupedCiphertext2HandlesValidityProofData::new(
            destination_elgamal_pubkey,
            &auditor_elgamal_pubkey,
            &mint_amount_grouped_ciphertext_lo,
            &mint_amount_grouped_ciphertext_hi,
            amount_lo,
            amount_hi,
            &mint_amount_opening_lo,
            &mint_amount_opening_hi,
        )
        .map_err(|_| TokenError::ProofGeneration)?,
        (mint_amount_opening_lo, mint_amount_opening_hi),
    ))
}
