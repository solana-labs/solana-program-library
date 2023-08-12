//! Ciphertext extraction and proof related helper logic
//!
//! This submodule should be removed with the next upgrade to the Solana program

use crate::{
    extension::{confidential_transfer::*, confidential_transfer_fee::EncryptedFee},
    solana_zk_token_sdk::{
        instruction::transfer::TransferProofContext,
        zk_token_elgamal::pod::{
            DecryptHandle, GroupedElGamalCiphertext2Handles, GroupedElGamalCiphertext3Handles,
            PedersenCommitment, TransferAmountCiphertext,
        },
    },
};

pub(crate) fn transfer_amount_commitment(
    transfer_amount_ciphertext: &GroupedElGamalCiphertext2Handles,
) -> PedersenCommitment {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);
    let transfer_amount_commitment_bytes =
        transfer_amount_ciphertext_bytes[..32].try_into().unwrap();
    PedersenCommitment(transfer_amount_commitment_bytes)
}

/// Extract the transfer amount ciphertext encrypted under the source ElGamal public key.
///
/// A transfer amount ciphertext consists of the following 32-byte components that are serialized
/// in order:
///   1. The `commitment` component that encodes the transfer amount.
///   2. The `decryption handle` component with respect to the source public key.
///   3. The `decryption handle` component with respect to the destination public key.
///   4. The `decryption handle` component with respect to the auditor public key.
///
/// An ElGamal ciphertext for the source consists of the `commitment` component and the `decryption
/// handle` component with respect to the source.
pub(crate) fn transfer_amount_source_ciphertext(
    transfer_amount_ciphertext: &TransferAmountCiphertext,
) -> ElGamalCiphertext {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);

    let mut source_ciphertext_bytes = [0u8; 64];
    source_ciphertext_bytes[..32].copy_from_slice(&transfer_amount_ciphertext_bytes[..32]);
    source_ciphertext_bytes[32..].copy_from_slice(&transfer_amount_ciphertext_bytes[32..64]);

    ElGamalCiphertext(source_ciphertext_bytes)
}

/// Extract the transfer amount ciphertext encrypted under the destination ElGamal public key.
///
/// A transfer amount ciphertext consists of the following 32-byte components that are serialized
/// in order:
///   1. The `commitment` component that encodes the transfer amount.
///   2. The `decryption handle` component with respect to the source public key.
///   3. The `decryption handle` component with respect to the destination public key.
///   4. The `decryption handle` component with respect to the auditor public key.
///
/// An ElGamal ciphertext for the destination consists of the `commitment` component and the
/// `decryption handle` component with respect to the destination public key.
#[cfg(feature = "zk-ops")]
pub(crate) fn transfer_amount_destination_ciphertext(
    transfer_amount_ciphertext: &TransferAmountCiphertext,
) -> ElGamalCiphertext {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);

    let mut destination_ciphertext_bytes = [0u8; 64];
    destination_ciphertext_bytes[..32].copy_from_slice(&transfer_amount_ciphertext_bytes[..32]);
    destination_ciphertext_bytes[32..].copy_from_slice(&transfer_amount_ciphertext_bytes[64..96]);

    ElGamalCiphertext(destination_ciphertext_bytes)
}

/// Extract the fee amount ciphertext encrypted under the destination ElGamal public key.
///
/// A fee encryption amount consists of the following 32-byte components that are serialized in
/// order:
///   1. The `commitment` component that encodes the fee amount.
///   2. The `decryption handle` component with respect to the destination public key.
///   3. The `decryption handle` component with respect to the withdraw withheld authority public
///      key.
///
/// An ElGamal ciphertext for the destination consists of the `commitment` component and the
/// `decryption handle` component with respect to the destination public key.
#[cfg(feature = "zk-ops")]
pub(crate) fn fee_amount_destination_ciphertext(
    transfer_amount_ciphertext: &EncryptedFee,
) -> ElGamalCiphertext {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);

    let mut source_ciphertext_bytes = [0u8; 64];
    source_ciphertext_bytes[..32].copy_from_slice(&transfer_amount_ciphertext_bytes[..32]);
    source_ciphertext_bytes[32..].copy_from_slice(&transfer_amount_ciphertext_bytes[32..64]);

    ElGamalCiphertext(source_ciphertext_bytes)
}

/// Extract the transfer amount ciphertext encrypted under the withdraw withheld authority ElGamal
/// public key.
///
/// A fee encryption amount consists of the following 32-byte components that are serialized in
/// order:
///   1. The `commitment` component that encodes the fee amount.
///   2. The `decryption handle` component with respect to the destination public key.
///   3. The `decryption handle` component with respect to the withdraw withheld authority public
///      key.
///
/// An ElGamal ciphertext for the destination consists of the `commitment` component and the
/// `decryption handle` component with respect to the withdraw withheld authority public key.
#[cfg(feature = "zk-ops")]
pub(crate) fn fee_amount_withdraw_withheld_authority_ciphertext(
    transfer_amount_ciphertext: &EncryptedFee,
) -> ElGamalCiphertext {
    let transfer_amount_ciphertext_bytes = bytemuck::bytes_of(transfer_amount_ciphertext);

    let mut destination_ciphertext_bytes = [0u8; 64];
    destination_ciphertext_bytes[..32].copy_from_slice(&transfer_amount_ciphertext_bytes[..32]);
    destination_ciphertext_bytes[32..].copy_from_slice(&transfer_amount_ciphertext_bytes[64..96]);

    ElGamalCiphertext(destination_ciphertext_bytes)
}

#[cfg(feature = "zk-ops")]
pub(crate) fn transfer_amount_encryption_from_decrypt_handle(
    source_decrypt_handle: &DecryptHandle,
    grouped_ciphertext: &GroupedElGamalCiphertext2Handles,
) -> TransferAmountCiphertext {
    let source_decrypt_handle_bytes = bytemuck::bytes_of(source_decrypt_handle);
    let grouped_ciphertext_bytes = bytemuck::bytes_of(grouped_ciphertext);

    let mut transfer_amount_ciphertext_bytes = [0u8; 128];
    transfer_amount_ciphertext_bytes[..32].copy_from_slice(&grouped_ciphertext_bytes[..32]);
    transfer_amount_ciphertext_bytes[32..64].copy_from_slice(&source_decrypt_handle_bytes);
    transfer_amount_ciphertext_bytes[64..128].copy_from_slice(&grouped_ciphertext_bytes[32..96]);

    TransferAmountCiphertext(GroupedElGamalCiphertext3Handles(
        transfer_amount_ciphertext_bytes,
    ))
}
