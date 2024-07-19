use {
    base64::{engine::general_purpose::STANDARD, Engine},
    bytemuck::bytes_of,
    solana_curve25519::{
        ristretto::{add_ristretto, multiply_ristretto, subtract_ristretto, PodRistrettoPoint},
        scalar::PodScalar,
    },
    solana_zk_sdk::encryption::pod::elgamal::PodElGamalCiphertext,
    std::str::FromStr,
};

const SHIFT_BITS: usize = 16;

const G: PodRistrettoPoint = PodRistrettoPoint([
    226, 242, 174, 10, 106, 188, 78, 113, 168, 132, 169, 97, 197, 0, 81, 95, 88, 227, 11, 106, 165,
    130, 221, 141, 182, 166, 89, 69, 224, 141, 45, 118,
]);

/// Add two ElGamal ciphertexts
pub fn add(
    left_ciphertext: &PodElGamalCiphertext,
    right_ciphertext: &PodElGamalCiphertext,
) -> Option<PodElGamalCiphertext> {
    let (left_commitment, left_handle) = elgamal_ciphertext_to_ristretto(left_ciphertext);
    let (right_commitment, right_handle) = elgamal_ciphertext_to_ristretto(right_ciphertext);

    let result_commitment = add_ristretto(&left_commitment, &right_commitment)?;
    let result_handle = add_ristretto(&left_handle, &right_handle)?;

    Some(ristretto_to_elgamal_ciphertext(
        &result_commitment,
        &result_handle,
    ))
}

/// Multiply an ElGamal ciphertext by a scalar
pub fn multiply(
    scalar: &PodScalar,
    ciphertext: &PodElGamalCiphertext,
) -> Option<PodElGamalCiphertext> {
    let (commitment, handle) = elgamal_ciphertext_to_ristretto(ciphertext);

    let result_commitment = multiply_ristretto(scalar, &commitment)?;
    let result_handle = multiply_ristretto(scalar, &handle)?;

    Some(ristretto_to_elgamal_ciphertext(
        &result_commitment,
        &result_handle,
    ))
}

/// Compute `left_ciphertext + (right_ciphertext_lo + 2^16 *
/// right_ciphertext_hi)`
pub fn add_with_lo_hi(
    left_ciphertext: &PodElGamalCiphertext,
    right_ciphertext_lo: &PodElGamalCiphertext,
    right_ciphertext_hi: &PodElGamalCiphertext,
) -> Option<PodElGamalCiphertext> {
    let shift_scalar = u64_to_scalar(1_u64 << SHIFT_BITS);
    let shifted_right_ciphertext_hi = multiply(&shift_scalar, right_ciphertext_hi)?;
    let combined_right_ciphertext = add(right_ciphertext_lo, &shifted_right_ciphertext_hi)?;
    add(left_ciphertext, &combined_right_ciphertext)
}

/// Subtract two ElGamal ciphertexts
pub fn subtract(
    left_ciphertext: &PodElGamalCiphertext,
    right_ciphertext: &PodElGamalCiphertext,
) -> Option<PodElGamalCiphertext> {
    let (left_commitment, left_handle) = elgamal_ciphertext_to_ristretto(left_ciphertext);
    let (right_commitment, right_handle) = elgamal_ciphertext_to_ristretto(right_ciphertext);

    let result_commitment = subtract_ristretto(&left_commitment, &right_commitment)?;
    let result_handle = subtract_ristretto(&left_handle, &right_handle)?;

    Some(ristretto_to_elgamal_ciphertext(
        &result_commitment,
        &result_handle,
    ))
}

/// Compute `left_ciphertext - (right_ciphertext_lo + 2^16 *
/// right_ciphertext_hi)`
pub fn subtract_with_lo_hi(
    left_ciphertext: &PodElGamalCiphertext,
    right_ciphertext_lo: &PodElGamalCiphertext,
    right_ciphertext_hi: &PodElGamalCiphertext,
) -> Option<PodElGamalCiphertext> {
    let shift_scalar = u64_to_scalar(1_u64 << SHIFT_BITS);
    let shifted_right_ciphertext_hi = multiply(&shift_scalar, right_ciphertext_hi)?;
    let combined_right_ciphertext = add(right_ciphertext_lo, &shifted_right_ciphertext_hi)?;
    subtract(left_ciphertext, &combined_right_ciphertext)
}

/// Add a constant amount to a ciphertext
pub fn add_to(ciphertext: &PodElGamalCiphertext, amount: u64) -> Option<PodElGamalCiphertext> {
    let amount_scalar = u64_to_scalar(amount);
    let amount_point = multiply_ristretto(&amount_scalar, &G)?;

    let (commitment, handle) = elgamal_ciphertext_to_ristretto(ciphertext);

    let result_commitment = add_ristretto(&commitment, &amount_point)?;

    Some(ristretto_to_elgamal_ciphertext(&result_commitment, &handle))
}

