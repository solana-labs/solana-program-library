//! Helper functions to generate split zero-knowledge proofs for confidential
//! transfers in the Confidential Transfer Extension.
//!
//! The logic in this submodule should belong to the `solana-zk-token-sdk` and
//! will be removed with the next upgrade to the Solana program.

use crate::{
    extension::confidential_transfer::{
        ciphertext_extraction::{transfer_amount_source_ciphertext, SourceDecryptHandles},
        processor::verify_and_split_deposit_amount,
        *,
    },
    solana_zk_token_sdk::{
        encryption::{
            auth_encryption::{AeCiphertext, AeKey},
            elgamal::{DecryptHandle, ElGamalCiphertext, ElGamalKeypair, ElGamalPubkey},
            grouped_elgamal::GroupedElGamal,
            pedersen::Pedersen,
        },
        instruction::{
            transfer::TransferAmountCiphertext, BatchedGroupedCiphertext2HandlesValidityProofData,
            BatchedRangeProofU128Data, CiphertextCommitmentEqualityProofData,
        },
        zk_token_elgamal::ops::subtract_with_lo_hi,
    },
};

/// The main logic to create the three split proof data for a transfer.
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
        BatchedGroupedCiphertext2HandlesValidityProofData,
        BatchedRangeProofU128Data,
        SourceDecryptHandles,
    ),
    TokenError,
> {
    let default_auditor_pubkey = ElGamalPubkey::default();
    let auditor_elgamal_pubkey = auditor_elgamal_pubkey.unwrap_or(&default_auditor_pubkey);

    // Split the transfer amount into the low and high bit components.
    let (transfer_amount_lo, transfer_amount_hi) =
        verify_and_split_deposit_amount(transfer_amount)?;

    // Encrypt the `lo` and `hi` transfer amounts.
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
        .ok_or(TokenError::AccountDecryption)?;

    // Compute the remaining balance at the source
    let new_decrypted_available_balance = current_decrypted_available_balance
        .checked_sub(transfer_amount)
        .ok_or(TokenError::InsufficientFunds)?;

    // Create a new Pedersen commitment for the remaining balance at the source
    let (new_available_balance_commitment, new_source_opening) =
        Pedersen::new(new_decrypted_available_balance);

    // Compute the remaining balance at the source as ElGamal ciphertexts
    let transfer_amount_source_ciphertext_lo =
        transfer_amount_source_ciphertext(&transfer_amount_grouped_ciphertext_lo.into());
    let transfer_amount_source_ciphertext_hi =
        transfer_amount_source_ciphertext(&transfer_amount_grouped_ciphertext_hi.into());

    let current_available_balance = (*current_available_balance).into();
    let new_available_balance_ciphertext = subtract_with_lo_hi(
        &current_available_balance,
        &transfer_amount_source_ciphertext_lo,
        &transfer_amount_source_ciphertext_hi,
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

    // create source decrypt handle
    let source_decrypt_handle_lo =
        DecryptHandle::new(source_elgamal_keypair.pubkey(), &transfer_amount_opening_lo);
    let source_decrypt_handle_hi =
        DecryptHandle::new(source_elgamal_keypair.pubkey(), &transfer_amount_opening_hi);

    let source_decrypt_handles = SourceDecryptHandles {
        lo: source_decrypt_handle_lo.into(),
        hi: source_decrypt_handle_hi.into(),
    };

    // encrypt the transfer amount under the destination and auditor ElGamal public
    // key
    let transfer_amount_destination_auditor_ciphertext_lo = GroupedElGamal::encrypt_with(
        [destination_elgamal_pubkey, auditor_elgamal_pubkey],
        transfer_amount_lo,
        &transfer_amount_opening_lo,
    );
    let transfer_amount_destination_auditor_ciphertext_hi = GroupedElGamal::encrypt_with(
        [destination_elgamal_pubkey, auditor_elgamal_pubkey],
        transfer_amount_hi,
        &transfer_amount_opening_hi,
    );

    // generate ciphertext validity data
    let ciphertext_validity_proof_data = BatchedGroupedCiphertext2HandlesValidityProofData::new(
        destination_elgamal_pubkey,
        auditor_elgamal_pubkey,
        &transfer_amount_destination_auditor_ciphertext_lo,
        &transfer_amount_destination_auditor_ciphertext_hi,
        transfer_amount_lo,
        transfer_amount_hi,
        &transfer_amount_opening_lo,
        &transfer_amount_opening_hi,
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
            TRANSFER_AMOUNT_LO_BIT_LENGTH,
            TRANSFER_AMOUNT_HI_BIT_LENGTH,
            PADDING_BIT_LENGTH,
        ],
        vec![
            &new_source_opening,
            &transfer_amount_opening_lo,
            &transfer_amount_opening_hi,
            &padding_opening,
        ],
    )
    .map_err(|_| TokenError::ProofGeneration)?;

    Ok((
        equality_proof_data,
        ciphertext_validity_proof_data,
        range_proof_data,
        source_decrypt_handles,
    ))
}
