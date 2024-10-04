#[cfg(not(target_os = "solana"))]
use crate::error::TokenError;
#[cfg(not(target_os = "solana"))]
use solana_zk_sdk::encryption::pod::elgamal::PodElGamalPubkey;
#[cfg(not(target_os = "solana"))]
use solana_zk_sdk::zk_elgamal_proof_program::proof_data::CiphertextCiphertextEqualityProofData;
#[cfg(not(target_os = "solana"))]
use solana_zk_sdk::{
    encryption::{
        auth_encryption::AeCiphertext,
        elgamal::{ElGamalCiphertext, ElGamalKeypair, ElGamalPubkey},
        pedersen::PedersenOpening,
    },
    zk_elgamal_proof_program::instruction::ProofInstruction,
};
#[cfg(not(target_os = "solana"))]
use {
    super::ConfidentialMintBurn,
    crate::{
        check_program_account,
        instruction::{encode_instruction, TokenInstruction},
    },
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        sysvar,
    },
};
use {
    crate::extension::confidential_transfer::DecryptableBalance,
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::pubkey::Pubkey,
    solana_zk_sdk::encryption::pod::auth_encryption::PodAeCiphertext,
    spl_pod::optional_keys::OptionalNonZeroElGamalPubkey,
};
#[cfg(feature = "serde-traits")]
use {
    crate::serialization::aeciphertext_fromstr,
    serde::{Deserialize, Serialize},
};

/// Confidential Transfer extension instructions
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ConfidentialMintBurnInstruction {
    /// Initializes confidential mints and burns for a mint.
    ///
    /// The `ConfidentialMintBurnInstruction::InitializeMint` instruction
    /// requires no signers and MUST be included within the same Transaction
    /// as `TokenInstruction::InitializeMint`. Otherwise another party can
    /// initialize the configuration.
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
    /// Updates mint-authority for confidential-mint-burn mint.
    UpdateAuthority,
    /// Rotates the ElGamal key used to encrypt confidential supply
    RotateSupplyElGamal,
    /// Mints confidential tokens to
    ConfidentialMint,
    /// Removes whitelist designation for specific address
    ConfidentialBurn,
}

/// Data expected by `ConfidentialMintBurnInstruction::InitializeMint`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct InitializeMintData {
    /// Authority used to modify the `ConfidentialMintBurn` mint
    /// configuration and mint new tokens
    pub authority: Pubkey,
    /// The ElGamal pubkey used to encrypt the confidential supply
    pub supply_elgamal_pubkey: OptionalNonZeroElGamalPubkey,
}

/// Data expected by `ConfidentialMintBurnInstruction::UpdateMint`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct UpdateAuthorityData {
    /// The new `authority` pubkey
    pub new_authority: Pubkey,
}

/// Data expected by `ConfidentialMintBurnInstruction::RotateSupplyElGamal`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct RotateSupplyElGamalData {
    /// The new ElGamal pubkey for supply encryption
    pub new_supply_elgamal_pubkey: OptionalNonZeroElGamalPubkey,
    /// The location of the
    /// `ProofInstruction::VerifyCiphertextCiphertextEquality` instruction
    /// relative to the `RotateSupplyElGamal` instruction in the transaction
    pub proof_location: i8,
}

