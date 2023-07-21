#[cfg(not(target_os = "solana"))]
use solana_zk_token_sdk::encryption::auth_encryption::AeCiphertext;
pub use solana_zk_token_sdk::{
    zk_token_proof_instruction::*, zk_token_proof_state::ProofContextState,
};
use {
    crate::{
        check_program_account,
        extension::confidential_transfer::*,
        instruction::{encode_instruction, TokenInstruction},
    },
    bytemuck::{Pod, Zeroable},
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvar,
    },
};

/// Confidential Transfer extension instructions
#[derive(Clone, Copy, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ConfidentialTransferInstruction {
    /// Initializes confidential transfers for a mint.
    ///
    /// The `ConfidentialTransferInstruction::InitializeMint` instruction requires no signers
    /// and MUST be included within the same Transaction as `TokenInstruction::InitializeMint`.
    /// Otherwise another party can initialize the configuration.
    ///
    /// The instruction fails if the `TokenInstruction::InitializeMint` instruction has already
    /// executed for the mint.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token mint.
    ///
    /// Data expected by this instruction:
    ///   `InitializeMintData`
    ///
    InitializeMint,

    /// Updates the confidential transfer mint configuration for a mint.
    ///
    /// Use `TokenInstruction::SetAuthority` to update the confidential transfer mint authority.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token mint.
    ///   1. `[signer]` Confidential transfer mint authority.
    ///
    /// Data expected by this instruction:
    ///   `UpdateMintData`
    ///
    UpdateMint,

    /// Configures confidential transfers for a token account.
    ///
    /// The instruction fails if the confidential transfers are already configured, or if the mint
    /// was not initialized with confidential transfer support.
    ///
    /// The instruction fails if the `TokenInstruction::InitializeAccount` instruction has not yet
    /// successfully executed for the token account.
    ///
    /// Upon success, confidential and non-confidential deposits and transfers are enabled. Use the
    /// `DisableConfidentialCredits` and `DisableNonConfidentialCredits` instructions to disable.
    ///
    /// In order for this instruction to be successfully processed, it must be accompanied by the
    /// `VerifyPubkey` instruction of the `zk_token_proof` program in the same transaction.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writeable]` The SPL Token account.
    ///   1. `[]` The corresponding SPL Token mint.
    ///   2. `[]` Instructions sysvar.
    ///   3. `[]` Context state account for `ZeroBalanceProof` (optional)
    ///   4. `[signer]` The single source account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writeable]` The SPL Token account.
    ///   1. `[]` The corresponding SPL Token mint.
    ///   2. `[]` Instructions sysvar.
    ///   3. `[]` Context state account `ZeroBalanceProof` (optional)
    ///   4. `[]` The multisig source account owner.
    ///   5.. `[signer]` Required M signer accounts for the SPL Token Multisig account.
    ///
    /// Data expected by this instruction:
    ///   `ConfigureAccountInstructionData`
    ///
    ConfigureAccount,

    /// Approves a token account for confidential transfers.
    ///
    /// Approval is only required when the `ConfidentialTransferMint::approve_new_accounts`
    /// field is set in the SPL Token mint.  This instruction must be executed after the account
    /// owner configures their account for confidential transfers with
    /// `ConfidentialTransferInstruction::ConfigureAccount`.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token account to approve.
    ///   1. `[]` The SPL Token mint.
    ///   2. `[signer]` Confidential transfer auditor authority.
    ///
    /// Data expected by this instruction:
    ///   None
    ///
    ApproveAccount,

    /// Empty the available balance in a confidential token account.
    ///
    /// A token account that is extended for confidential transfers can only be closed if the
    /// pending and available balance ciphertexts are emptied. The pending balance can be emptied
    /// via the `ConfidentialTransferInstruction::ApplyPendingBalance` instruction. Use the
    /// `ConfidentialTransferInstruction::EmptyAccount` instruction to empty the available balance
    /// ciphertext.
    ///
    /// Note that a newly configured account is always empty, so this instruction is not required
    /// prior to account closing if no instructions beyond
    /// `ConfidentialTransferInstruction::ConfigureAccount` have affected the token account.
    ///
    /// In order for this instruction to be successfully processed, it must be accompanied by the
    /// `VerifyCloseAccount` instruction of the `zk_token_proof` program in the same transaction.
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` Instructions sysvar.
    ///   2. `[]` Context state account for `ZeroBalanceProof` (optional)
    ///   3. `[signer]` The single account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` Instructions sysvar.
    ///   2. `[]` The multisig account owner.
    ///   3. `[]` Context state account for `ZeroBalanceProof` (optional)
    ///   4.. `[signer]` Required M signer accounts for the SPL Token Multisig account.
    ///
    /// Data expected by this instruction:
    ///   `EmptyAccountInstructionData`
    ///
    EmptyAccount,

    /// Deposit SPL Tokens into the pending balance of a confidential token account.
    ///
    /// The account owner can then invoke the `ApplyPendingBalance` instruction to roll the deposit
    /// into their available balance at a time of their choosing.
    ///
    /// Fails if the source or destination accounts are frozen.
    /// Fails if the associated mint is extended as `NonTransferable`.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` The token mint.
    ///   2. `[signer]` The single account owner or delegate.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` The token mint.
    ///   2. `[]` The multisig account owner or delegate.
    ///   3.. `[signer]` Required M signer accounts for the SPL Token Multisig account.
    ///
    /// Data expected by this instruction:
    ///   `DepositInstructionData`
    ///
    Deposit,

    /// Withdraw SPL Tokens from the available balance of a confidential token account.
    ///
    /// Fails if the source or destination accounts are frozen.
    /// Fails if the associated mint is extended as `NonTransferable`.
    ///
    /// In order for this instruction to be successfully processed, it must be accompanied by the
    /// `VerifyWithdraw` instruction of the `zk_token_proof` program in the same transaction.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` The token mint.
    ///   2. `[]` Instructions sysvar.
    ///   3. `[signer]` The single source account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` The token mint.
    ///   2. `[]` Instructions sysvar.
    ///   3. `[]` The multisig  source account owner.
    ///   4.. `[signer]` Required M signer accounts for the SPL Token Multisig account.
    ///
    /// Data expected by this instruction:
    ///   `WithdrawInstructionData`
    ///
    Withdraw,

    /// Transfer tokens confidentially.
    ///
    /// In order for this instruction to be successfully processed, it must be accompanied by
    /// either the `VerifyTransfer` or `VerifyTransferWithFee` instruction of the `zk_token_proof`
    /// program in the same transaction.
    ///
    /// Fails if the associated mint is extended as `NonTransferable`.
    ///
    ///   * Single owner/delegate
    ///   1. `[writable]` The source SPL Token account.
    ///   2. `[writable]` The destination SPL Token account.
    ///   3. `[]` The token mint.
    ///   4. `[]` Instructions sysvar.
    ///   5. `[signer]` The single source account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   1. `[writable]` The source SPL Token account.
    ///   2. `[writable]` The destination SPL Token account.
    ///   3. `[]` The token mint.
    ///   4. `[]` Instructions sysvar.
    ///   5. `[]` The multisig  source account owner.
    ///   6.. `[signer]` Required M signer accounts for the SPL Token Multisig account.
    ///
    /// Data expected by this instruction:
    ///   `TransferInstructionData`
    ///
    Transfer,

    /// Applies the pending balance to the available balance, based on the history of `Deposit`
    /// and/or `Transfer` instructions.
    ///
    /// After submitting `ApplyPendingBalance`, the client should compare
    /// `ConfidentialTransferAccount::expected_pending_balance_credit_counter` with
    /// `ConfidentialTransferAccount::actual_applied_pending_balance_instructions`.  If they are
    /// equal then the `ConfidentialTransferAccount::decryptable_available_balance` is consistent
    /// with `ConfidentialTransferAccount::available_balance`. If they differ then there is more
    /// pending balance to be applied.
    ///
    /// Account expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[signer]` The single account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` The multisig account owner.
    ///   2.. `[signer]` Required M signer accounts for the SPL Token Multisig account.
    ///
    /// Data expected by this instruction:
    ///   `ApplyPendingBalanceData`
    ///
    ApplyPendingBalance,

    /// Configure a confidential extension account to accept incoming confidential transfers.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[signer]` Single authority.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` Multisig authority.
    ///   2.. `[signer]` Required M signer accounts for the SPL Token Multisig account.
    ///
    /// Data expected by this instruction:
    ///   None
    ///
    EnableConfidentialCredits,

    /// Configure a confidential extension account to reject any incoming confidential transfers.
    ///
    /// If the `allow_non_confidential_credits` field is `true`, then the base account can still
    /// receive non-confidential transfers.
    ///
    /// This instruction can be used to disable confidential payments after a token account has
    /// already been extended for confidential transfers.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[signer]` The single account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` The multisig account owner.
    ///   2.. `[signer]` Required M signer accounts for the SPL Token Multisig account.
    ///
    /// Data expected by this instruction:
    ///   None
    ///
    DisableConfidentialCredits,

    /// Configure an account with the confidential extension to accept incoming non-confidential
    /// transfers.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[signer]` The single account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` The multisig account owner.
    ///   2.. `[signer]` Required M signer accounts for the SPL Token Multisig account.
    ///
    /// Data expected by this instruction:
    ///   None
    ///
    EnableNonConfidentialCredits,

    /// Configure an account with the confidential extension to reject any incoming
    /// non-confidential transfers.
    ///
    /// This instruction can be used to configure a confidential extension account to exclusively
    /// receive confidential payments.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[signer]` The single account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` The multisig account owner.
    ///   2.. `[signer]` Required M signer accounts for the SPL Token Multisig account.
    ///
    /// Data expected by this instruction:
    ///   None
    ///
    DisableNonConfidentialCredits,
}