/// Subtract a constant amount to a ciphertext
pub fn subtract_from(
    ciphertext: &PodElGamalCiphertext,
    amount: u64,
) -> Option<PodElGamalCiphertext> {
    let amount_scalar = u64_to_scalar(amount);
    let amount_point = multiply_ristretto(&amount_scalar, &G)?;

    let (commitment, handle) = elgamal_ciphertext_to_ristretto(ciphertext);

    let result_commitment = subtract_ristretto(&commitment, &amount_point)?;

    Some(ristretto_to_elgamal_ciphertext(&result_commitment, &handle))
}

/// Convert a `u64` amount into a curve25519 scalar
fn u64_to_scalar(amount: u64) -> PodScalar {
    let mut amount_bytes = [0u8; 32];
    amount_bytes[..8].copy_from_slice(&amount.to_le_bytes());
    PodScalar(amount_bytes)
}

/// Convert a `PodElGamalCiphertext` into a tuple of commitment and decrypt
/// handle `PodRistrettoPoint`
fn elgamal_ciphertext_to_ristretto(
    ciphertext: &PodElGamalCiphertext,
) -> (PodRistrettoPoint, PodRistrettoPoint) {
    let ciphertext_bytes = bytes_of(ciphertext); // must be of length 64 by type
    let commitment_bytes = ciphertext_bytes[..32].try_into().unwrap();
    let handle_bytes = ciphertext_bytes[32..64].try_into().unwrap();
    (
        PodRistrettoPoint(commitment_bytes),
        PodRistrettoPoint(handle_bytes),
    )
}

/// Convert a pair of `PodRistrettoPoint` to a `PodElGamalCiphertext`
/// interpretting the first as the commitment and the second as the handle
fn ristretto_to_elgamal_ciphertext(
    commitment: &PodRistrettoPoint,
    handle: &PodRistrettoPoint,
) -> PodElGamalCiphertext {
    let mut ciphertext_bytes = [0u8; 64];
    ciphertext_bytes[..32].copy_from_slice(bytes_of(commitment));
    ciphertext_bytes[32..64].copy_from_slice(bytes_of(handle));
    // Unfortunately, the `solana-zk-sdk` does not exporse a constructor interface
    // to construct `PodRistrettoPoint` from bytes. As a work-around, encode the
    // bytes as base64 string and then convert the string to a
    // `PodElGamalCiphertext`.
    let ciphertext_string = STANDARD.encode(ciphertext_bytes);
    FromStr::from_str(&ciphertext_string).unwrap()
}

#[cfg(test)]
mod tests {
    use {
        super::*,
        bytemuck::Zeroable,
        curve25519_dalek::scalar::Scalar,
        solana_zk_sdk::encryption::{
            elgamal::{ElGamalCiphertext, ElGamalKeypair},
            pedersen::{Pedersen, PedersenOpening},
            pod::{elgamal::PodDecryptHandle, pedersen::PodPedersenCommitment},
        },
        spl_token_confidential_transfer_proof_generation::try_split_u64,
    };

    const TWO_16: u64 = 65536;

    #[test]
    fn test_zero_ct() {
        let spendable_balance = PodElGamalCiphertext::zeroed();
        let spendable_ct: ElGamalCiphertext = spendable_balance.try_into().unwrap();

        // spendable_ct should be an encryption of 0 for any public key when
        // `PedersenOpen::default()` is used
        let keypair = ElGamalKeypair::new_rand();
        let public = keypair.pubkey();
        let balance: u64 = 0;
        assert_eq!(
            spendable_ct,
            public.encrypt_with(balance, &PedersenOpening::default())
        );

        // homomorphism should work like any other ciphertext
        let open = PedersenOpening::new_rand();
        let transfer_amount_ciphertext = public.encrypt_with(55_u64, &open);
        let transfer_amount_pod: PodElGamalCiphertext = transfer_amount_ciphertext.into();

        let sum = add(&spendable_balance, &transfer_amount_pod).unwrap();

        let expected: PodElGamalCiphertext = public.encrypt_with(55_u64, &open).into();
        assert_eq!(expected, sum);
    }

    #[test]
    fn test_add_to() {
        let spendable_balance = PodElGamalCiphertext::zeroed();

        let added_ciphertext = add_to(&spendable_balance, 55).unwrap();

        let keypair = ElGamalKeypair::new_rand();
        let public = keypair.pubkey();
        let expected: PodElGamalCiphertext = public
            .encrypt_with(55_u64, &PedersenOpening::default())
            .into();

        assert_eq!(expected, added_ciphertext);
    }

    #[test]
    fn test_subtract_from() {
        let amount = 77_u64;
        let keypair = ElGamalKeypair::new_rand();
        let public = keypair.pubkey();
        let open = PedersenOpening::new_rand();
        let encrypted_amount: PodElGamalCiphertext = public.encrypt_with(amount, &open).into();

        let subtracted_ciphertext = subtract_from(&encrypted_amount, 55).unwrap();

        let expected: PodElGamalCiphertext = public.encrypt_with(22_u64, &open).into();

        assert_eq!(expected, subtracted_ciphertext);
    }