/// Data expected by `ConfidentialMintBurnInstruction::ConfidentialMint`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct MintInstructionData {
    /// The new decryptable supply if the mint succeeds
    #[cfg_attr(feature = "serde-traits", serde(with = "aeciphertext_fromstr"))]
    pub new_decryptable_supply: PodAeCiphertext,
    /// Relative location of the `ProofInstruction::VerifyBatchedRangeProofU64`
    /// instruction to the `ConfidentialMint` instruction in the
    /// transaction. The
    /// `ProofInstruction::VerifyBatchedGroupedCiphertext2HandlesValidity`
    /// has to always be at the instruction directly after the range proof one.
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialMintBurnInstruction::ConfidentialBurn`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct BurnInstructionData {
    /// The new decryptable balance of the burner if the burn succeeds
    #[cfg_attr(feature = "serde-traits", serde(with = "aeciphertext_fromstr"))]
    pub new_decryptable_available_balance: DecryptableBalance,
    /// Relative location of the
    /// `ProofInstruction::VerifyCiphertextCommitmentEquality` instruction
    /// to the `ConfidentialBurn` instruction in the transaction. The
    /// `ProofInstruction::VerifyBatchedRangeProofU128` has to always be at
    /// the instruction directly after the equality proof one,
    /// with the `ProofInstruction::VerifyBatchedGroupedCiphertext2HandlesValidity`
    /// following after that.
    pub proof_instruction_offset: i8,
}

/// Create a `InitializeMint` instruction
#[cfg(not(target_os = "solana"))]
pub fn initialize_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: Pubkey,
    confidential_supply_pubkey: Option<PodElGamalPubkey>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![AccountMeta::new(*mint, false)];

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialMintBurnExtension,
        ConfidentialMintBurnInstruction::InitializeMint,
        &InitializeMintData {
            authority,
            supply_elgamal_pubkey: confidential_supply_pubkey.try_into()?,
        },
    ))
}

/// Create a `UpdateMint` instruction
#[cfg(not(target_os = "solana"))]
pub fn update_authority(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    new_authority: Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];
    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialMintBurnExtension,
        ConfidentialMintBurnInstruction::UpdateAuthority,
        &UpdateAuthorityData { new_authority },
    ))
}

/// Create a `RotateSupplyElGamal` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_os = "solana"))]
pub fn rotate_supply_elgamal(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    extension_state: &ConfidentialMintBurn,
    current_supply: u64,
    supply_elgamal_keypair: &ElGamalKeypair,
    new_supply_elgamal_keypair: &ElGamalKeypair,
) -> Result<Vec<Instruction>, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];
    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    let new_supply_opening = PedersenOpening::new_rand();
    let new_supply_ciphertext = new_supply_elgamal_keypair
        .pubkey()
        .encrypt_with(current_supply, &new_supply_opening);

    let proof_data = CiphertextCiphertextEqualityProofData::new(
        supply_elgamal_keypair,
        new_supply_elgamal_keypair.pubkey(),
        &ElGamalCiphertext::try_from(extension_state.confidential_supply)
            .map_err(|_| TokenError::InvalidState)?,
        &new_supply_ciphertext,
        &new_supply_opening,
        current_supply,
    )
    .map_err(|_| TokenError::ProofGeneration)?;
    accounts.push(AccountMeta::new_readonly(sysvar::instructions::id(), false));

    Ok(vec![
        encode_instruction(
            token_program_id,
            accounts,
            TokenInstruction::ConfidentialMintBurnExtension,
            ConfidentialMintBurnInstruction::RotateSupplyElGamal,
            &RotateSupplyElGamalData {
                new_supply_elgamal_pubkey: Some(Into::<PodElGamalPubkey>::into(
                    *new_supply_elgamal_keypair.pubkey(),
                ))
                .try_into()?,
                proof_location: 1,
            },
        ),
        ProofInstruction::VerifyCiphertextCiphertextEquality.encode_verify_proof(None, &proof_data),
    ])
}

/// Create a `ConfidentialMint` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_os = "solana"))]
pub fn confidential_mint_with_split_proofs(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    supply_elgamal_pubkey: Option<ElGamalPubkey>,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    context_accounts: &MintSplitContextStateAccounts,
    new_decryptable_supply: AeCiphertext,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![AccountMeta::new(*token_account, false)];
    // we only need write lock to adjust confidential suppy on
    // mint if a value for supply_elgamal_pubkey has been set
    if supply_elgamal_pubkey.is_some() {
        accounts.push(AccountMeta::new(*mint, false));
    } else {
        accounts.push(AccountMeta::new_readonly(*mint, false));
    }

    accounts.push(AccountMeta::new_readonly(
        *context_accounts.equality_proof,
        false,
    ));
    accounts.push(AccountMeta::new_readonly(
        *context_accounts.ciphertext_validity_proof,
        false,
    ));
    accounts.push(AccountMeta::new_readonly(
        *context_accounts.range_proof,
        false,
    ));

    accounts.push(AccountMeta::new_readonly(
        *authority,
        multisig_signers.is_empty(),
    ));
    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialMintBurnExtension,
        ConfidentialMintBurnInstruction::ConfidentialMint,
        &MintInstructionData {
            new_decryptable_supply: new_decryptable_supply.into(),
            proof_instruction_offset: 0,
        },
    ))
}

/// Context state accounts used in confidential mint
#[derive(Clone, Copy)]
pub struct BurnSplitContextStateAccounts<'a> {
    /// Location of equality proof
    pub equality_proof: &'a Pubkey,
    /// Location of ciphertext validity proof
    pub ciphertext_validity_proof: &'a Pubkey,
    /// Location of range proof
    pub range_proof: &'a Pubkey,
    /// Authority able to close proof accounts
    pub authority: &'a Pubkey,
}

/// Context state accounts used in confidential mint
#[derive(Clone, Copy)]
pub struct MintSplitContextStateAccounts<'a> {
    /// Location of equality proof
    pub equality_proof: &'a Pubkey,
    /// Location of ciphertext validity proof
    pub ciphertext_validity_proof: &'a Pubkey,
    /// Location of range proof
    pub range_proof: &'a Pubkey,
    /// Authority able to close proof accounts
    pub authority: &'a Pubkey,
}

/// Create a `ConfidentialBurn` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_os = "solana"))]
pub fn confidential_burn_with_split_proofs(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    supply_elgamal_pubkey: Option<ElGamalPubkey>,
    new_decryptable_available_balance: DecryptableBalance,
    context_accounts: &BurnSplitContextStateAccounts,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Vec<Instruction>, ProgramError> {
    Ok(vec![inner_confidential_burn_with_split_proofs(
        token_program_id,
        token_account,
        mint,
        supply_elgamal_pubkey,
        new_decryptable_available_balance,
        context_accounts,
        authority,
        multisig_signers,
    )?])
}

/// Create a inner `ConfidentialBurn` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_os = "solana"))]
pub fn inner_confidential_burn_with_split_proofs(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    supply_elgamal_pubkey: Option<ElGamalPubkey>,
    new_decryptable_available_balance: DecryptableBalance,
    context_accounts: &BurnSplitContextStateAccounts,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![AccountMeta::new(*token_account, false)];
    if supply_elgamal_pubkey.is_some() {
        accounts.push(AccountMeta::new(*mint, false));
    } else {
        accounts.push(AccountMeta::new_readonly(*mint, false));
    }

    accounts.push(AccountMeta::new_readonly(
        *context_accounts.equality_proof,
        false,
    ));
    accounts.push(AccountMeta::new_readonly(
        *context_accounts.ciphertext_validity_proof,
        false,
    ));
    accounts.push(AccountMeta::new_readonly(
        *context_accounts.range_proof,
        false,
    ));

    accounts.push(AccountMeta::new_readonly(
        *authority,
        multisig_signers.is_empty(),
    ));

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialMintBurnExtension,
        ConfidentialMintBurnInstruction::ConfidentialBurn,
        &BurnInstructionData {
            new_decryptable_available_balance,
            proof_instruction_offset: 0,
        },
    ))
}
