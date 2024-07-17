use {
    crate::errors::TokenProofGenerationError,
    solana_zk_sdk::{
        encryption::{
            elgamal::{ElGamal, ElGamalCiphertext, ElGamalKeypair},
            pedersen::Pedersen,
        },
        zk_elgamal_proof_program::proof_data::{
            BatchedRangeProofU64Data, CiphertextCommitmentEqualityProofData,
        },
    },
};

const REMAINING_BALANCE_BIT_LENGTH: usize = 64;

/// Proof data required for a withdraw instruction
pub struct WithdrawProofData {
    pub equality_proof_data: CiphertextCommitmentEqualityProofData,
    pub range_proof_data: BatchedRangeProofU64Data,
}

pub fn withdraw_proof_data(
    current_available_balance: &ElGamalCiphertext,
    current_balance: u64,
    withdraw_amount: u64,
    elgamal_keypair: &ElGamalKeypair,
) -> Result<WithdrawProofData, TokenProofGenerationError> {
    // Calculate the remaining balance after withdraw
    let remaining_balance = current_balance
        .checked_sub(withdraw_amount)
        .ok_or(TokenProofGenerationError::NotEnoughFunds)?;

    // Generate a Pedersen commitment for the remaining balance
    let (remaining_balance_commitment, remaining_balance_opening) =
        Pedersen::new(remaining_balance);

    // Compute the remaining balance ciphertext
    #[allow(clippy::arithmetic_side_effects)]
    let remaining_balance_ciphertext = current_available_balance - ElGamal::encode(withdraw_amount);

    // Generate proof data
    let equality_proof_data = CiphertextCommitmentEqualityProofData::new(
        elgamal_keypair,
        &remaining_balance_ciphertext,
        &remaining_balance_commitment,
        &remaining_balance_opening,
        remaining_balance,
    )
    .map_err(TokenProofGenerationError::from)?;

    let range_proof_data = BatchedRangeProofU64Data::new(
        vec![&remaining_balance_commitment],
        vec![remaining_balance],
        vec![REMAINING_BALANCE_BIT_LENGTH],
        vec![&remaining_balance_opening],
    )
    .map_err(TokenProofGenerationError::from)?;

    Ok(WithdrawProofData {
        equality_proof_data,
        range_proof_data,
    })
}
