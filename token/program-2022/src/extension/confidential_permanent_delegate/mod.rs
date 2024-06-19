use crate::pod::{PodAccount, PodMint};
#[cfg(not(target_os = "solana"))]
use rsa::{BigUint, PublicKeyParts, RsaPublicKey};

use super::{PodStateWithExtensions, PodStateWithExtensionsMut};

use {
    super::BaseStateWithExtensions,
    crate::extension::{Extension, ExtensionType},
    bytemuck::{Pod, Zeroable},
    solana_program::{account_info::AccountInfo, hash::hash, pubkey::Pubkey},
    spl_pod::primitives::{PodBool, PodU16},
};

/// Maximum bit length of any mint or burn amount
///
/// Any mint or burn amount must be less than 2^48
pub const MAXIMUM_DEPOSIT_TRANSFER_AMOUNT: u64 = (u16::MAX as u64) + (1 << 16) * (u32::MAX as u64);

/// Bit length of the low bits of pending balance plaintext
pub const PENDING_BALANCE_LO_BIT_LENGTH: u32 = 16;

/// Maximum length of 512 bytes allows RSA keys
/// with a modulus of up to 4096 bits
pub const MAX_MODULUS_LENGTH: usize = 512;

/// Maximum length of 17 bytes allows for the usage
/// of 2^16 + 1 as the RSA public key exponent
pub const MAX_EXPONENT_LENGTH: usize = 17;

/// Confidential Transfer Extension instructions
pub mod instruction;

/// Confidential Transfer Extension processor
pub mod processor;

/// Confidential permanent delegate mint
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable, Default)]
#[repr(C)]
pub struct ConfidentialPermanentDelegate {
    /// Authority to modify the `ConfidentialTransferMint` configuration and to
    /// approve new accounts (if `auto_approve_new_accounts` is true)
    ///
    /// The legacy Token Multisig account is not supported as the authority
    pub permanent_delegate: Pubkey,

    /// Flag whether the encryption public key has been initialized after
    /// the creation of a mint with a confidential permanent delegate
    pub delegate_initialized: PodBool,

    /// RSA public key to encrypt AES-Key and ElGamal-Keypair for new
    /// confidential balance accounts with
    pub encryption_pubkey: EncyptionPublicKey,
}

/// Representation of RsaPublicKey usable for extension state
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct EncyptionPublicKey {
    /// RSA public key modulus
    pub n: [u8; MAX_MODULUS_LENGTH],
    /// RSA public key exponent
    pub e: [u8; MAX_EXPONENT_LENGTH],
    /// RSA public key modulus length
    pub len_n: PodU16,
    /// RSA public key exponent length
    pub len_e: u8,
}

impl Default for EncyptionPublicKey {
    fn default() -> Self {
        Self {
            n: [0_u8; MAX_MODULUS_LENGTH],
            e: [0_u8; MAX_EXPONENT_LENGTH],
            len_n: PodU16::zeroed(),
            len_e: 0_u8,
        }
    }
}

#[cfg(not(target_os = "solana"))]
impl EncyptionPublicKey {
    /// converts EncyptionPublicKey into rsa::RsaPublicKey
    pub fn to_rsa_public_key(&self) -> RsaPublicKey {
        let mut n = Vec::from(self.n);
        n.truncate(Into::<u16>::into(self.len_n) as usize);
        let mut e = Vec::from(self.e);
        e.truncate(self.len_e as usize);
        let n = BigUint::from_bytes_le(&n);
        let e = BigUint::from_bytes_le(&e);

        RsaPublicKey::new(n, e).unwrap()
    }
}

#[cfg(not(target_os = "solana"))]
impl From<RsaPublicKey> for EncyptionPublicKey {
    fn from(rsa_pubkey: RsaPublicKey) -> Self {
        let n = rsa_pubkey.n().to_bytes_le();
        let e = rsa_pubkey.e().to_bytes_le();

        let mut pk = EncyptionPublicKey::zeroed();
        pk.n[..n.len()].copy_from_slice(&n);
        pk.e[..e.len()].copy_from_slice(&e);
        pk.len_n = PodU16::from(n.len() as u16);
        pk.len_e = e.len() as u8;
        pk
    }
}

impl Extension for ConfidentialPermanentDelegate {
    const TYPE: ExtensionType = ExtensionType::ConfidentialPermanentDelegate;
}

/// generates seed for pda to store encrypted private keys in
pub fn encrypted_keys_pda_seed(mint: &Pubkey, ata: &Pubkey) -> [u8; 32] {
    let mut enc_key_pda_seed = mint.to_bytes().to_vec();
    enc_key_pda_seed.extend(ata.to_bytes());
    enc_key_pda_seed.extend(b"encrypted_keys");
    hash(&enc_key_pda_seed).to_bytes()
}

/// generates address and bump for pda to store whitelist info into
pub fn encrypted_keys_pda_address_bump(seed: [u8; 32], program_id: &Pubkey) -> (Pubkey, u8) {
    Pubkey::find_program_address(&[&seed], program_id)
}

/// generates address for pda to store whitelist info into
pub fn encrypted_keys_pda_address(mint: &Pubkey, ata: &Pubkey, program_id: &Pubkey) -> Pubkey {
    let seed = encrypted_keys_pda_seed(mint, ata);
    let (pda, _) = encrypted_keys_pda_address_bump(seed, program_id);
    pda
}

/// Returns the expected authority for the execution of a given instruction.
/// In case of the confidential-permanent-delegate extension not being
/// enabled on a mint this always return the token account owner
pub fn expected_authority(
    mint: &PodStateWithExtensions<'_, PodMint>,
    authority_info: &AccountInfo,
    token_account: &PodStateWithExtensionsMut<'_, PodAccount>,
) -> Pubkey {
    if let Ok(perm_del_ext) = mint.get_extension::<ConfidentialPermanentDelegate>() {
        if &perm_del_ext.permanent_delegate == authority_info.key {
            perm_del_ext.permanent_delegate
        } else {
            token_account.base.owner
        }
    } else {
        token_account.base.owner
    }
}