/// Data expected by `ConfidentialTransferInstruction::InitializeMint`
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct InitializeMintData {
    /// Authority to modify the `ConfidentialTransferMint` configuration and to approve new
    /// accounts.
    pub authority: OptionalNonZeroPubkey,
    /// Determines if newly configured accounts must be approved by the `authority` before they may
    /// be used by the user.
    pub auto_approve_new_accounts: PodBool,
    /// New authority to decode any transfer amount in a confidential transfer.
    pub auditor_elgamal_pubkey: OptionalNonZeroElGamalPubkey,
}

/// Data expected by `ConfidentialTransferInstruction::UpdateMint`
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct UpdateMintData {
    /// Determines if newly configured accounts must be approved by the `authority` before they may
    /// be used by the user.
    pub auto_approve_new_accounts: PodBool,
    /// New authority to decode any transfer amount in a confidential transfer.
    pub auditor_elgamal_pubkey: OptionalNonZeroElGamalPubkey,
}

/// Data expected by `ConfidentialTransferInstruction::ConfigureAccount`
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct ConfigureAccountInstructionData {
    /// The decryptable balance (always 0) once the configure account succeeds
    pub decryptable_zero_balance: DecryptableBalance,
    /// The maximum number of despots and transfers that an account can receiver before the
    /// `ApplyPendingBalance` is executed
    pub maximum_pending_balance_credit_counter: PodU64,
    /// Relative location of the `ProofInstruction::ZeroBalanceProof` instruction to the
    /// `ConfigureAccount` instruction in the transaction. If the offset is `0`, then use a context
    /// state account for the proof.
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialTransferInstruction::EmptyAccount`
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct EmptyAccountInstructionData {
    /// Relative location of the `ProofInstruction::VerifyCloseAccount` instruction to the
    /// `EmptyAccount` instruction in the transaction. If the offset is `0`, then use a context
    /// state account for the proof.
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialTransferInstruction::Deposit`
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct DepositInstructionData {
    /// The amount of tokens to deposit
    pub amount: PodU64,
    /// Expected number of base 10 digits to the right of the decimal place
    pub decimals: u8,
}

/// Data expected by `ConfidentialTransferInstruction::Withdraw`
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct WithdrawInstructionData {
    /// The amount of tokens to withdraw
    pub amount: PodU64,
    /// Expected number of base 10 digits to the right of the decimal place
    pub decimals: u8,
    /// The new decryptable balance if the withdrawal succeeds
    pub new_decryptable_available_balance: DecryptableBalance,
    /// Relative location of the `ProofInstruction::VerifyWithdraw` instruction to the `Withdraw`
    /// instruction in the transaction
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialTransferInstruction::Transfer`
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct TransferInstructionData {
    /// The new source decryptable balance if the transfer succeeds
    pub new_source_decryptable_available_balance: DecryptableBalance,
    /// Relative location of the `ProofInstruction::VerifyTransfer` instruction to the
    /// `Transfer` instruction in the transaction
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialTransferInstruction::ApplyPendingBalance`
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct ApplyPendingBalanceData {
    /// The expected number of pending balance credits since the last successful
    /// `ApplyPendingBalance` instruction
    pub expected_pending_balance_credit_counter: PodU64,
    /// The new decryptable balance if the pending balance is applied successfully
    pub new_decryptable_available_balance: DecryptableBalance,
}

/// Create a `InitializeMint` instruction
#[cfg(not(target_os = "solana"))]
pub fn initialize_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: Option<Pubkey>,
    auto_approve_new_accounts: bool,
    auditor_elgamal_pubkey: Option<ElGamalPubkey>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![AccountMeta::new(*mint, false)];

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::InitializeMint,
        &InitializeMintData {
            authority: authority.try_into()?,
            auto_approve_new_accounts: auto_approve_new_accounts.into(),
            auditor_elgamal_pubkey: auditor_elgamal_pubkey.try_into()?,
        },
    ))
}

