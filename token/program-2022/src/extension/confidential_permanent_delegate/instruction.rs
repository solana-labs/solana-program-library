#[cfg(not(target_os = "solana"))]
use crate::{error::TokenError, proof::ProofLocation};
#[cfg(feature = "serde-traits")]
use serde::{Deserialize, Serialize};
#[cfg(not(target_os = "solana"))]
use solana_zk_token_sdk::instruction::PubkeyValidityData;
#[cfg(not(target_os = "solana"))]
use {
    super::encrypted_keys_pda_address,
    crate::{
        check_program_account,
        instruction::{encode_instruction, TokenInstruction},
    },
    rand_core::OsRng,
    rsa::{PaddingScheme, PublicKey, RsaPublicKey},
    sha2::Sha256,
    solana_program::{
        instruction::{AccountMeta, Instruction},
        system_program, sysvar,
    },
    solana_zk_token_sdk::zk_token_proof_instruction::verify_pubkey_validity,
};
use {
    super::{EncyptionPublicKey, MAX_MODULUS_LENGTH},
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{program_error::ProgramError, pubkey::Pubkey},
    spl_pod::bytemuck::pod_get_packed_len,
};

/// Confidential Transfer extension instructions
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ConfidentialPermanentDelegateInstruction {
    /// Initializes the permanent delegate for the confidential mint
    ///
    /// The `ConfidentialPermanentDelegateInstruction::InitializeMint`
    /// instruction requires no signers and MUST be included within the same
    /// Transaction as `TokenInstruction::InitializeMint`. Otherwise another
    /// party can initialize the configuration.
    ///
    /// The instruction fails if the `TokenInstruction::InitializeMint`
    /// instruction has already executed for the mint.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token mint.
    ///
    /// Data expected by this instruction:
    ///   `InitializeMintData`
    InitializeMint,
    /// Updates the permanent delegate for the mint
    UpdateMint,
    /// Configures RSA pubkey to be used for private key encryption
    ConfigureRSA,
    /// Persists ElGamal keypair and AES key encrypted with RSA
    /// public key of permanent delegate into PDA
    PostEncryptedPrivateKey,
    /// Approves confidential transfer account for usage. For the approval
    /// to happen, the permanent delegate signs the transaction and provides
    /// a valid proof generated with the keys encrypted and posted by the
    /// account owner
    ApproveAccount,
}

/// The type of private key shared / persited on chain
/// via a PostEncryptedPrivateKey instruction
#[derive(Clone, Copy, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum PrivateKeyType {
    /// ElGamalKeypair
    ElGamalKeypair,
    /// AeKey
    AeKey,
}

/// Data expected by `ConfidentialPermanentDelegateInstruction::InitializeMint`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct InitializeMintData {
    /// Pubkey of the permanent delegate for the mint
    pub permanent_delegate: Pubkey,
}

/// Data expected by `ConfidentialPermanentDelegateInstruction::UpdateMint`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct UpdateMintData {
    /// Updates the permanent delegate for the mint
    pub new_permanent_delegate: Pubkey,
}

/// Data expected by `ConfidentialPermanentDelegateInstruction::ConfigureRSA`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct ConfigureRSAInstructionData {
    /// the rsa public key to be used for the encryption of the
    /// private keys to be shared with the permanent delegate
    pub rsa_pubkey: EncyptionPublicKey,
}

/// Data expected by
/// `ConfidentialPermanentDelegateInstruction::PostEncryptedPrivateKeys`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct PostEncryptedKeysInstructionData {
    /// Encrypted private key
    pub data: [u8; MAX_MODULUS_LENGTH],
    /// Type of encrypted key posted
    pub key_type: u8,
}

/// Data expected by
/// `ConfidentialPermanentDelegateInstruction::PostEncryptedPrivateKeys`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct EncryptedPrivateKeyData {
    /// Encrypted elgamal_keypair
    pub elgamal_keypair: [u8; MAX_MODULUS_LENGTH],
    /// Encrypted aekey
    pub ae_key: [u8; MAX_MODULUS_LENGTH],
}

impl TryFrom<Vec<u8>> for EncryptedPrivateKeyData {
    type Error = ProgramError;
    fn try_from(bytes: Vec<u8>) -> Result<Self, Self::Error> {
        if bytes.len() != pod_get_packed_len::<Self>() {
            return Err(ProgramError::InvalidAccountData);
        }

        let mut s = Self::zeroed();
        s.elgamal_keypair
            .copy_from_slice(&bytes[..MAX_MODULUS_LENGTH]);
        s.ae_key.copy_from_slice(&bytes[MAX_MODULUS_LENGTH..]);
        Ok(s)
    }
}

