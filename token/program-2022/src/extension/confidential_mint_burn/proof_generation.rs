#[cfg(not(target_os = "solana"))]
use solana_program::program_error::ProgramError;
#[cfg(not(target_os = "solana"))]
use solana_zk_sdk::encryption::{elgamal::ElGamalPubkey, pedersen::PedersenOpening};
#[cfg(not(target_os = "solana"))]
use solana_zk_sdk::{
    encryption::{
        auth_encryption::{AeCiphertext, AeKey},
        elgamal::ElGamalCiphertext,
        elgamal::ElGamalKeypair,
        grouped_elgamal::GroupedElGamal,
        pedersen::Pedersen,
    },
    zk_elgamal_proof_program::proof_data::{
        BatchedGroupedCiphertext3HandlesValidityProofData, BatchedRangeProofU128Data,
        BatchedRangeProofU64Data, CiphertextCommitmentEqualityProofData,
    },
};
#[cfg(not(target_os = "solana"))]
use {
    crate::{
        error::TokenError,
        extension::confidential_transfer::processor::verify_and_split_deposit_amount,
    },
    spl_token_confidential_transfer_ciphertext_arithmetic::subtract_with_lo_hi,
};

/// Generates proof data for mint instruction
#[cfg(not(target_os = "solana"))]
pub fn generate_mint_proofs(
    mint_amount: u64,
    destination_elgamal_pubkey: &ElGamalPubkey,
    auditor_elgamal_pubkey: &Option<ElGamalPubkey>,
    supply_elgamal_pubkey: &Option<ElGamalPubkey>,
) -> Result<
    (
        BatchedRangeProofU64Data,
        BatchedGroupedCiphertext3HandlesValidityProofData,
        (PedersenOpening, PedersenOpening),
    ),
    ProgramError,