/// Create a `UpdateMint` instruction
#[cfg(not(target_os = "solana"))]
pub fn update_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    auto_approve_new_accounts: bool,
    auditor_elgamal_pubkey: Option<ElGamalPubkey>,
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
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::UpdateMint,
        &UpdateMintData {
            auto_approve_new_accounts: auto_approve_new_accounts.into(),
            auditor_elgamal_pubkey: auditor_elgamal_pubkey.try_into()?,
        },
    ))
}

/// Create a `ConfigureAccount` instruction
///
/// This instruction is suitable for use with a cross-program `invoke`
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_os = "solana"))]
pub fn inner_configure_account(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    decryptable_zero_balance: AeCiphertext,
    maximum_pending_balance_credit_counter: u64,
    context_state_account: Option<&Pubkey>,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_instruction_offset: i8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;

    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
    ];

    if proof_instruction_offset == 0 {
        let context_state_account = context_state_account.ok_or(ProgramError::InvalidArgument)?;
        accounts.push(AccountMeta::new_readonly(*context_state_account, false));
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
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::ConfigureAccount,
        &ConfigureAccountInstructionData {
            decryptable_zero_balance: decryptable_zero_balance.into(),
            maximum_pending_balance_credit_counter: maximum_pending_balance_credit_counter.into(),
            proof_instruction_offset,
        },
    ))
}

