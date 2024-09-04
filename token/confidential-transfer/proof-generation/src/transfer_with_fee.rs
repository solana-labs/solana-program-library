use {
    crate::{
        encryption::{FeeCiphertext, TransferAmountCiphertext},
        errors::TokenProofGenerationError,
        try_combine_lo_hi_ciphertexts, try_combine_lo_hi_commitments, try_combine_lo_hi_openings,
        try_split_u64, TRANSFER_AMOUNT_HI_BITS, TRANSFER_AMOUNT_LO_BITS,
    },
    curve25519_dalek::scalar::Scalar,
    solana_zk_sdk::{
        encryption::{
            auth_encryption::{AeCiphertext, AeKey},
            elgamal::{ElGamalCiphertext, ElGamalKeypair, ElGamalPubkey},
            grouped_elgamal::GroupedElGamal,
            pedersen::{Pedersen, PedersenCommitment, PedersenOpening},
        },
        zk_elgamal_proof_program::proof_data::{
            BatchedGroupedCiphertext2HandlesValidityProofData,
            BatchedGroupedCiphertext3HandlesValidityProofData, BatchedRangeProofU256Data,
            CiphertextCommitmentEqualityProofData, PercentageWithCapProofData,
        },
    },
};

const MAX_FEE_BASIS_POINTS: u64 = 10_000;
const ONE_IN_BASIS_POINTS: u128 = MAX_FEE_BASIS_POINTS as u128;

const FEE_AMOUNT_LO_BITS: usize = 16;
const FEE_AMOUNT_HI_BITS: usize = 32;

const REMAINING_BALANCE_BIT_LENGTH: usize = 64;
const DELTA_BIT_LENGTH: usize = 48;

/// The proof data required for a confidential transfer instruction when the
/// mint is extended for fees
pub struct TransferWithFeeProofData {
    pub equality_proof_data: CiphertextCommitmentEqualityProofData,
    pub transfer_amount_ciphertext_validity_proof_data:
        BatchedGroupedCiphertext3HandlesValidityProofData,
    pub percentage_with_cap_proof_data: PercentageWithCapProofData,
    pub fee_ciphertext_validity_proof_data: BatchedGroupedCiphertext2HandlesValidityProofData,
    pub range_proof_data: BatchedRangeProofU256Data,
}

