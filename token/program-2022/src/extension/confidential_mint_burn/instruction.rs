#[cfg(not(target_os = "solana"))]
use crate::proof::{process_proof_location, ProofLocation};
#[cfg(not(target_os = "solana"))]
use solana_zk_sdk::encryption::{auth_encryption::AeCiphertext, elgamal::ElGamalPubkey};
#[cfg(not(target_os = "solana"))]
use solana_zk_sdk::zk_elgamal_proof_program::{
    instruction::ProofInstruction,
    proof_data::{
        BatchedGroupedCiphertext3HandlesValidityProofData, BatchedRangeProofU128Data,
        CiphertextCiphertextEqualityProofData, CiphertextCommitmentEqualityProofData,
    },
};
#[cfg(feature = "serde-traits")]
use {
    crate::serialization::aeciphertext_fromstr,
    serde::{Deserialize, Serialize},
};
use {
    crate::{
        check_program_account,
        extension::confidential_transfer::DecryptableBalance,
        instruction::{encode_instruction, TokenInstruction},
    },
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
    },
    solana_zk_sdk::encryption::pod::{auth_encryption::PodAeCiphertext, elgamal::PodElGamalPubkey},
    spl_pod::optional_keys::{OptionalNonZeroElGamalPubkey, OptionalNonZeroPubkey},
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
    /// Rotates the ElGamal pubkey used to encrypt confidential supply
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single authority
    ///   0. `[writable]` The SPL Token mint.
    ///   1. `[]` Instructions sysvar if `CiphertextCiphertextEquality` is
    ///      included in the same transaction or context state account if
    ///      `CiphertextCiphertextEquality` is pre-verified into a context state
    ///      account.
    ///   2. `[signer]` Confidential mint authority.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The SPL Token mint.
    ///   1. `[]` Instructions sysvar if `CiphertextCiphertextEquality` is
    ///      included in the same transaction or context state account if
    ///      `CiphertextCiphertextEquality` is pre-verified into a context state
    ///      account.
    ///   2. `[]` The multisig authority account owner.
    ///   3.. `[signer]` Required M signer accounts for the SPL Token Multisig
    ///
    /// Data expected by this instruction:
    ///   `RotateSupplyElGamalPubkeyData`
    RotateSupplyElGamalPubkey,
    /// Updates the decryptable supply of the mint
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single authority
    ///   0. `[writable]` The SPL Token mint.
    ///   1. `[signer]` Confidential mint authority.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The SPL Token mint.
    ///   1. `[]` The multisig authority account owner.
    ///   2.. `[signer]` Required M signer accounts for the SPL Token Multisig
    ///
    /// Data expected by this instruction:
    ///   `UpdateDecryptableSupplyData`
    UpdateDecryptableSupply,
    /// Mints tokens to confidential balance
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single authority
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` The SPL Token mint. `[writable]` if the mint has a non-zero
    ///      supply elgamal-pubkey
    ///   2. `[]` (Optional) Instructions sysvar if at least one of the
    ///      `zk_elgamal_proof` instructions are included in the same
    ///      transaction.
    ///   3. `[]` (Optional) The context state account containing the
    ///      pre-verified `VerifyCiphertextCommitmentEquality` proof
    ///   4. `[]` (Optional) The context state account containing the
    ///      pre-verified `VerifyBatchedGroupedCiphertext3HandlesValidity` proof
    ///   5. `[]` (Optional) The context state account containing the
    ///      pre-verified `VerifyBatchedRangeProofU128`
    ///   6. `[signer]` The single account owner.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The SPL Token mint.
    ///   1. `[]` The SPL Token mint. `[writable]` if the mint has a non-zero
    ///      supply elgamal-pubkey
    ///   2. `[]` (Optional) Instructions sysvar if at least one of the
    ///      `zk_elgamal_proof` instructions are included in the same
    ///      transaction.
    ///   3. `[]` (Optional) The context state account containing the
    ///      pre-verified `VerifyCiphertextCommitmentEquality` proof
    ///   4. `[]` (Optional) The context state account containing the
    ///      pre-verified `VerifyBatchedGroupedCiphertext3HandlesValidity` proof
    ///   5. `[]` (Optional) The context state account containing the
    ///      pre-verified `VerifyBatchedRangeProofU128`
    ///   6. `[]` The multisig account owner.
    ///   7.. `[signer]` Required M signer accounts for the SPL Token Multisig
    ///
    /// Data expected by this instruction:
    ///   `MintInstructionData`
    ConfidentialMint,
    /// Burn tokens from confidential balance
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single authority
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` The SPL Token mint. `[writable]` if the mint has a non-zero
    ///      supply elgamal-pubkey
    ///   2. `[]` (Optional) Instructions sysvar if at least one of the
    ///      `zk_elgamal_proof` instructions are included in the same
    ///      transaction.
    ///   3. `[]` (Optional) The context state account containing the
    ///      pre-verified `VerifyCiphertextCommitmentEquality` proof
    ///   4. `[]` (Optional) The context state account containing the
    ///      pre-verified `VerifyBatchedGroupedCiphertext3HandlesValidity` proof
    ///   5. `[]` (Optional) The context state account containing the
    ///      pre-verified `VerifyBatchedRangeProofU128`
    ///   6. `[signer]` The single account owner.
    ///
    ///   * Multisignature authority
    ///   0. `[writable]` The SPL Token mint.
    ///   1. `[]` The SPL Token mint. `[writable]` if the mint has a non-zero
    ///      supply elgamal-pubkey
    ///   2. `[]` (Optional) Instructions sysvar if at least one of the
    ///      `zk_elgamal_proof` instructions are included in the same
    ///      transaction.
    ///   3. `[]` (Optional) The context state account containing the
    ///      pre-verified `VerifyCiphertextCommitmentEquality` proof
    ///   4. `[]` (Optional) The context state account containing the
    ///      pre-verified `VerifyBatchedGroupedCiphertext3HandlesValidity` proof
    ///   5. `[]` (Optional) The context state account containing the
    ///      pre-verified `VerifyBatchedRangeProofU128`
    ///   6. `[]` The multisig account owner.
    ///   7.. `[signer]` Required M signer accounts for the SPL Token Multisig
    ///
    /// Data expected by this instruction:
    ///   `BurnInstructionData`
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
    pub authority: OptionalNonZeroPubkey,
    /// The ElGamal pubkey used to encrypt the confidential supply
    pub supply_elgamal_pubkey: OptionalNonZeroElGamalPubkey,
    /// The initial 0 supply ecrypted with the supply aes key
    #[cfg_attr(feature = "serde-traits", serde(with = "aeciphertext_fromstr"))]
    pub decryptable_supply: PodAeCiphertext,
}