/// Create a `ConfigureAccount` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_os = "solana"))]
pub fn configure_account(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    decryptable_zero_balance: AeCiphertext,
    maximum_pending_balance_credit_counter: u64,
    context_state_account: Option<&Pubkey>,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data: Option<&PubkeyValidityData>,
) -> Result<Vec<Instruction>, ProgramError> {
    if let Some(proof_data) = proof_data {
        Ok(vec![
            inner_configure_account(
                token_program_id,
                token_account,
                mint,
                decryptable_zero_balance,
                maximum_pending_balance_credit_counter,
                None,
                authority,
                multisig_signers,
                1,
            )?,
            verify_pubkey_validity(None, proof_data),
        ])
    } else {
        Ok(vec![inner_configure_account(
            token_program_id,
            token_account,
            mint,
            decryptable_zero_balance,
            maximum_pending_balance_credit_counter,
            context_state_account,
            authority,
            multisig_signers,
            0,
        )?])
    }
}

/// Create an `ApproveAccount` instruction
pub fn approve_account(
    token_program_id: &Pubkey,
    account_to_approve: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*account_to_approve, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];
    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::ApproveAccount,
        &(),
    ))
}

/// Create an inner `EmptyAccount` instruction
///
/// This instruction is suitable for use with a cross-program `invoke`
pub fn inner_empty_account(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    context_state_account: Option<&Pubkey>,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_instruction_offset: i8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
    ];

    if proof_instruction_offset == 0 {
        let context_state_account = context_state_account.ok_or(ProgramError::InvalidArgument)?;
        accounts.push(AccountMeta::new_readonly(*context_state_account, false));
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
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::EmptyAccount,
        &EmptyAccountInstructionData {
            proof_instruction_offset,
        },
    ))
}

