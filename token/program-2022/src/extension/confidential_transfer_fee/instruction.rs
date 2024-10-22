#[cfg(feature = "serde-traits")]
use {
    crate::serialization::{aeciphertext_fromstr, elgamalpubkey_fromstr},
    serde::{Deserialize, Serialize},
};
use {
    crate::{
        check_program_account,
        error::TokenError,
        extension::confidential_transfer::{
            instruction::CiphertextCiphertextEqualityProofData, DecryptableBalance,
        },
        instruction::{encode_instruction, TokenInstruction},
        solana_zk_sdk::{
            encryption::pod::elgamal::PodElGamalPubkey,
            zk_elgamal_proof_program::instruction::ProofInstruction,
        },
    },
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvar,
    },
    spl_pod::optional_keys::OptionalNonZeroPubkey,
    spl_token_confidential_transfer_proof_extraction::instruction::{ProofData, ProofLocation},
    std::convert::TryFrom,
};

/// Confidential Transfer extension instructions
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ConfidentialTransferFeeInstruction {
    /// Initializes confidential transfer fees for a mint.
    ///
    /// The `ConfidentialTransferFeeInstruction::InitializeConfidentialTransferFeeConfig`
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
    ///   `InitializeConfidentialTransferFeeConfigData`
    InitializeConfidentialTransferFeeConfig,

    /// Transfer all withheld confidential tokens in the mint to an account.
    /// Signed by the mint's withdraw withheld tokens authority.
    ///
    /// The withheld confidential tokens are aggregated directly into the
    /// destination available balance.
    ///
    /// In order for this instruction to be successfully processed, it must be
    /// accompanied by the `VerifyCiphertextCiphertextEquality` instruction
    /// of the `zk_elgamal_proof` program in the same transaction or the
    /// address of a context state account for the proof must be provided.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The token mint. Must include the `TransferFeeConfig`
    ///      extension.
    ///   1. `[writable]` The fee receiver account. Must include the
    ///      `TransferFeeAmount` and `ConfidentialTransferAccount` extensions.
    ///   2. `[]` Instructions sysvar if `VerifyCiphertextCiphertextEquality` is
    ///      included in the same transaction or context state account if
    ///      `VerifyCiphertextCiphertextEquality` is pre-verified into a context
    ///      state account.
    ///   3. `[]` (Optional) Record account if the accompanying proof is to be
    ///      read from a record account.
    ///   4. `[signer]` The mint's `withdraw_withheld_authority`.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The token mint. Must include the `TransferFeeConfig`
    ///      extension.
    ///   1. `[writable]` The fee receiver account. Must include the
    ///      `TransferFeeAmount` and `ConfidentialTransferAccount` extensions.
    ///   2. `[]` Instructions sysvar if `VerifyCiphertextCiphertextEquality` is
    ///      included in the same transaction or context state account if
    ///      `VerifyCiphertextCiphertextEquality` is pre-verified into a context
    ///      state account.
    ///   3. `[]` (Optional) Record account if the accompanying proof is to be
    ///      read from a record account.
    ///   4. `[]` The mint's multisig `withdraw_withheld_authority`.
    ///   5. ..3+M `[signer]` M signer accounts.
    ///
    /// Data expected by this instruction:
    ///   WithdrawWithheldTokensFromMintData
    WithdrawWithheldTokensFromMint,

    /// Transfer all withheld tokens to an account. Signed by the mint's
    /// withdraw withheld tokens authority. This instruction is susceptible
    /// to front-running. Use `HarvestWithheldTokensToMint` and
    /// `WithdrawWithheldTokensFromMint` as an alternative.
    ///
    /// The withheld confidential tokens are aggregated directly into the
    /// destination available balance.
    ///
    /// Note on front-running: This instruction requires a zero-knowledge proof
    /// verification instruction that is checked with respect to the account
    /// state (the currently withheld fees). Suppose that a withdraw
    /// withheld authority generates the
    /// `WithdrawWithheldTokensFromAccounts` instruction along with a
    /// corresponding zero-knowledge proof for a specified set of accounts,
    /// and submits it on chain. If the withheld fees at any
    /// of the specified accounts change before the
    /// `WithdrawWithheldTokensFromAccounts` is executed on chain, the
    /// zero-knowledge proof will not verify with respect to the new state,
    /// forcing the transaction to fail.
    ///
    /// If front-running occurs, then users can look up the updated states of
    /// the accounts, generate a new zero-knowledge proof and try again.
    /// Alternatively, withdraw withheld authority can first move the
    /// withheld amount to the mint using `HarvestWithheldTokensToMint` and
    /// then move the withheld fees from mint to a specified destination
    /// account using `WithdrawWithheldTokensFromMint`.
    ///
    /// In order for this instruction to be successfully processed, it must be
    /// accompanied by the `VerifyWithdrawWithheldTokens` instruction of the
    /// `zk_elgamal_proof` program in the same transaction or the address of a
    /// context state account for the proof must be provided.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[]` The token mint. Must include the `TransferFeeConfig`
    ///      extension.
    ///   1. `[writable]` The fee receiver account. Must include the
    ///      `TransferFeeAmount` and `ConfidentialTransferAccount` extensions.
    ///   2. `[]` Instructions sysvar if `VerifyCiphertextCiphertextEquality` is
    ///      included in the same transaction or context state account if
    ///      `VerifyCiphertextCiphertextEquality` is pre-verified into a context
    ///      state account.
    ///   3. `[]` (Optional) Record account if the accompanying proof is to be
    ///      read from a record account.
    ///   4. `[signer]` The mint's `withdraw_withheld_authority`.
    ///   5. ..3+N `[writable]` The source accounts to withdraw from.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[]` The token mint. Must include the `TransferFeeConfig`
    ///      extension.
    ///   1. `[writable]` The fee receiver account. Must include the
    ///      `TransferFeeAmount` and `ConfidentialTransferAccount` extensions.
    ///   2. `[]` Instructions sysvar if `VerifyCiphertextCiphertextEquality` is
    ///      included in the same transaction or context state account if
    ///      `VerifyCiphertextCiphertextEquality` is pre-verified into a context
    ///      state account.
    ///   3. `[]` (Optional) Record account if the accompanying proof is to be
    ///      read from a record account.
    ///   4. `[]` The mint's multisig `withdraw_withheld_authority`.
    ///   5. ..5+M `[signer]` M signer accounts.
    ///   5+M+1. ..5+M+N `[writable]` The source accounts to withdraw from.
    ///
    /// Data expected by this instruction:
    ///   WithdrawWithheldTokensFromAccountsData
    WithdrawWithheldTokensFromAccounts,

    /// Permissionless instruction to transfer all withheld confidential tokens
    /// to the mint.
    ///
    /// Succeeds for frozen accounts.
    ///
    /// Accounts provided should include both the `TransferFeeAmount` and
    /// `ConfidentialTransferAccount` extension. If not, the account is skipped.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The mint.
    ///   1. ..1+N `[writable]` The source accounts to harvest from.
    ///
    /// Data expected by this instruction:
    ///   None
    HarvestWithheldTokensToMint,

    /// Configure a confidential transfer fee mint to accept harvested
    /// confidential fees.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The token mint.
    ///   1. `[signer]` The confidential transfer fee authority.
    ///
    ///   *Multisignature owner/delegate
    ///   0. `[writable]` The token mint.
    ///   1. `[]` The confidential transfer fee multisig authority,
    ///   2. `[signer]` Required M signer accounts for the SPL Token Multisig
    ///      account.
    ///
    /// Data expected by this instruction:
    ///   None
    EnableHarvestToMint,

    /// Configure a confidential transfer fee mint to reject any harvested
    /// confidential fees.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The token mint.
    ///   1. `[signer]` The confidential transfer fee authority.
    ///
    ///   *Multisignature owner/delegate
    ///   0. `[writable]` The token mint.
    ///   1. `[]` The confidential transfer fee multisig authority,
    ///   2. `[signer]` Required M signer accounts for the SPL Token Multisig
    ///      account.
    ///
    /// Data expected by this instruction:
    ///   None
    DisableHarvestToMint,
}

