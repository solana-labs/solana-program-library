#[cfg(feature = "proof-program")]
use crate::extension::confidential_transfer::instruction::{
    verify_withdraw_withheld_tokens, WithdrawWithheldTokensData,
};
use {
    crate::{
        check_program_account,
        instruction::{encode_instruction, TokenInstruction},
        pod::{EncryptionPubkey, OptionalNonZeroPubkey},
    },
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvar,
    },
    std::convert::TryFrom,
};

/// Confidential Transfer extension instructions
#[derive(Clone, Copy, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ConfidentialTransferFeeInstruction {
    /// Initializes confidential transfer fees for a mint.
    ///
    /// The `ConfidentialTransferFeeInstruction::InitializeConfidentialTransferFeeConfig`
    /// instruction requires no signers and MUST be included within the same Transaction as
    /// `TokenInstruction::InitializeMint`. Otherwise another party can initialize the
    /// configuration.
    ///
    /// The instruction fails if the `TokenInstruction::InitializeMint` instruction has already
    /// executed for the mint.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token mint.
    ///
    /// Data expected by this instruction:
    ///   `InitializeConfidentialTransferFeeConfigData`
    ///
    InitializeConfidentialTransferFeeConfig,

    /// Transfer all withheld confidential tokens in the mint to an account. Signed by the mint's
    /// withdraw withheld tokens authority.
    ///
    /// In order for this instruction to be successfully processed, it must be accompanied by the
    /// `VerifyWithdrawWithheldTokens` instruction of the `zk_token_proof` program in the same
    /// transaction.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The token mint. Must include the `TransferFeeConfig` extension.
    ///   1. `[writable]` The fee receiver account. Must include the `TransferFeeAmount` and
    ///      `ConfidentialTransferAccount` extensions.
    ///   2. `[]` Instructions sysvar.
    ///   3. `[signer]` The mint's `withdraw_withheld_authority`.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The token mint. Must include the `TransferFeeConfig` extension.
    ///   1. `[writable]` The fee receiver account. Must include the `TransferFeeAmount` and
    ///      `ConfidentialTransferAccount` extensions.
    ///   2. `[]` Instructions sysvar.
    ///   3. `[]` The mint's multisig `withdraw_withheld_authority`.
    ///   4. ..3+M `[signer]` M signer accounts.
    ///
    /// Data expected by this instruction:
    ///   WithdrawWithheldTokensFromMintData
    ///
    WithdrawWithheldTokensFromMint,

    /// Transfer all withheld tokens to an account. Signed by the mint's withdraw withheld tokens
    /// authority. This instruction is susceptible to front-running. Use
    /// `HarvestWithheldTokensToMint` and `WithdrawWithheldTokensFromMint` as an alternative.
    ///
    /// Note on front-running: This instruction requires a zero-knowledge proof verification
    /// instruction that is checked with respect to the account state (the currently withheld
    /// fees). Suppose that a withdraw withheld authority generates the
    /// `WithdrawWithheldTokensFromAccounts` instruction along with a corresponding zero-knowledge
    /// proof for a specified set of accounts, and submits it on chain. If the withheld fees at any
    /// of the specified accounts change before the `WithdrawWithheldTokensFromAccounts` is
    /// executed on chain, the zero-knowledge proof will not verify with respect to the new state,
    /// forcing the transaction to fail.
    ///
    /// If front-running occurs, then users can look up the updated states of the accounts,
    /// generate a new zero-knowledge proof and try again. Alternatively, withdraw withheld
    /// authority can first move the withheld amount to the mint using
    /// `HarvestWithheldTokensToMint` and then move the withheld fees from mint to a specified
    /// destination account using `WithdrawWithheldTokensFromMint`.
    ///
    /// In order for this instruction to be successfully processed, it must be accompanied by the
    /// `VerifyWithdrawWithheldTokens` instruction of the `zk_token_proof` program in the same
    /// transaction.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[]` The token mint. Must include the `TransferFeeConfig` extension.
    ///   1. `[writable]` The fee receiver account. Must include the `TransferFeeAmount` and
    ///      `ConfidentialTransferAccount` extensions.
    ///   2. `[]` Instructions sysvar.
    ///   3. `[signer]` The mint's `withdraw_withheld_authority`.
    ///   4. ..3+N `[writable]` The source accounts to withdraw from.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[]` The token mint. Must include the `TransferFeeConfig` extension.
    ///   1. `[writable]` The fee receiver account. Must include the `TransferFeeAmount` and
    ///      `ConfidentialTransferAccount` extensions.
    ///   2. `[]` Instructions sysvar.
    ///   3. `[]` The mint's multisig `withdraw_withheld_authority`.
    ///   4. ..4+M `[signer]` M signer accounts.
    ///   4+M+1. ..3+M+N `[writable]` The source accounts to withdraw from.
    ///
    /// Data expected by this instruction:
    ///   WithdrawWithheldTokensFromAccountsData
    ///
    WithdrawWithheldTokensFromAccounts,

    /// Permissionless instruction to transfer all withheld confidential tokens to the mint.
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
    ///
    HarvestWithheldTokensToMint,
}

