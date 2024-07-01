use {
    crate::{encryption::BurnAmountCiphertext, errors::TokenProofGenerationError},
    solana_zk_sdk::{
        encryption::{
            elgamal::{ElGamalCiphertext, ElGamalKeypair, ElGamalPubkey},
            pedersen::Pedersen,
        },
        zk_elgamal_proof_program::proof_data::{
            BatchedRangeProofU64Data, CiphertextCommitmentEqualityProofData,
            GroupedCiphertext2HandlesValidityProofData,
        },
    },
};

const BURN_AMOUNT_BIT_LENGTH: usize = 64;

pub fn burn_split_proof_data(
    burn_amount: u64,
    source_elgamal_keypair: &ElGamalKeypair,
    auditor_elgamal_pubkey: &ElGamalPubkey,
    current_spendable_balance: u64,
    current_spendable_balance_ciphertext: &ElGamalCiphertext,
) -> Result<
    (
        CiphertextCommitmentEqualityProofData,
        GroupedCiphertext2HandlesValidityProofData,
        BatchedRangeProofU64Data,
    ),
    TokenProofGenerationError,
> {
    // Encrypt the burn amount under the source and auditor's ElGamal public key
    let (burn_amount_ciphertext, burn_amount_opening) = BurnAmountCiphertext::new(
        burn_amount,
        source_elgamal_keypair.pubkey(),
        auditor_elgamal_pubkey,
    );

    // Copmute the remaining balance ciphertext
    let burn_amount_ciphertext_source = burn_amount_ciphertext.0.to_elgamal_ciphertext(0).unwrap();

    #[allow(clippy::arithmetic_side_effects)]
    let remaining_balance_ciphertext =
        current_spendable_balance_ciphertext - burn_amount_ciphertext_source;

    // Compute the remaining balance at the source
    let remaining_balance = current_spendable_balance
        .checked_sub(burn_amount)
        .ok_or(TokenProofGenerationError::NotEnoughFunds)?;

    let (remaining_balance_commitment, remaining_balance_opening) =
        Pedersen::new(remaining_balance);

    let equality_proof_data = CiphertextCommitmentEqualityProofData::new(
        source_elgamal_keypair,
        &remaining_balance_ciphertext,
        &remaining_balance_commitment,
        &remaining_balance_opening,
        remaining_balance,
    )
    .map_err(TokenProofGenerationError::from)?;

    let ciphertext_validity_proof_data = GroupedCiphertext2HandlesValidityProofData::new(
        source_elgamal_keypair.pubkey(),
        auditor_elgamal_pubkey,
        &burn_amount_ciphertext.0,
        burn_amount,
        &burn_amount_opening,
    )
    .map_err(TokenProofGenerationError::from)?;

    let range_proof_data = BatchedRangeProofU64Data::new(
        vec![&remaining_balance_commitment],
        vec![remaining_balance],
        vec![BURN_AMOUNT_BIT_LENGTH],
        vec![&remaining_balance_opening],
    )
    .map_err(TokenProofGenerationError::from)?;

    Ok((
        equality_proof_data,
        ciphertext_validity_proof_data,
        range_proof_data,
    ))
}