/// Data expected by `InitializeConfidentialTransferFeeConfig`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct InitializeConfidentialTransferFeeConfigData {
    /// confidential transfer fee authority
    pub authority: OptionalNonZeroPubkey,

    /// ElGamal public key used to encrypt withheld fees.
    #[cfg_attr(feature = "serde-traits", serde(with = "elgamalpubkey_fromstr"))]
    pub withdraw_withheld_authority_elgamal_pubkey: PodElGamalPubkey,
}

/// Data expected by
/// `ConfidentialTransferFeeInstruction::WithdrawWithheldTokensFromMint`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct WithdrawWithheldTokensFromMintData {
    /// Relative location of the `ProofInstruction::VerifyWithdrawWithheld`
    /// instruction to the `WithdrawWithheldTokensFromMint` instruction in
    /// the transaction. If the offset is `0`, then use a context state
    /// account for the proof.
    pub proof_instruction_offset: i8,
    /// The new decryptable balance in the destination token account.
    #[cfg_attr(feature = "serde-traits", serde(with = "aeciphertext_fromstr"))]
    pub new_decryptable_available_balance: DecryptableBalance,
}

/// Data expected by
/// `ConfidentialTransferFeeInstruction::WithdrawWithheldTokensFromAccounts`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct WithdrawWithheldTokensFromAccountsData {
    /// Number of token accounts harvested
    pub num_token_accounts: u8,
    /// Relative location of the `ProofInstruction::VerifyWithdrawWithheld`
    /// instruction to the `VerifyWithdrawWithheldTokensFromAccounts`
    /// instruction in the transaction. If the offset is `0`, then use a
    /// context state account for the proof.
    pub proof_instruction_offset: i8,
    /// The new decryptable balance in the destination token account.
    #[cfg_attr(feature = "serde-traits", serde(with = "aeciphertext_fromstr"))]
    pub new_decryptable_available_balance: DecryptableBalance,
}