    #[test]
    fn test_transfer_arithmetic() {
        // transfer amount
        let transfer_amount: u64 = 55;
        let (amount_lo, amount_hi) = try_split_u64(transfer_amount, 16).unwrap();

        // generate public keys
        let source_keypair = ElGamalKeypair::new_rand();
        let source_pubkey = source_keypair.pubkey();

        let destination_keypair = ElGamalKeypair::new_rand();
        let destination_pubkey = destination_keypair.pubkey();

        let auditor_keypair = ElGamalKeypair::new_rand();
        let auditor_pubkey = auditor_keypair.pubkey();

        // commitments associated with TransferRangeProof
        let (commitment_lo, opening_lo) = Pedersen::new(amount_lo);
        let (commitment_hi, opening_hi) = Pedersen::new(amount_hi);

        let commitment_lo: PodPedersenCommitment = commitment_lo.into();
        let commitment_hi: PodPedersenCommitment = commitment_hi.into();

        // decryption handles associated with TransferValidityProof
        let source_handle_lo: PodDecryptHandle = source_pubkey.decrypt_handle(&opening_lo).into();
        let destination_handle_lo: PodDecryptHandle =
            destination_pubkey.decrypt_handle(&opening_lo).into();
        let _auditor_handle_lo: PodDecryptHandle =
            auditor_pubkey.decrypt_handle(&opening_lo).into();

        let source_handle_hi: PodDecryptHandle = source_pubkey.decrypt_handle(&opening_hi).into();
        let destination_handle_hi: PodDecryptHandle =
            destination_pubkey.decrypt_handle(&opening_hi).into();
        let _auditor_handle_hi: PodDecryptHandle =
            auditor_pubkey.decrypt_handle(&opening_hi).into();

        // source spendable and recipient pending
        let source_opening = PedersenOpening::new_rand();
        let destination_opening = PedersenOpening::new_rand();

        let source_spendable_ciphertext: PodElGamalCiphertext =
            source_pubkey.encrypt_with(77_u64, &source_opening).into();
        let destination_pending_ciphertext: PodElGamalCiphertext = destination_pubkey
            .encrypt_with(77_u64, &destination_opening)
            .into();

        // program arithmetic for the source account
        let commitment_lo_point = PodRistrettoPoint(bytes_of(&commitment_lo).try_into().unwrap());
        let source_handle_lo_point =
            PodRistrettoPoint(bytes_of(&source_handle_lo).try_into().unwrap());

        let commitment_hi_point = PodRistrettoPoint(bytes_of(&commitment_hi).try_into().unwrap());
        let source_handle_hi_point =
            PodRistrettoPoint(bytes_of(&source_handle_hi).try_into().unwrap());

        let source_ciphertext_lo =
            ristretto_to_elgamal_ciphertext(&commitment_lo_point, &source_handle_lo_point);
        let source_ciphertext_hi =
            ristretto_to_elgamal_ciphertext(&commitment_hi_point, &source_handle_hi_point);

        let final_source_spendable = subtract_with_lo_hi(
            &source_spendable_ciphertext,
            &source_ciphertext_lo,
            &source_ciphertext_hi,
        )
        .unwrap();

        let final_source_opening =
            source_opening - (opening_lo.clone() + opening_hi.clone() * Scalar::from(TWO_16));
        let expected_source: PodElGamalCiphertext = source_pubkey
            .encrypt_with(22_u64, &final_source_opening)
            .into();
        assert_eq!(expected_source, final_source_spendable);

        // program arithmetic for the destination account
        let destination_handle_lo_point =
            PodRistrettoPoint(bytes_of(&destination_handle_lo).try_into().unwrap());
        let destination_handle_hi_point =
            PodRistrettoPoint(bytes_of(&destination_handle_hi).try_into().unwrap());

        let destination_ciphertext_lo =
            ristretto_to_elgamal_ciphertext(&commitment_lo_point, &destination_handle_lo_point);
        let destination_ciphertext_hi =
            ristretto_to_elgamal_ciphertext(&commitment_hi_point, &destination_handle_hi_point);

        let final_destination_pending_ciphertext = add_with_lo_hi(
            &destination_pending_ciphertext,
            &destination_ciphertext_lo,
            &destination_ciphertext_hi,
        )
        .unwrap();

        let final_destination_opening =
            destination_opening + (opening_lo + opening_hi * Scalar::from(TWO_16));
        let expected_destination_ciphertext: PodElGamalCiphertext = destination_pubkey
            .encrypt_with(132_u64, &final_destination_opening)
            .into();
        assert_eq!(
            expected_destination_ciphertext,
            final_destination_pending_ciphertext
        );
    }
}