/// Data expected by `ConfidentialPermanentDelegateInstruction::ApproveAccount`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct ApproveAccountData {
    /// Relative location of the `ProofInstruction::VerifyPubkeyValidity` instruction
    /// to the `ApproveAccount` instruction in the transaction. If the offset
    /// is `0`, then use a context state account for the proof.
    pub proof_instruction_offset: i8,
}

/// Create a `InitializeMint` instruction
#[cfg(not(target_os = "solana"))]
pub fn initialize_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    permanent_delegate: Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![AccountMeta::new(*mint, false)];

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialPermanentDelegateExtension,
        ConfidentialPermanentDelegateInstruction::InitializeMint,
        &InitializeMintData { permanent_delegate },
    ))
}

/// Create a `UpdateMint` instruction
#[cfg(not(target_os = "solana"))]
pub fn update_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    permanent_delegate: &Pubkey,
    multisig_signers: &[&Pubkey],
    new_permanent_delegate: Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new_readonly(*permanent_delegate, multisig_signers.is_empty()),
    ];
    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialPermanentDelegateExtension,
        ConfidentialPermanentDelegateInstruction::UpdateMint,
        &UpdateMintData {
            new_permanent_delegate,
        },
    ))
}

/// Create a `ConfigureRSA` instruction
#[cfg(not(target_os = "solana"))]
pub fn configure_rsa(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    rsa_pubkey: RsaPublicKey,
    permanent_delegate: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new_readonly(*permanent_delegate, multisig_signers.is_empty()),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialPermanentDelegateExtension,
        ConfidentialPermanentDelegateInstruction::ConfigureRSA,
        &ConfigureRSAInstructionData {
            rsa_pubkey: EncyptionPublicKey::from(rsa_pubkey),
        },
    ))
}

/// Create a `PostEncryptedPrivateKeys` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_os = "solana"))]
pub fn post_encrypted_private_keys(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    token_account: &Pubkey,
    ata_authority: &Pubkey,
    rent_payer: &Pubkey,
    multisig_signers: &[&Pubkey],
    rsa_pubkey: RsaPublicKey,
    key_bytes: Vec<u8>,
    key_type: PrivateKeyType,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new(
            encrypted_keys_pda_address(mint, token_account, token_program_id),
            false,
        ),
        AccountMeta::new_readonly(*ata_authority, multisig_signers.is_empty()),
        AccountMeta::new_readonly(system_program::ID, false),
        AccountMeta::new(*rent_payer, true),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    let mut rng = OsRng;
    let padding = PaddingScheme::new_oaep::<Sha256>();
    let encrypted_key = rsa_pubkey
        .encrypt(&mut rng, padding, &key_bytes)
        .expect("failed to encrypt secret key");
    let mut data = PostEncryptedKeysInstructionData::zeroed();
    data.key_type = key_type.into();
    if encrypted_key.len() != MAX_MODULUS_LENGTH {
        println!("provided RSA public key is too large, only modulus lengths of up to 4096 bits are supported");
        return Err(ProgramError::InvalidInstructionData);
    }
    data.data[..encrypted_key.len()].copy_from_slice(&encrypted_key);

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialPermanentDelegateExtension,
        ConfidentialPermanentDelegateInstruction::PostEncryptedPrivateKey,
        &data,
    ))
}

/// Create a `ApproveAccount` instruction
#[cfg(not(target_os = "solana"))]
pub fn approve_account(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    token_account: &Pubkey,
    permanent_delegate: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data_location: ProofLocation<PubkeyValidityData>,
) -> Result<Vec<Instruction>, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*token_account, false),
        AccountMeta::new(
            encrypted_keys_pda_address(mint, token_account, token_program_id),
            false,
        ),
        AccountMeta::new_readonly(*permanent_delegate, multisig_signers.is_empty()),
    ];

    let proof_instruction_offset = match proof_data_location {
        ProofLocation::InstructionOffset(proof_instruction_offset, _) => {
            accounts.push(AccountMeta::new_readonly(sysvar::instructions::id(), false));
            proof_instruction_offset.into()
        }
        ProofLocation::ContextStateAccount(context_state_account) => {
            accounts.push(AccountMeta::new_readonly(*context_state_account, false));
            0
        }
    };

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    let mut instructions = vec![encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialPermanentDelegateExtension,
        ConfidentialPermanentDelegateInstruction::ApproveAccount,
        &ApproveAccountData {
            proof_instruction_offset,
        },
    )];

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        proof_data_location
    {
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != 1 {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        instructions.push(verify_pubkey_validity(None, proof_data));
    };

    Ok(instructions)
}