/// Create a `InitializeConfidentialTransferFeeConfig` instruction
pub fn initialize_confidential_transfer_fee_config(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: Option<Pubkey>,
    withdraw_withheld_authority_elgamal_pubkey: PodElGamalPubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![AccountMeta::new(*mint, false)];

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferFeeExtension,
        ConfidentialTransferFeeInstruction::InitializeConfidentialTransferFeeConfig,
        &InitializeConfidentialTransferFeeConfigData {
            authority: authority.try_into()?,
            withdraw_withheld_authority_elgamal_pubkey,
        },
    ))
}

/// Create an inner `WithdrawWithheldTokensFromMint` instruction
///
/// This instruction is suitable for use with a cross-program `invoke`
pub fn inner_withdraw_withheld_tokens_from_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    new_decryptable_available_balance: &DecryptableBalance,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data_location: ProofLocation<CiphertextCiphertextEqualityProofData>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new(*destination, false),
    ];

    let proof_instruction_offset = match proof_data_location {
        ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) => {
            accounts.push(AccountMeta::new_readonly(sysvar::instructions::id(), false));
            if let ProofData::RecordAccount(record_address, _) = proof_data {
                accounts.push(AccountMeta::new_readonly(*record_address, false));
            }
            proof_instruction_offset.into()
        }
        ProofLocation::ContextStateAccount(context_state_account) => {
            accounts.push(AccountMeta::new_readonly(*context_state_account, false));
            0
        }
    };

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
        TokenInstruction::ConfidentialTransferFeeExtension,
        ConfidentialTransferFeeInstruction::WithdrawWithheldTokensFromMint,
        &WithdrawWithheldTokensFromMintData {
            proof_instruction_offset,
            new_decryptable_available_balance: *new_decryptable_available_balance,
        },
    ))
}

