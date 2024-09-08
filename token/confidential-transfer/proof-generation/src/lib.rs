use {
    curve25519_dalek::scalar::Scalar,
    solana_zk_sdk::encryption::{
        elgamal::ElGamalCiphertext,
        pedersen::{PedersenCommitment, PedersenOpening},
    },
};

pub mod burn;
pub mod encryption;
pub mod errors;
pub mod mint;
pub mod transfer;
pub mod transfer_with_fee;
pub mod withdraw;

/// The low bit length of the encrypted transfer amount
pub const TRANSFER_AMOUNT_LO_BITS: usize = 16;
/// The high bit length of the encrypted transfer amount
pub const TRANSFER_AMOUNT_HI_BITS: usize = 32;
/// The bit length of the encrypted remaining balance in a token account
pub const REMAINING_BALANCE_BIT_LENGTH: usize = 64;

/// Takes in a 64-bit number `amount` and a bit length `bit_length`. It returns:
/// - the `bit_length` low bits of `amount` interpretted as u64
/// - the `(64 - bit_length)` high bits of `amount` interpretted as u64
pub fn try_split_u64(amount: u64, bit_length: usize) -> Option<(u64, u64)> {
    match bit_length {
        0 => Some((0, amount)),
        1..=63 => {
            let bit_length_complement = u64::BITS.checked_sub(bit_length as u32).unwrap();
            // shifts are safe as long as `bit_length` and `bit_length_complement` < 64
            let lo = amount
                .checked_shl(bit_length_complement)?
                .checked_shr(bit_length_complement)?;
            let hi = amount.checked_shr(bit_length as u32)?;
            Some((lo, hi))
        }
        64 => Some((amount, 0)),
        _ => None,
    }
}

/// Combine two numbers that are interpretted as the low and high bits of a
/// target number. The `bit_length` parameter specifies the number of bits that
/// `amount_hi` is to be shifted by.
pub fn try_combine_lo_hi_u64(amount_lo: u64, amount_hi: u64, bit_length: usize) -> Option<u64> {
    match bit_length {
        0 => Some(amount_hi),
        1..=63 => {
            // shifts are safe as long as `bit_length` < 64
            amount_hi
                .checked_shl(bit_length as u32)?
                .checked_add(amount_hi)
        }
        64 => Some(amount_lo),
        _ => None,
    }
}

#[allow(clippy::arithmetic_side_effects)]
pub fn try_combine_lo_hi_ciphertexts(
    ciphertext_lo: &ElGamalCiphertext,
    ciphertext_hi: &ElGamalCiphertext,
    bit_length: usize,
) -> Option<ElGamalCiphertext> {
    let two_power = 1_u64.checked_shl(bit_length as u32)?;
    Some(ciphertext_lo + ciphertext_hi * Scalar::from(two_power))
}

#[allow(clippy::arithmetic_side_effects)]
pub fn try_combine_lo_hi_commitments(
    comm_lo: &PedersenCommitment,
    comm_hi: &PedersenCommitment,
    bit_length: usize,
) -> Option<PedersenCommitment> {
    let two_power = 1_u64.checked_shl(bit_length as u32)?;
    Some(comm_lo + comm_hi * Scalar::from(two_power))
}

#[allow(clippy::arithmetic_side_effects)]
pub fn try_combine_lo_hi_openings(
    opening_lo: &PedersenOpening,
    opening_hi: &PedersenOpening,
    bit_length: usize,
) -> Option<PedersenOpening> {
    let two_power = 1_u64.checked_shl(bit_length as u32)?;
    Some(opening_lo + opening_hi * Scalar::from(two_power))
}