/// Data expected by `ConfidentialMintBurnInstruction::RotateSupplyElGamal`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct RotateSupplyElGamalPubkeyData {
    /// The new ElGamal pubkey for supply encryption
    pub new_supply_elgamal_pubkey: OptionalNonZeroElGamalPubkey,
    /// The location of the
    /// `ProofInstruction::VerifyCiphertextCiphertextEquality` instruction
    /// relative to the `RotateSupplyElGamal` instruction in the transaction
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialMintBurnInstruction::UpdateDecryptableSupply`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct UpdateDecryptableSupplyData {
    /// The new decryptable supply
    #[cfg_attr(feature = "serde-traits", serde(with = "aeciphertext_fromstr"))]
    pub new_decryptable_supply: PodAeCiphertext,
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
    /// Relative location of the
    /// `ProofInstruction::VerifyCiphertextCommitmentEquality` instruction
    /// to the `ConfidentialMint` instruction in the transaction. 0 if the
    /// proof is in a pre-verified context account
    pub equality_proof_instruction_offset: i8,
    /// Relative location of the
    /// `ProofInstruction::VerifyBatchedGroupedCiphertext3HandlesValidity`
    /// instruction to the `ConfidentialMint` instruction in the
    /// transaction. 0 if the proof is in a pre-verified context account
    pub ciphertext_validity_proof_instruction_offset: i8,
    /// Relative location of the `ProofInstruction::VerifyBatchedRangeProofU128`
    /// instruction to the `ConfidentialMint` instruction in the
    /// transaction. 0 if the proof is in a pre-verified context account
    pub range_proof_instruction_offset: i8,
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
    /// to the `ConfidentialMint` instruction in the transaction. 0 if the
    /// proof is in a pre-verified context account
    pub equality_proof_instruction_offset: i8,
    /// Relative location of the
    /// `ProofInstruction::VerifyBatchedGroupedCiphertext3HandlesValidity`
    /// instruction to the `ConfidentialMint` instruction in the
    /// transaction. 0 if the proof is in a pre-verified context account
    pub ciphertext_validity_proof_instruction_offset: i8,
    /// Relative location of the `ProofInstruction::VerifyBatchedRangeProofU128`
    /// instruction to the `ConfidentialMint` instruction in the
    /// transaction. 0 if the proof is in a pre-verified context account
    pub range_proof_instruction_offset: i8,
}

/// Create a `InitializeMint` instruction
pub fn initialize_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: Pubkey,
    confidential_supply_pubkey: Option<PodElGamalPubkey>,
    decryptable_supply: Option<PodAeCiphertext>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![AccountMeta::new(*mint, false)];

    let decryptable_supply = if confidential_supply_pubkey.is_some() {
        decryptable_supply.ok_or(ProgramError::InvalidInstructionData)?
    } else {
        PodAeCiphertext::zeroed()
    };

    let authority = Some(authority);
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialMintBurnExtension,
        ConfidentialMintBurnInstruction::InitializeMint,
        &InitializeMintData {
            authority: authority.try_into()?,
            supply_elgamal_pubkey: confidential_supply_pubkey.try_into()?,
            decryptable_supply,
        },
    ))
}

/// Create a `RotateSupplyElGamal` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_os = "solana"))]
pub fn rotate_supply_elgamal_pubkey(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    new_supply_elgamal_pubkey: ElGamalPubkey,
    ciphertext_equality_proof: ProofLocation<CiphertextCiphertextEqualityProofData>,
) -> Result<Vec<Instruction>, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![AccountMeta::new(*mint, false)];

    let mut expected_instruction_offset = 1;
    let mut proof_instructions = vec![];

    let proof_instruction_offset = process_proof_location(
        &mut accounts,
        &mut expected_instruction_offset,
        &mut proof_instructions,
        ciphertext_equality_proof,
        true,
        ProofInstruction::VerifyCiphertextCiphertextEquality,
    )?;

    accounts.push(AccountMeta::new_readonly(
        *authority,
        multisig_signers.is_empty(),
    ));
    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    let mut instructions = vec![encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialMintBurnExtension,
        ConfidentialMintBurnInstruction::RotateSupplyElGamalPubkey,
        &RotateSupplyElGamalPubkeyData {
            new_supply_elgamal_pubkey: Some(Into::<PodElGamalPubkey>::into(
                new_supply_elgamal_pubkey,
            ))
            .try_into()?,
            proof_instruction_offset,
        },
    )];

    instructions.extend_from_slice(&proof_instructions);

    Ok(instructions)
}

/// Create a `UpdateMint` instruction
#[cfg(not(target_os = "solana"))]
pub fn update_decryptable_supply(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    new_decryptable_supply: AeCiphertext,
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
        ConfidentialMintBurnInstruction::UpdateDecryptableSupply,
        &UpdateDecryptableSupplyData {
            new_decryptable_supply: new_decryptable_supply.into(),
        },
    ))
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
    equality_proof_location: ProofLocation<CiphertextCommitmentEqualityProofData>,
    ciphertext_validity_proof_location: ProofLocation<
        BatchedGroupedCiphertext3HandlesValidityProofData,
    >,
    range_proof_location: ProofLocation<BatchedRangeProofU128Data>,
    new_decryptable_supply: AeCiphertext,
) -> Result<Vec<Instruction>, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![AccountMeta::new(*token_account, false)];
    // we only need write lock to adjust confidential suppy on
    // mint if a value for supply_elgamal_pubkey has been set
    if supply_elgamal_pubkey.is_some() {
        accounts.push(AccountMeta::new(*mint, false));
    } else {
        accounts.push(AccountMeta::new_readonly(*mint, false));
    }

    let mut expected_instruction_offset = 1;
    let mut proof_instructions = vec![];

    let equality_proof_instruction_offset = process_proof_location(
        &mut accounts,
        &mut expected_instruction_offset,
        &mut proof_instructions,
        equality_proof_location,
        true,
        ProofInstruction::VerifyCiphertextCommitmentEquality,
    )?;

    let ciphertext_validity_proof_instruction_offset = process_proof_location(
        &mut accounts,
        &mut expected_instruction_offset,
        &mut proof_instructions,
        ciphertext_validity_proof_location,
        false,
        ProofInstruction::VerifyBatchedGroupedCiphertext3HandlesValidity,
    )?;

    let range_proof_instruction_offset = process_proof_location(
        &mut accounts,
        &mut expected_instruction_offset,
        &mut proof_instructions,
        range_proof_location,
        false,
        ProofInstruction::VerifyBatchedRangeProofU128,
    )?;

    accounts.push(AccountMeta::new_readonly(
        *authority,
        multisig_signers.is_empty(),
    ));
    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    let mut instructions = vec![encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialMintBurnExtension,
        ConfidentialMintBurnInstruction::ConfidentialMint,
        &MintInstructionData {
            new_decryptable_supply: new_decryptable_supply.into(),
            equality_proof_instruction_offset,
            ciphertext_validity_proof_instruction_offset,
            range_proof_instruction_offset,
        },
    )];

    instructions.extend_from_slice(&proof_instructions);

    Ok(instructions)
}

/// Create a inner `ConfidentialBurn` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_os = "solana"))]
pub fn confidential_burn_with_split_proofs(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    supply_elgamal_pubkey: Option<ElGamalPubkey>,
    new_decryptable_available_balance: DecryptableBalance,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    equality_proof_location: ProofLocation<CiphertextCommitmentEqualityProofData>,
    ciphertext_validity_proof_location: ProofLocation<
        BatchedGroupedCiphertext3HandlesValidityProofData,
    >,
    range_proof_location: ProofLocation<BatchedRangeProofU128Data>,
) -> Result<Vec<Instruction>, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![AccountMeta::new(*token_account, false)];
    if supply_elgamal_pubkey.is_some() {
        accounts.push(AccountMeta::new(*mint, false));
    } else {
        accounts.push(AccountMeta::new_readonly(*mint, false));
    }

    let mut expected_instruction_offset = 1;
    let mut proof_instructions = vec![];

    let equality_proof_instruction_offset = process_proof_location(
        &mut accounts,
        &mut expected_instruction_offset,
        &mut proof_instructions,
        equality_proof_location,
        true,
        ProofInstruction::VerifyCiphertextCommitmentEquality,
    )?;

    let ciphertext_validity_proof_instruction_offset = process_proof_location(
        &mut accounts,
        &mut expected_instruction_offset,
        &mut proof_instructions,
        ciphertext_validity_proof_location,
        false,
        ProofInstruction::VerifyBatchedGroupedCiphertext3HandlesValidity,
    )?;

    let range_proof_instruction_offset = process_proof_location(
        &mut accounts,
        &mut expected_instruction_offset,
        &mut proof_instructions,
        range_proof_location,
        false,
        ProofInstruction::VerifyBatchedRangeProofU128,
    )?;

    accounts.push(AccountMeta::new_readonly(
        *authority,
        multisig_signers.is_empty(),
    ));

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    let mut instructions = vec![encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialMintBurnExtension,
        ConfidentialMintBurnInstruction::ConfidentialBurn,
        &BurnInstructionData {
            new_decryptable_available_balance,
            equality_proof_instruction_offset,
            ciphertext_validity_proof_instruction_offset,
            range_proof_instruction_offset,
        },
    )];

    instructions.extend_from_slice(&proof_instructions);

    Ok(instructions)
}