/// Create a `EmptyAccount` instruction
pub fn empty_account(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    context_state_account: Option<&Pubkey>,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data: Option<&ZeroBalanceProofData>,
) -> Result<Vec<Instruction>, ProgramError> {
    if let Some(proof_data) = proof_data {
        Ok(vec![
            inner_empty_account(
                token_program_id,
                token_account,
                context_state_account,
                authority,
                multisig_signers,
                1,
            )?,
            verify_zero_balance(None, proof_data),
        ])
    } else {
        Ok(vec![inner_empty_account(
            token_program_id,
            token_account,
            context_state_account,
            authority,
            multisig_signers,
            0,
        )?])
    }
}

/// Create a `Deposit` instruction
#[allow(clippy::too_many_arguments)]
pub fn deposit(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    amount: u64,
    decimals: u8,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::Deposit,
        &DepositInstructionData {
            amount: amount.into(),
            decimals,
        },
    ))
}

/// Create a inner `Withdraw` instruction
///
/// This instruction is suitable for use with a cross-program `invoke`
#[allow(clippy::too_many_arguments)]
pub fn inner_withdraw(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    amount: u64,
    decimals: u8,
    new_decryptable_available_balance: DecryptableBalance,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_instruction_offset: i8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::Withdraw,
        &WithdrawInstructionData {
            amount: amount.into(),
            decimals,
            new_decryptable_available_balance,
            proof_instruction_offset,
        },
    ))
}

/// Create a `Withdraw` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_os = "solana"))]
pub fn withdraw(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    amount: u64,
    decimals: u8,
    new_decryptable_available_balance: AeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data: &WithdrawData,
) -> Result<Vec<Instruction>, ProgramError> {
    Ok(vec![
        inner_withdraw(
            token_program_id,
            token_account,
            mint,
            amount,
            decimals,
            new_decryptable_available_balance.into(),
            authority,
            multisig_signers,
            1,
        )?, // calls check_program_account
        verify_withdraw(None, proof_data),
    ])
}

/// Create a inner `Transfer` instruction
///
/// This instruction is suitable for use with a cross-program `invoke`
#[allow(clippy::too_many_arguments)]
pub fn inner_transfer(
    token_program_id: &Pubkey,
    source_token_account: &Pubkey,
    destination_token_account: &Pubkey,
    mint: &Pubkey,
    new_source_decryptable_available_balance: DecryptableBalance,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_instruction_offset: i8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*source_token_account, false),
        AccountMeta::new(*destination_token_account, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::Transfer,
        &TransferInstructionData {
            new_source_decryptable_available_balance,
            proof_instruction_offset,
        },
    ))
}