/// Create an `WithdrawWithheldTokensFromMint` instruction
pub fn withdraw_withheld_tokens_from_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    new_decryptable_available_balance: &DecryptableBalance,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data_location: ProofLocation<CiphertextCiphertextEqualityProofData>,
) -> Result<Vec<Instruction>, ProgramError> {
    let mut instructions = vec![inner_withdraw_withheld_tokens_from_mint(
        token_program_id,
        mint,
        destination,
        new_decryptable_available_balance,
        authority,
        multisig_signers,
        proof_data_location,
    )?];

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        proof_data_location
    {
        // This constructor appends the proof instruction right after the
        // `WithdrawWithheldTokensFromMint` instruction. This means that the proof
        // instruction offset must be always be 1. To use an arbitrary proof
        // instruction offset, use the
        // `inner_withdraw_withheld_tokens_from_mint` constructor.
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != 1 {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        match proof_data {
            ProofData::InstructionData(data) => instructions.push(
                ProofInstruction::VerifyCiphertextCiphertextEquality
                    .encode_verify_proof(None, data),
            ),
            ProofData::RecordAccount(address, offset) => instructions.push(
                ProofInstruction::VerifyCiphertextCiphertextEquality
                    .encode_verify_proof_from_account(None, address, offset),
            ),
        };
    };

    Ok(instructions)
}

/// Create an inner `WithdrawWithheldTokensFromMint` instruction
///
/// This instruction is suitable for use with a cross-program `invoke`
#[allow(clippy::too_many_arguments)]
pub fn inner_withdraw_withheld_tokens_from_accounts(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    new_decryptable_available_balance: &DecryptableBalance,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    sources: &[&Pubkey],
    proof_data_location: ProofLocation<CiphertextCiphertextEqualityProofData>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let num_token_accounts =
        u8::try_from(sources.len()).map_err(|_| ProgramError::InvalidInstructionData)?;
    let mut accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new(*destination, false),
    ];

    let proof_instruction_offset = match proof_data_location {
        ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) => {
            accounts.push(AccountMeta::new_readonly(sysvar::instructions::id(), false));
            if let ProofData::RecordAccount(record_address, _) = proof_data {
                accounts.push(AccountMeta::new_readonly(*record_address, false));
            }
            proof_instruction_offset.into()
        }
        ProofLocation::ContextStateAccount(context_state_account) => {
            accounts.push(AccountMeta::new_readonly(*context_state_account, false));
            0
        }
    };

    accounts.push(AccountMeta::new_readonly(
        *authority,
        multisig_signers.is_empty(),
    ));

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    for source in sources.iter() {
        accounts.push(AccountMeta::new(**source, false));
    }

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferFeeExtension,
        ConfidentialTransferFeeInstruction::WithdrawWithheldTokensFromAccounts,
        &WithdrawWithheldTokensFromAccountsData {
            proof_instruction_offset,
            num_token_accounts,
            new_decryptable_available_balance: *new_decryptable_available_balance,
        },
    ))
}

/// Create a `WithdrawWithheldTokensFromAccounts` instruction
#[allow(clippy::too_many_arguments)]
pub fn withdraw_withheld_tokens_from_accounts(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    new_decryptable_available_balance: &DecryptableBalance,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    sources: &[&Pubkey],
    proof_data_location: ProofLocation<CiphertextCiphertextEqualityProofData>,
) -> Result<Vec<Instruction>, ProgramError> {
    let mut instructions = vec![inner_withdraw_withheld_tokens_from_accounts(
        token_program_id,
        mint,
        destination,
        new_decryptable_available_balance,
        authority,
        multisig_signers,
        sources,
        proof_data_location,
    )?];

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        proof_data_location
    {
        // This constructor appends the proof instruction right after the
        // `WithdrawWithheldTokensFromAccounts` instruction. This means that the proof
        // instruction offset must always be 1. To use an arbitrary proof
        // instruction offset, use the
        // `inner_withdraw_withheld_tokens_from_accounts` constructor.
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != 1 {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        match proof_data {
            ProofData::InstructionData(data) => instructions.push(
                ProofInstruction::VerifyCiphertextCiphertextEquality
                    .encode_verify_proof(None, data),
            ),
            ProofData::RecordAccount(address, offset) => instructions.push(
                ProofInstruction::VerifyCiphertextCiphertextEquality
                    .encode_verify_proof_from_account(None, address, offset),
            ),
        };
    };

    Ok(instructions)
}

/// Creates a `HarvestWithheldTokensToMint` instruction
pub fn harvest_withheld_tokens_to_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    sources: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![AccountMeta::new(*mint, false)];

    for source in sources.iter() {
        accounts.push(AccountMeta::new(**source, false));
    }

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferFeeExtension,
        ConfidentialTransferFeeInstruction::HarvestWithheldTokensToMint,
        &(),
    ))
}

/// Create an `EnableHarvestToMint` instruction
pub fn enable_harvest_to_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
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
        TokenInstruction::ConfidentialTransferFeeExtension,
        ConfidentialTransferFeeInstruction::EnableHarvestToMint,
        &(),
    ))
}

/// Create a `DisableHarvestToMint` instruction
pub fn disable_harvest_to_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
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
        TokenInstruction::ConfidentialTransferFeeExtension,
        ConfidentialTransferFeeInstruction::DisableHarvestToMint,
        &(),
    ))
}
