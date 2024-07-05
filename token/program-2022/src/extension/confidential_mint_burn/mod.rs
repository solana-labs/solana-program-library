use {
    crate::extension::{Extension, ExtensionType},
    bytemuck::{Pod, Zeroable},
    solana_program::pubkey::Pubkey,
    solana_zk_token_sdk::zk_token_elgamal::pod::ElGamalCiphertext,
    spl_pod::optional_keys::OptionalNonZeroElGamalPubkey,
};

/// Maximum bit length of any mint or burn amount
///
/// Any mint or burn amount must be less than 2^48
pub const MAXIMUM_DEPOSIT_TRANSFER_AMOUNT: u64 = (u16::MAX as u64) + (1 << 16) * (u32::MAX as u64);

/// Bit length of the low bits of pending balance plaintext
pub const PENDING_BALANCE_LO_BIT_LENGTH: u32 = 16;

/// Confidential Mint-Burn Extension instructions
pub mod instruction;

/// Confidential Mint-Burn Extension processor
pub mod processor;

/// Confidential Mint-Burn proof generation
pub mod proof_generation;

/// Confidential Mint-Burn proof verification
pub mod verify_proof;

/// Confidential Mint-Burn proof verification
pub mod ciphertext_extraction;

/// Confidential transfer mint configuration
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct ConfidentialMintBurn {
    /// Authority to modify the `ConfidentialMintBurnMint` configuration and to
    /// mint new confidential tokens
    pub mint_authority: Pubkey,
    /// The confidential supply of the mint
    pub confidential_supply: ElGamalCiphertext,
    /// The ElGamal pubkey used to encrypt the confidential supply
    pub supply_elgamal_pubkey: OptionalNonZeroElGamalPubkey,
}

impl Extension for ConfidentialMintBurn {
    const TYPE: ExtensionType = ExtensionType::ConfidentialMintBurn;
}