/// Create a `Transfer` instruction with regular (no-fee) proof
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_os = "solana"))]
pub fn transfer(
    token_program_id: &Pubkey,
    source_token_account: &Pubkey,
    destination_token_account: &Pubkey,
    mint: &Pubkey,
    new_source_decryptable_available_balance: AeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data: &TransferData,
) -> Result<Vec<Instruction>, ProgramError> {
    Ok(vec![
        inner_transfer(
            token_program_id,
            source_token_account,
            destination_token_account,
            mint,
            new_source_decryptable_available_balance.into(),
            authority,
            multisig_signers,
            1,
        )?, // calls check_program_account
        verify_transfer(None, proof_data),
    ])
}

/// Create a `Transfer` instruction with fee proof
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_os = "solana"))]
pub fn transfer_with_fee(
    token_program_id: &Pubkey,
    source_token_account: &Pubkey,
    destination_token_account: &Pubkey,
    mint: &Pubkey,
    new_source_decryptable_available_balance: AeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data: &TransferWithFeeData,
) -> Result<Vec<Instruction>, ProgramError> {
    Ok(vec![
        inner_transfer(
            token_program_id,
            source_token_account,
            destination_token_account,
            mint,
            new_source_decryptable_available_balance.into(),
            authority,
            multisig_signers,
            1,
        )?, // calls check_program_account
        verify_transfer_with_fee(None, proof_data),
    ])
}

/// Create a inner `ApplyPendingBalance` instruction
///
/// This instruction is suitable for use with a cross-program `invoke`
pub fn inner_apply_pending_balance(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    expected_pending_balance_credit_counter: u64,
    new_decryptable_available_balance: DecryptableBalance,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::ApplyPendingBalance,
        &ApplyPendingBalanceData {
            expected_pending_balance_credit_counter: expected_pending_balance_credit_counter.into(),
            new_decryptable_available_balance,
        },
    ))
}

/// Create a `ApplyPendingBalance` instruction
#[cfg(not(target_os = "solana"))]
pub fn apply_pending_balance(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    pending_balance_instructions: u64,
    new_decryptable_available_balance: AeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    inner_apply_pending_balance(
        token_program_id,
        token_account,
        pending_balance_instructions,
        new_decryptable_available_balance.into(),
        authority,
        multisig_signers,
    ) // calls check_program_account
}

fn enable_or_disable_balance_credits(
    instruction: ConfidentialTransferInstruction,
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferExtension,
        instruction,
        &(),
    ))
}

/// Create a `EnableConfidentialCredits` instruction
pub fn enable_confidential_credits(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    enable_or_disable_balance_credits(
        ConfidentialTransferInstruction::EnableConfidentialCredits,
        token_program_id,
        token_account,
        authority,
        multisig_signers,
    )
}

/// Create a `DisableConfidentialCredits` instruction
pub fn disable_confidential_credits(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    enable_or_disable_balance_credits(
        ConfidentialTransferInstruction::DisableConfidentialCredits,
        token_program_id,
        token_account,
        authority,
        multisig_signers,
    )
}

/// Create a `EnableNonConfidentialCredits` instruction
pub fn enable_non_confidential_credits(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    enable_or_disable_balance_credits(
        ConfidentialTransferInstruction::EnableNonConfidentialCredits,
        token_program_id,
        token_account,
        authority,
        multisig_signers,
    )
}

/// Create a `DisableNonConfidentialCredits` instruction
pub fn disable_non_confidential_credits(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    enable_or_disable_balance_credits(
        ConfidentialTransferInstruction::DisableNonConfidentialCredits,
        token_program_id,
        token_account,
        authority,
        multisig_signers,
    )
}