/// Data expected by `InitializeConfidentialTransferFeeConfig`
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct InitializeConfidentialTransferFeeConfigData {
    /// confidential transfer fee authority
    pub authority: OptionalNonZeroPubkey,

    /// ElGamal public key used to encrypt withheld fees.
    pub withdraw_withheld_authority_encryption_pubkey: EncryptionPubkey,
}

/// Data expected by `ConfidentialTransferInstruction::ApplyPendingBalance`
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct ApplyPendingBalanceData {
    /// The expected number of pending balance credits since the last successful
    /// `ApplyPendingBalance` instruction
    pub expected_pending_balance_credit_counter: PodU64,
    /// The new decryptable balance if the pending balance is applied successfully
    pub new_decryptable_available_balance: pod::AeCiphertext,
}

/// Data expected by `ConfidentialTransferInstruction::WithdrawWithheldTokensFromMint`
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct WithdrawWithheldTokensFromMintData {
    /// Relative location of the `ProofInstruction::VerifyWithdrawWithheld` instruction to the
    /// `WithdrawWithheldTokensFromMint` instruction in the transaction
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialTransferInstruction::WithdrawWithheldTokensFromAccounts`
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct WithdrawWithheldTokensFromAccountsData {
    /// Number of token accounts harvested
    pub num_token_accounts: u8,
    /// Relative location of the `ProofInstruction::VerifyWithdrawWithheld` instruction to the
    /// `VerifyWithdrawWithheldTokensFromAccounts` instruction in the transaction
    pub proof_instruction_offset: i8,
}

/// Create a `InitializeConfidentialTransferFeeConfig` instruction
pub fn initialize_confidential_transfer_fee_config(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: Option<Pubkey>,
    withdraw_withheld_authority_encryption_pubkey: EncryptionPubkey,
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
            withdraw_withheld_authority_encryption_pubkey,
        },
    ))
}

/// Create a inner `WithdrawWithheldTokensFromMint` instruction
///
/// This instruction is suitable for use with a cross-program `invoke`
pub fn inner_withdraw_withheld_tokens_from_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_instruction_offset: i8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new(*destination, false),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new(**multisig_signer, false));
    }

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::WithdrawWithheldTokensFromMint,
        &WithdrawWithheldTokensFromMintData {
            proof_instruction_offset,
        },
    ))
}

/// Create a `WithdrawWithheldTokensFromMint` instruction
#[cfg(feature = "proof-program")]
pub fn withdraw_withheld_tokens_from_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data: &WithdrawWithheldTokensData,
) -> Result<Vec<Instruction>, ProgramError> {
    Ok(vec![
        inner_withdraw_withheld_tokens_from_mint(
            token_program_id,
            mint,
            destination,
            authority,
            multisig_signers,
            1,
        )?,
        #[cfg(feature = "proof-program")]
        verify_withdraw_withheld_tokens(proof_data),
    ])
}

/// Create a inner `WithdrawWithheldTokensFromMint` instruction
///
/// This instruction is suitable for use with a cross-program `invoke`
pub fn inner_withdraw_withheld_tokens_from_accounts(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    sources: &[&Pubkey],
    proof_instruction_offset: i8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let num_token_accounts =
        u8::try_from(sources.len()).map_err(|_| ProgramError::InvalidInstructionData)?;
    let mut accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new(*destination, false),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new(**multisig_signer, false));
    }

    for source in sources.iter() {
        accounts.push(AccountMeta::new(**source, false));
    }

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::WithdrawWithheldTokensFromAccounts,
        &WithdrawWithheldTokensFromAccountsData {
            proof_instruction_offset,
            num_token_accounts,
        },
    ))
}

/// Create a `WithdrawWithheldTokensFromAccounts` instruction
#[cfg(feature = "proof-program")]
pub fn withdraw_withheld_tokens_from_accounts(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    sources: &[&Pubkey],
    proof_data: &WithdrawWithheldTokensData,
) -> Result<Vec<Instruction>, ProgramError> {
    Ok(vec![
        inner_withdraw_withheld_tokens_from_accounts(
            token_program_id,
            mint,
            destination,
            authority,
            multisig_signers,
            sources,
            1,
        )?,
        #[cfg(feature = "proof-program")]
        verify_withdraw_withheld_tokens(proof_data),
    ])
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
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::HarvestWithheldTokensToMint,
        &(),
    ))
}