#[allow(clippy::too_many_arguments)]
pub fn transfer_with_fee_split_proof_data(
    current_available_balance: &ElGamalCiphertext,
    current_decryptable_available_balance: &AeCiphertext,
    transfer_amount: u64,
    source_elgamal_keypair: &ElGamalKeypair,
    aes_key: &AeKey,
    destination_elgamal_pubkey: &ElGamalPubkey,
    auditor_elgamal_pubkey: Option<&ElGamalPubkey>,
    withdraw_withheld_authority_elgamal_pubkey: &ElGamalPubkey,
    fee_rate_basis_points: u16,
    maximum_fee: u64,
) -> Result<TransferWithFeeProofData, TokenProofGenerationError> {
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

    #[allow(clippy::arithmetic_side_effects)]
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
    let transfer_amount_ciphertext_validity_proof_data =
        BatchedGroupedCiphertext3HandlesValidityProofData::new(
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

    // calculate fee
    let transfer_fee_basis_points = fee_rate_basis_points;
    let transfer_fee_maximum_fee = maximum_fee;
    let (raw_fee_amount, delta_fee) = calculate_fee(transfer_amount, transfer_fee_basis_points)
        .ok_or(TokenProofGenerationError::FeeCalculation)?;

    // if raw fee is greater than the maximum fee, then use the maximum fee for the
    // fee amount
    let fee_amount = std::cmp::min(transfer_fee_maximum_fee, raw_fee_amount);

    // split and encrypt fee
    let (fee_amount_lo, fee_amount_hi) = try_split_u64(fee_amount, FEE_AMOUNT_LO_BITS)
        .ok_or(TokenProofGenerationError::IllegalAmountBitLength)?;
    let (fee_ciphertext_lo, fee_opening_lo) = FeeCiphertext::new(
        fee_amount_lo,
        destination_elgamal_pubkey,
        withdraw_withheld_authority_elgamal_pubkey,
    );
    let (fee_ciphertext_hi, fee_opening_hi) = FeeCiphertext::new(
        fee_amount_hi,
        destination_elgamal_pubkey,
        withdraw_withheld_authority_elgamal_pubkey,
    );

    // create combined commitments and openings to be used to generate proofs
    let combined_transfer_amount_commitment = try_combine_lo_hi_commitments(
        transfer_amount_grouped_ciphertext_lo.get_commitment(),
        transfer_amount_grouped_ciphertext_hi.get_commitment(),
        TRANSFER_AMOUNT_LO_BITS,
    )
    .ok_or(TokenProofGenerationError::IllegalAmountBitLength)?;
    let combined_transfer_amount_opening = try_combine_lo_hi_openings(
        &transfer_amount_opening_lo,
        &transfer_amount_opening_hi,
        TRANSFER_AMOUNT_LO_BITS,
    )
    .ok_or(TokenProofGenerationError::IllegalAmountBitLength)?;

    let combined_fee_commitment = try_combine_lo_hi_commitments(
        fee_ciphertext_lo.get_commitment(),
        fee_ciphertext_hi.get_commitment(),
        FEE_AMOUNT_LO_BITS,
    )
    .ok_or(TokenProofGenerationError::IllegalAmountBitLength)?;
    let combined_fee_opening =
        try_combine_lo_hi_openings(&fee_opening_lo, &fee_opening_hi, FEE_AMOUNT_LO_BITS)
            .ok_or(TokenProofGenerationError::IllegalAmountBitLength)?;

    // compute claimed and real delta commitment
    let (claimed_commitment, claimed_opening) = Pedersen::new(delta_fee);
    let (delta_commitment, delta_opening) = compute_delta_commitment_and_opening(
        (
            &combined_transfer_amount_commitment,
            &combined_transfer_amount_opening,
        ),
        (&combined_fee_commitment, &combined_fee_opening),
        transfer_fee_basis_points,
    );

    // generate fee sigma proof
    let percentage_with_cap_proof_data = PercentageWithCapProofData::new(
        &combined_fee_commitment,
        &combined_fee_opening,
        fee_amount,
        &delta_commitment,
        &delta_opening,
        delta_fee,
        &claimed_commitment,
        &claimed_opening,
        transfer_fee_maximum_fee,
    )
    .map_err(TokenProofGenerationError::from)?;

    // encrypt the fee amount under the destination and withdraw withheld authority
    // ElGamal public key
    let fee_destination_withdraw_withheld_authority_ciphertext_lo = GroupedElGamal::encrypt_with(
        [
            destination_elgamal_pubkey,
            withdraw_withheld_authority_elgamal_pubkey,
        ],
        fee_amount_lo,
        &fee_opening_lo,
    );
    let fee_destination_withdraw_withheld_authority_ciphertext_hi = GroupedElGamal::encrypt_with(
        [
            destination_elgamal_pubkey,
            withdraw_withheld_authority_elgamal_pubkey,
        ],
        fee_amount_hi,
        &fee_opening_hi,
    );

    // generate fee ciphertext validity data
    let fee_ciphertext_validity_proof_data =
        BatchedGroupedCiphertext2HandlesValidityProofData::new(
            destination_elgamal_pubkey,
            withdraw_withheld_authority_elgamal_pubkey,
            &fee_destination_withdraw_withheld_authority_ciphertext_lo,
            &fee_destination_withdraw_withheld_authority_ciphertext_hi,
            fee_amount_lo,
            fee_amount_hi,
            &fee_opening_lo,
            &fee_opening_hi,
        )
        .map_err(TokenProofGenerationError::from)?;

    // generate range proof data
    let delta_fee_complement = MAX_FEE_BASIS_POINTS
        .checked_sub(delta_fee)
        .ok_or(TokenProofGenerationError::FeeCalculation)?;

    let max_fee_basis_points_commitment =
        Pedersen::with(MAX_FEE_BASIS_POINTS, &PedersenOpening::default());
    #[allow(clippy::arithmetic_side_effects)]
    let claimed_complement_commitment = max_fee_basis_points_commitment - claimed_commitment;
    #[allow(clippy::arithmetic_side_effects)]
    let claimed_complement_opening = PedersenOpening::default() - &claimed_opening;

    let range_proof_data = BatchedRangeProofU256Data::new(
        vec![
            &new_available_balance_commitment,
            transfer_amount_grouped_ciphertext_lo.get_commitment(),
            transfer_amount_grouped_ciphertext_hi.get_commitment(),
            &claimed_commitment,
            &claimed_complement_commitment,
            fee_ciphertext_lo.get_commitment(),
            fee_ciphertext_hi.get_commitment(),
        ],
        vec![
            new_decrypted_available_balance,
            transfer_amount_lo,
            transfer_amount_hi,
            delta_fee,
            delta_fee_complement,
            fee_amount_lo,
            fee_amount_hi,
        ],
        vec![
            REMAINING_BALANCE_BIT_LENGTH,
            TRANSFER_AMOUNT_LO_BITS,
            TRANSFER_AMOUNT_HI_BITS,
            DELTA_BIT_LENGTH,
            DELTA_BIT_LENGTH,
            FEE_AMOUNT_LO_BITS,
            FEE_AMOUNT_HI_BITS,
        ],
        vec![
            &new_source_opening,
            &transfer_amount_opening_lo,
            &transfer_amount_opening_hi,
            &claimed_opening,
            &claimed_complement_opening,
            &fee_opening_lo,
            &fee_opening_hi,
        ],
    )
    .map_err(TokenProofGenerationError::from)?;

    Ok(TransferWithFeeProofData {
        equality_proof_data,
        transfer_amount_ciphertext_validity_proof_data,
        percentage_with_cap_proof_data,
        fee_ciphertext_validity_proof_data,
        range_proof_data,
    })
}

fn calculate_fee(transfer_amount: u64, fee_rate_basis_points: u16) -> Option<(u64, u64)> {
    let numerator = (transfer_amount as u128).checked_mul(fee_rate_basis_points as u128)?;

    // Warning: Division may involve CPU opcodes that have variable execution times.
    // This non-constant-time execution of the fee calculation can theoretically
    // reveal information about the transfer amount. For transfers that involve
    // extremely sensitive data, additional care should be put into how the fees
    // are calculated.
    let fee = numerator
        .checked_add(ONE_IN_BASIS_POINTS)?
        .checked_sub(1)?
        .checked_div(ONE_IN_BASIS_POINTS)?;

    let delta_fee = fee
        .checked_mul(ONE_IN_BASIS_POINTS)?
        .checked_sub(numerator)?;

    Some((fee as u64, delta_fee as u64))
}

#[allow(clippy::arithmetic_side_effects)]
fn compute_delta_commitment_and_opening(
    (combined_commitment, combined_opening): (&PedersenCommitment, &PedersenOpening),
    (combined_fee_commitment, combined_fee_opening): (&PedersenCommitment, &PedersenOpening),
    fee_rate_basis_points: u16,
) -> (PedersenCommitment, PedersenOpening) {
    let fee_rate_scalar = Scalar::from(fee_rate_basis_points);
    let delta_commitment = combined_fee_commitment * Scalar::from(MAX_FEE_BASIS_POINTS)
        - combined_commitment * fee_rate_scalar;
    let delta_opening = combined_fee_opening * Scalar::from(MAX_FEE_BASIS_POINTS)
        - combined_opening * fee_rate_scalar;

    (delta_commitment, delta_opening)
}