> {
    let (amount_lo, amount_hi) = verify_and_split_deposit_amount(mint_amount)?;
    let auditor_elgamal_pubkey = auditor_elgamal_pubkey.unwrap_or_default();
    let supply_elgamal_pubkey = supply_elgamal_pubkey.unwrap_or_default();

    const MINT_AMOUNT_LO_BIT_LENGTH: usize = 16;
    const MINT_AMOUNT_HI_BIT_LENGTH: usize = 32;
    const PADDING_BIT_LENGTH: usize = 16;

    // Encrypt the `lo` and `hi` transfer amounts.
    let mint_amount_opening_lo = PedersenOpening::new_rand();
    let mint_amount_grouped_ciphertext_lo = GroupedElGamal::<3>::encrypt_with(
        [
            destination_elgamal_pubkey,
            &auditor_elgamal_pubkey,
            &supply_elgamal_pubkey,
        ],
        amount_lo,
        &mint_amount_opening_lo,
    );

    let mint_amount_opening_hi = PedersenOpening::new_rand();
    let mint_amount_grouped_ciphertext_hi = GroupedElGamal::<3>::encrypt_with(
        [
            destination_elgamal_pubkey,
            &auditor_elgamal_pubkey,
            &supply_elgamal_pubkey,
        ],
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
        BatchedGroupedCiphertext3HandlesValidityProofData::new(
            destination_elgamal_pubkey,
            &auditor_elgamal_pubkey,
            &supply_elgamal_pubkey,
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

/// Generates proof data for burn instruction
#[cfg(not(target_os = "solana"))]
#[allow(clippy::type_complexity)]
pub fn generate_burn_proofs(
    current_available_balance: &ElGamalCiphertext,
    current_decryptable_available_balance: &AeCiphertext,
    burn_amount: u64,
    source_elgamal_keypair: &ElGamalKeypair,
    aes_key: &AeKey,
    auditor_elgamal_pubkey: &Option<ElGamalPubkey>,
    supply_elgamal_pubkey: &Option<ElGamalPubkey>,
) -> Result<
    (
        CiphertextCommitmentEqualityProofData,
        BatchedGroupedCiphertext3HandlesValidityProofData,
        BatchedRangeProofU128Data,
        (PedersenOpening, PedersenOpening),
    ),
    TokenError,
> {
    let burner_elgamal_pubkey = source_elgamal_keypair.pubkey();
    let auditor_elgamal_pubkey = auditor_elgamal_pubkey.unwrap_or_default();
    let supply_elgamal_pubkey = supply_elgamal_pubkey.unwrap_or_default();

    // Split the transfer amount into the low and high bit components.
    let (burn_amount_lo, burn_amount_hi) = verify_and_split_deposit_amount(burn_amount)?;

    // Encrypt the `lo` and `hi` transfer amounts.
    let burn_amount_opening_lo = PedersenOpening::new_rand();
    let burn_amount_grouped_ciphertext_lo = GroupedElGamal::<3>::encrypt_with(
        [
            burner_elgamal_pubkey,
            &auditor_elgamal_pubkey,
            &supply_elgamal_pubkey,
        ],
        burn_amount_lo,
        &burn_amount_opening_lo,
    );

    let burn_amount_opening_hi = PedersenOpening::new_rand();
    let burn_amount_grouped_ciphertext_hi = GroupedElGamal::<3>::encrypt_with(
        [
            burner_elgamal_pubkey,
            &auditor_elgamal_pubkey,
            &supply_elgamal_pubkey,
        ],
        burn_amount_hi,
        &burn_amount_opening_hi,
    );

    // Decrypt the current available balance at the source
    let current_decrypted_available_balance = current_decryptable_available_balance
        .decrypt(aes_key)
        .ok_or(TokenError::AccountDecryption)?;

    // Compute the remaining balance at the source
    let new_decrypted_available_balance = current_decrypted_available_balance
        .checked_sub(burn_amount)
        .ok_or(TokenError::InsufficientFunds)?;

    // Create a new Pedersen commitment for the remaining balance at the source
    let (new_available_balance_commitment, new_source_opening) =
        Pedersen::new(new_decrypted_available_balance);

    // Compute the remaining balance at the source as ElGamal ciphertexts
    let transfer_amount_source_ciphertext_lo = burn_amount_grouped_ciphertext_lo
        .to_elgamal_ciphertext(0)
        .expect("ElGamalCiphertext for burn incorrectly generated");

    let transfer_amount_source_ciphertext_hi = burn_amount_grouped_ciphertext_hi
        .to_elgamal_ciphertext(0)
        .expect("ElGamalCiphertext for burn incorrectly generated");

    let current_available_balance = (*current_available_balance).into();
    let new_available_balance_ciphertext = subtract_with_lo_hi(
        &current_available_balance,
        &transfer_amount_source_ciphertext_lo.into(),
        &transfer_amount_source_ciphertext_hi.into(),
    )
    .ok_or(TokenError::CiphertextArithmeticFailed)?;
    let new_available_balance_ciphertext: ElGamalCiphertext = new_available_balance_ciphertext
        .try_into()
        .map_err(|_| TokenError::MalformedCiphertext)?;

    // generate equality proof data
    let equality_proof_data = CiphertextCommitmentEqualityProofData::new(
        source_elgamal_keypair,
        &new_available_balance_ciphertext,
        &new_available_balance_commitment,
        &new_source_opening,
        new_decrypted_available_balance,
    )
    .map_err(|_| TokenError::ProofGeneration)?;

    // generate ciphertext validity data
    let ciphertext_validity_proof_data = BatchedGroupedCiphertext3HandlesValidityProofData::new(
        burner_elgamal_pubkey,
        &auditor_elgamal_pubkey,
        &supply_elgamal_pubkey,
        &burn_amount_grouped_ciphertext_lo,
        &burn_amount_grouped_ciphertext_hi,
        burn_amount_lo,
        burn_amount_hi,
        &burn_amount_opening_lo,
        &burn_amount_opening_hi,
    )
    .map_err(|_| TokenError::ProofGeneration)?;

    // generate range proof data
    const REMAINING_BALANCE_BIT_LENGTH: usize = 64;
    const TRANSFER_AMOUNT_LO_BIT_LENGTH: usize = 16;
    const TRANSFER_AMOUNT_HI_BIT_LENGTH: usize = 32;
    const PADDING_BIT_LENGTH: usize = 16;

    let (padding_commitment, padding_opening) = Pedersen::new(0_u64);

    let range_proof_data = BatchedRangeProofU128Data::new(
        vec![
            &new_available_balance_commitment,
            &burn_amount_grouped_ciphertext_lo.commitment,
            &burn_amount_grouped_ciphertext_hi.commitment,
            &padding_commitment,
        ],
        vec![
            new_decrypted_available_balance,
            burn_amount_lo,
            burn_amount_hi,
            0,
        ],
        vec![
            REMAINING_BALANCE_BIT_LENGTH,
            TRANSFER_AMOUNT_LO_BIT_LENGTH,
            TRANSFER_AMOUNT_HI_BIT_LENGTH,
            PADDING_BIT_LENGTH,
        ],
        vec![
            &new_source_opening,
            &burn_amount_opening_lo,
            &burn_amount_opening_hi,
            &padding_opening,
        ],
    )
    .map_err(|_| TokenError::ProofGeneration)?;

    Ok((
        equality_proof_data,
        ciphertext_validity_proof_data,
        range_proof_data,
        (burn_amount_opening_hi, burn_amount_opening_lo),
    ))
}
