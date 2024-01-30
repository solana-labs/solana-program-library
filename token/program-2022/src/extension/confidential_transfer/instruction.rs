#[cfg(not(target_os = "solana"))]
use solana_zk_token_sdk::encryption::auth_encryption::AeCiphertext;
pub use solana_zk_token_sdk::{
    zk_token_proof_instruction::*, zk_token_proof_state::ProofContextState,
};
#[cfg(feature = "serde-traits")]
use {
    crate::serialization::aeciphertext_fromstr,
    serde::{Deserialize, Serialize},
};
use {
    crate::{
        check_program_account,
        extension::confidential_transfer::{ciphertext_extraction::SourceDecryptHandles, *},
        instruction::{encode_instruction, TokenInstruction},
        proof::ProofLocation,
    },
    bytemuck::Zeroable, // `Pod` comes from zk_token_proof_instruction
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvar,
    },
};

/// Confidential Transfer extension instructions
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, TryFromPrimitive, IntoPrimitive)]
#[repr(u8)]
pub enum ConfidentialTransferInstruction {
    /// Initializes confidential transfers for a mint.
    ///
    /// The `ConfidentialTransferInstruction::InitializeMint` instruction
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

    /// Updates the confidential transfer mint configuration for a mint.
    ///
    /// Use `TokenInstruction::SetAuthority` to update the confidential transfer
    /// mint authority.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token mint.
    ///   1. `[signer]` Confidential transfer mint authority.
    ///
    /// Data expected by this instruction:
    ///   `UpdateMintData`
    UpdateMint,

    /// Configures confidential transfers for a token account.
    ///
    /// The instruction fails if the confidential transfers are already
    /// configured, or if the mint was not initialized with confidential
    /// transfer support.
    ///
    /// The instruction fails if the `TokenInstruction::InitializeAccount`
    /// instruction has not yet successfully executed for the token account.
    ///
    /// Upon success, confidential and non-confidential deposits and transfers
    /// are enabled. Use the `DisableConfidentialCredits` and
    /// `DisableNonConfidentialCredits` instructions to disable.
    ///
    /// In order for this instruction to be successfully processed, it must be
    /// accompanied by the `VerifyPubkeyValidityProof` instruction of the
    /// `zk_token_proof` program in the same transaction or the address of a
    /// context state account for the proof must be provided.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writeable]` The SPL Token account.
    ///   1. `[]` The corresponding SPL Token mint.
    ///   2. `[]` Instructions sysvar if `VerifyPubkeyValidityProof` is included
    ///      in the same transaction or context state account if
    ///      `VerifyPubkeyValidityProof` is pre-verified into a context state
    ///      account.
    ///   3. `[signer]` The single source account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writeable]` The SPL Token account.
    ///   1. `[]` The corresponding SPL Token mint.
    ///   2. `[]` Instructions sysvar if `VerifyPubkeyValidityProof` is included
    ///      in the same transaction or context state account if
    ///      `VerifyPubkeyValidityProof` is pre-verified into a context state
    ///      account.
    ///   3. `[]` The multisig source account owner.
    ///   4.. `[signer]` Required M signer accounts for the SPL Token Multisig
    /// account.
    ///
    /// Data expected by this instruction:
    ///   `ConfigureAccountInstructionData`
    ConfigureAccount,

    /// Approves a token account for confidential transfers.
    ///
    /// Approval is only required when the
    /// `ConfidentialTransferMint::approve_new_accounts` field is set in the
    /// SPL Token mint.  This instruction must be executed after the account
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
    ApproveAccount,

    /// Empty the available balance in a confidential token account.
    ///
    /// A token account that is extended for confidential transfers can only be
    /// closed if the pending and available balance ciphertexts are emptied.
    /// The pending balance can be emptied
    /// via the `ConfidentialTransferInstruction::ApplyPendingBalance`
    /// instruction. Use the `ConfidentialTransferInstruction::EmptyAccount`
    /// instruction to empty the available balance ciphertext.
    ///
    /// Note that a newly configured account is always empty, so this
    /// instruction is not required prior to account closing if no
    /// instructions beyond
    /// `ConfidentialTransferInstruction::ConfigureAccount` have affected the
    /// token account.
    ///
    /// In order for this instruction to be successfully processed, it must be
    /// accompanied by the `VerifyZeroBalanceProof` instruction of the
    /// `zk_token_proof` program in the same transaction or the address of a
    /// context state account for the proof must be provided.
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` Instructions sysvar if `VerifyZeroBalanceProof` is included in
    ///      the same transaction or context state account if
    ///      `VerifyZeroBalanceProof` is pre-verified into a context state
    ///      account.
    ///   2. `[signer]` The single account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` Instructions sysvar if `VerifyZeroBalanceProof` is included in
    ///      the same transaction or context state account if
    ///      `VerifyZeroBalanceProof` is pre-verified into a context state
    ///      account.
    ///   2. `[]` The multisig account owner.
    ///   3.. `[signer]` Required M signer accounts for the SPL Token Multisig
    /// account.
    ///
    /// Data expected by this instruction:
    ///   `EmptyAccountInstructionData`
    EmptyAccount,

    /// Deposit SPL Tokens into the pending balance of a confidential token
    /// account.
    ///
    /// The account owner can then invoke the `ApplyPendingBalance` instruction
    /// to roll the deposit into their available balance at a time of their
    /// choosing.
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
    ///   3.. `[signer]` Required M signer accounts for the SPL Token Multisig
    /// account.
    ///
    /// Data expected by this instruction:
    ///   `DepositInstructionData`
    Deposit,

    /// Withdraw SPL Tokens from the available balance of a confidential token
    /// account.
    ///
    /// Fails if the source or destination accounts are frozen.
    /// Fails if the associated mint is extended as `NonTransferable`.
    ///
    /// In order for this instruction to be successfully processed, it must be
    /// accompanied by the `VerifyWithdraw` instruction of the
    /// `zk_token_proof` program in the same transaction or the address of a
    /// context state account for the proof must be provided.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` The token mint.
    ///   2. `[]` Instructions sysvar if `VerifyWithdraw` is included in the
    ///      same transaction or context state account if `VerifyWithdraw` is
    ///      pre-verified into a context state account.
    ///   3. `[signer]` The single source account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` The token mint.
    ///   2. `[]` Instructions sysvar if `VerifyWithdraw` is included in the
    ///      same transaction or context state account if `VerifyWithdraw` is
    ///      pre-verified into a context state account.
    ///   3. `[]` The multisig  source account owner.
    ///   4.. `[signer]` Required M signer accounts for the SPL Token Multisig
    /// account.
    ///
    /// Data expected by this instruction:
    ///   `WithdrawInstructionData`
    Withdraw,

    /// Transfer tokens confidentially.
    ///
    /// In order for this instruction to be successfully processed, it must be
    /// accompanied by either the `VerifyTransfer` or
    /// `VerifyTransferWithFee` instruction of the `zk_token_proof`
    /// program in the same transaction or the address of a context state
    /// account for the proof must be provided.
    ///
    /// Fails if the associated mint is extended as `NonTransferable`.
    ///
    ///   * Single owner/delegate
    ///   1. `[writable]` The source SPL Token account.
    ///   2. `[]` The token mint.
    ///   3. `[writable]` The destination SPL Token account.
    ///   4. `[]` Instructions sysvar if `VerifyTransfer` or
    ///      `VerifyTransferWithFee` is included in the same transaction or
    ///      context state account if these proofs are pre-verified into a
    ///      context state account.
    ///   5. `[signer]` The single source account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   1. `[writable]` The source SPL Token account.
    ///   2. `[]` The token mint.
    ///   3. `[writable]` The destination SPL Token account.
    ///   4. `[]` Instructions sysvar if `VerifyTransfer` or
    ///      `VerifyTransferWithFee` is included in the same transaction or
    ///      context state account if these proofs are pre-verified into a
    ///      context state account.
    ///   5. `[]` The multisig  source account owner.
    ///   6.. `[signer]` Required M signer accounts for the SPL Token Multisig
    /// account.
    ///
    /// Data expected by this instruction:
    ///   `TransferInstructionData`
    Transfer,

    /// Applies the pending balance to the available balance, based on the
    /// history of `Deposit` and/or `Transfer` instructions.
    ///
    /// After submitting `ApplyPendingBalance`, the client should compare
    /// `ConfidentialTransferAccount::expected_pending_balance_credit_counter`
    /// with
    /// `ConfidentialTransferAccount::actual_applied_pending_balance_instructions`.  If they are
    /// equal then the
    /// `ConfidentialTransferAccount::decryptable_available_balance` is
    /// consistent with `ConfidentialTransferAccount::available_balance`. If
    /// they differ then there is more pending balance to be applied.
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
    ///   2.. `[signer]` Required M signer accounts for the SPL Token Multisig
    /// account.
    ///
    /// Data expected by this instruction:
    ///   `ApplyPendingBalanceData`
    ApplyPendingBalance,

    /// Configure a confidential extension account to accept incoming
    /// confidential transfers.
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
    ///   2.. `[signer]` Required M signer accounts for the SPL Token Multisig
    /// account.
    ///
    /// Data expected by this instruction:
    ///   None
    EnableConfidentialCredits,

    /// Configure a confidential extension account to reject any incoming
    /// confidential transfers.
    ///
    /// If the `allow_non_confidential_credits` field is `true`, then the base
    /// account can still receive non-confidential transfers.
    ///
    /// This instruction can be used to disable confidential payments after a
    /// token account has already been extended for confidential transfers.
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
    ///   2.. `[signer]` Required M signer accounts for the SPL Token Multisig
    /// account.
    ///
    /// Data expected by this instruction:
    ///   None
    DisableConfidentialCredits,

    /// Configure an account with the confidential extension to accept incoming
    /// non-confidential transfers.
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
    ///   2.. `[signer]` Required M signer accounts for the SPL Token Multisig
    /// account.
    ///
    /// Data expected by this instruction:
    ///   None
    EnableNonConfidentialCredits,

    /// Configure an account with the confidential extension to reject any
    /// incoming non-confidential transfers.
    ///
    /// This instruction can be used to configure a confidential extension
    /// account to exclusively receive confidential payments.
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
    ///   2.. `[signer]` Required M signer accounts for the SPL Token Multisig
    /// account.
    ///
    /// Data expected by this instruction:
    ///   None
    DisableNonConfidentialCredits,

    /// Transfer tokens confidentially with zero-knowledge proofs that are split
    /// into smaller components.
    ///
    /// In order for this instruction to be successfully processed, it must be
    /// accompanied by suitable zero-knowledge proof context accounts listed
    /// below.
    ///
    /// The same restrictions for the `Transfer` applies to
    /// `TransferWithSplitProofs`. Namely, the instruction fails if the
    /// associated mint is extended as `NonTransferable`.
    ///
    ///   * Transfer without fee
    ///   1. `[writable]` The source SPL Token account.
    ///   2. `[]` The token mint.
    ///   3. `[writable]` The destination SPL Token account.
    ///   4. `[]` Context state account for
    ///      `VerifyCiphertextCommitmentEqualityProof`.
    ///   5. `[]` Context state account for
    ///      `VerifyBatchedGroupedCiphertext2HandlesValidityProof`.
    ///   6. `[]` Context state account for `VerifyBatchedRangeProofU128`.
    ///   If `close_split_context_state_on_execution` is set, all context state
    ///     accounts must be `writable` and the following sequence
    ///     of accounts that are marked with asterisk are needed:
    ///   7*. `[]` The destination account for lamports from the context state
    ///      accounts.
    ///   8*. `[signer]` The context state account owner.
    ///   9*. `[]` The zk token proof program.
    ///   10. `[signer]` The source account owner.
    ///
    ///   * Transfer with fee
    ///   1. `[writable]` The source SPL Token account.
    ///   2. `[]` The token mint.
    ///   3. `[writable]` The destination SPL Token account.
    ///   4. `[]` Context state account for
    ///      `VerifyCiphertextCommitmentEqualityProof`.
    ///   5. `[]` Context state account for
    ///      `VerifyBatchedGroupedCiphertext2HandlesValidityProof`.
    ///   6. `[]` Context state account for `VerifyFeeSigmaProof`.
    ///   7. `[]` Context state account for
    ///      `VerifyBatchedGroupedCiphertext2HandlesValidityProof`.
    ///   8. `[]` Context state account for `VerifyBatchedRangeProofU256`.
    ///   If `close_split_context_state_on_execution` is set, all context state
    ///     accounts must be  `writable` and the following sequence
    ///     of accounts that are marked with asterisk are needed:
    ///   9*. `[]` The destination account for lamports from the context state
    ///       accounts.
    ///   10*. `[signer]` The context state account owner.
    ///   11*. `[]` The zk token proof program.
    ///   12. `[signer]` The source account owner.
    ///
    /// Data expected by this instruction:
    ///   `TransferWithSplitProofsInstructionData`
    TransferWithSplitProofs,
}

/// Data expected by `ConfidentialTransferInstruction::InitializeMint`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct InitializeMintData {
    /// Authority to modify the `ConfidentialTransferMint` configuration and to
    /// approve new accounts.
    pub authority: OptionalNonZeroPubkey,
    /// Determines if newly configured accounts must be approved by the
    /// `authority` before they may be used by the user.
    pub auto_approve_new_accounts: PodBool,
    /// New authority to decode any transfer amount in a confidential transfer.
    pub auditor_elgamal_pubkey: OptionalNonZeroElGamalPubkey,
}

/// Data expected by `ConfidentialTransferInstruction::UpdateMint`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct UpdateMintData {
    /// Determines if newly configured accounts must be approved by the
    /// `authority` before they may be used by the user.
    pub auto_approve_new_accounts: PodBool,
    /// New authority to decode any transfer amount in a confidential transfer.
    pub auditor_elgamal_pubkey: OptionalNonZeroElGamalPubkey,
}

/// Data expected by `ConfidentialTransferInstruction::ConfigureAccount`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct ConfigureAccountInstructionData {
    /// The decryptable balance (always 0) once the configure account succeeds
    #[cfg_attr(feature = "serde-traits", serde(with = "aeciphertext_fromstr"))]
    pub decryptable_zero_balance: DecryptableBalance,
    /// The maximum number of despots and transfers that an account can receiver
    /// before the `ApplyPendingBalance` is executed
    pub maximum_pending_balance_credit_counter: PodU64,
    /// Relative location of the `ProofInstruction::ZeroBalanceProof`
    /// instruction to the `ConfigureAccount` instruction in the
    /// transaction. If the offset is `0`, then use a context state account
    /// for the proof.
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialTransferInstruction::EmptyAccount`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct EmptyAccountInstructionData {
    /// Relative location of the `ProofInstruction::VerifyCloseAccount`
    /// instruction to the `EmptyAccount` instruction in the transaction. If
    /// the offset is `0`, then use a context state account for the proof.
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialTransferInstruction::Deposit`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct DepositInstructionData {
    /// The amount of tokens to deposit
    pub amount: PodU64,
    /// Expected number of base 10 digits to the right of the decimal place
    pub decimals: u8,
}

/// Data expected by `ConfidentialTransferInstruction::Withdraw`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct WithdrawInstructionData {
    /// The amount of tokens to withdraw
    pub amount: PodU64,
    /// Expected number of base 10 digits to the right of the decimal place
    pub decimals: u8,
    /// The new decryptable balance if the withdrawal succeeds
    #[cfg_attr(feature = "serde-traits", serde(with = "aeciphertext_fromstr"))]
    pub new_decryptable_available_balance: DecryptableBalance,
    /// Relative location of the `ProofInstruction::VerifyWithdraw` instruction
    /// to the `Withdraw` instruction in the transaction. If the offset is
    /// `0`, then use a context state account for the proof.
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialTransferInstruction::Transfer`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct TransferInstructionData {
    /// The new source decryptable balance if the transfer succeeds
    #[cfg_attr(feature = "serde-traits", serde(with = "aeciphertext_fromstr"))]
    pub new_source_decryptable_available_balance: DecryptableBalance,
    /// Relative location of the `ProofInstruction::VerifyTransfer` instruction
    /// to the `Transfer` instruction in the transaction. If the offset is
    /// `0`, then use a context state account for the proof.
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialTransferInstruction::ApplyPendingBalance`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[cfg_attr(feature = "serde-traits", serde(rename_all = "camelCase"))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct ApplyPendingBalanceData {
    /// The expected number of pending balance credits since the last successful
    /// `ApplyPendingBalance` instruction
    pub expected_pending_balance_credit_counter: PodU64,
    /// The new decryptable balance if the pending balance is applied
    /// successfully
    #[cfg_attr(feature = "serde-traits", serde(with = "aeciphertext_fromstr"))]
    pub new_decryptable_available_balance: DecryptableBalance,
}

/// Data expected by `ConfidentialTransferInstruction::TransferWithSplitProofs`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct TransferWithSplitProofsInstructionData {
    /// The new source decryptable balance if the transfer succeeds
    #[cfg_attr(feature = "serde-traits", serde(with = "aeciphertext_fromstr"))]
    pub new_source_decryptable_available_balance: DecryptableBalance,
    /// If true, execute no op when an associated context state account is not
    /// initialized. Otherwise, fail on an uninitialized context state
    /// account.
    pub no_op_on_uninitialized_split_context_state: PodBool,
    /// Close associated context states after a complete execution of the
    /// transfer instruction.
    pub close_split_context_state_on_execution: PodBool,
    /// The ElGamal decryption handle pertaining to the low and high bits of the
    /// transfer amount. This field is used when the transfer proofs are
    /// split and verified as smaller components.
    ///
    /// NOTE: This field is to be removed in the next Solana upgrade.
    pub source_decrypt_handles: SourceDecryptHandles,
}

/// Type for split transfer (without fee) instruction proof context state
/// account addresses intended to be used as parameters to functions.
#[derive(Clone, Copy)]
pub struct TransferSplitContextStateAccounts<'a> {
    /// The context state account address for an equality proof needed for a
    /// transfer.
    pub equality_proof: &'a Pubkey,
    /// The context state account address for a ciphertext validity proof needed
    /// for a transfer.
    pub ciphertext_validity_proof: &'a Pubkey,
    /// The context state account address for a range proof needed for a
    /// transfer.
    pub range_proof: &'a Pubkey,
    /// The context state accounts authority
    pub authority: &'a Pubkey,
    /// No op if an associated split proof context state account is not
    /// initialized.
    pub no_op_on_uninitialized_split_context_state: bool,
    /// Accounts needed if `close_split_context_state_on_execution` flag is
    /// enabled.
    pub close_split_context_state_accounts: Option<CloseSplitContextStateAccounts<'a>>,
}

/// Type for split transfer (with fee) instruction proof context state account
/// addresses intended to be used as parameters to functions.
#[derive(Clone, Copy)]
pub struct TransferWithFeeSplitContextStateAccounts<'a> {
    /// The context state account address for an equality proof needed for a
    /// transfer with fee.
    pub equality_proof: &'a Pubkey,
    /// The context state account address for a transfer amount ciphertext
    /// validity proof needed for a transfer with fee.
    pub transfer_amount_ciphertext_validity_proof: &'a Pubkey,
    /// The context state account address for a fee sigma proof needed for a
    /// transfer with fee.
    pub fee_sigma_proof: &'a Pubkey,
    /// The context state account address for a fee ciphertext validity proof
    /// needed for a transfer with fee.
    pub fee_ciphertext_validity_proof: &'a Pubkey,
    /// The context state account address for a range proof needed for a
    /// transfer with fee.
    pub range_proof: &'a Pubkey,
    /// The context state accounts authority
    pub authority: &'a Pubkey,
    /// No op if an associated split proof context state account is not
    /// initialized.
    pub no_op_on_uninitialized_split_context_state: bool,
    /// Accounts needed if `close_split_context_state_on_execution` flag is
    /// enabled.
    pub close_split_context_state_accounts: Option<CloseSplitContextStateAccounts<'a>>,
}

/// Accounts needed if `close_split_context_state_on_execution` flag is enabled
/// on a transfer.
#[derive(Clone, Copy)]
pub struct CloseSplitContextStateAccounts<'a> {
    /// The lamport destination account.
    pub lamport_destination: &'a Pubkey,
    /// The ZK Token proof program.
    pub zk_token_proof_program: &'a Pubkey,
}

/// Create a `InitializeMint` instruction
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
pub fn inner_configure_account(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    decryptable_zero_balance: AeCiphertext,
    maximum_pending_balance_credit_counter: u64,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data_location: ProofLocation<PubkeyValidityData>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;

    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*mint, false),
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
pub fn configure_account(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    decryptable_zero_balance: AeCiphertext,
    maximum_pending_balance_credit_counter: u64,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data_location: ProofLocation<PubkeyValidityData>,
) -> Result<Vec<Instruction>, ProgramError> {
    let mut instructions = vec![inner_configure_account(
        token_program_id,
        token_account,
        mint,
        decryptable_zero_balance,
        maximum_pending_balance_credit_counter,
        authority,
        multisig_signers,
        proof_data_location,
    )?];

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        proof_data_location
    {
        // This constructor appends the proof instruction right after the
        // `ConfigureAccount` instruction. This means that the proof instruction
        // offset must be always be 1. To use an arbitrary proof instruction
        // offset, use the `inner_configure_account` constructor.
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != 1 {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        instructions.push(verify_pubkey_validity(None, proof_data));
    };

    Ok(instructions)
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
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data_location: ProofLocation<ZeroBalanceProofData>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![AccountMeta::new(*token_account, false)];

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
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data_location: ProofLocation<ZeroBalanceProofData>,
) -> Result<Vec<Instruction>, ProgramError> {
    let mut instructions = vec![inner_empty_account(
        token_program_id,
        token_account,
        authority,
        multisig_signers,
        proof_data_location,
    )?];

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        proof_data_location
    {
        // This constructor appends the proof instruction right after the `EmptyAccount`
        // instruction. This means that the proof instruction offset must be always be
        // 1. To use an arbitrary proof instruction offset, use the
        // `inner_empty_account` constructor.
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != 1 {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        instructions.push(verify_zero_balance(None, proof_data));
    };

    Ok(instructions)
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
    proof_data_location: ProofLocation<WithdrawData>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*mint, false),
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
pub fn withdraw(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    amount: u64,
    decimals: u8,
    new_decryptable_available_balance: AeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data_location: ProofLocation<WithdrawData>,
) -> Result<Vec<Instruction>, ProgramError> {
    let mut instructions = vec![inner_withdraw(
        token_program_id,
        token_account,
        mint,
        amount,
        decimals,
        new_decryptable_available_balance.into(),
        authority,
        multisig_signers,
        proof_data_location,
    )?];

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        proof_data_location
    {
        // This constructor appends the proof instruction right after the `Withdraw`
        // instruction. This means that the proof instruction offset must be
        // always be 1. To use an arbitrary proof instruction offset, use the
        // `inner_withdraw` constructor.
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != 1 {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        instructions.push(verify_withdraw(None, proof_data));
    };

    Ok(instructions)
}

/// Create a inner `Transfer` instruction
///
/// This instruction is suitable for use with a cross-program `invoke`
#[allow(clippy::too_many_arguments)]
pub fn inner_transfer(
    token_program_id: &Pubkey,
    source_token_account: &Pubkey,
    mint: &Pubkey,
    destination_token_account: &Pubkey,
    new_source_decryptable_available_balance: DecryptableBalance,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data_location: ProofLocation<TransferData>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*source_token_account, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*destination_token_account, false),
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
        ConfidentialTransferInstruction::Transfer,
        &TransferInstructionData {
            new_source_decryptable_available_balance,
            proof_instruction_offset,
        },
    ))
}

/// Create a `Transfer` instruction with regular (no-fee) proof
#[allow(clippy::too_many_arguments)]
pub fn transfer(
    token_program_id: &Pubkey,
    source_token_account: &Pubkey,
    mint: &Pubkey,
    destination_token_account: &Pubkey,
    new_source_decryptable_available_balance: AeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data_location: ProofLocation<TransferData>,
) -> Result<Vec<Instruction>, ProgramError> {
    let mut instructions = vec![inner_transfer(
        token_program_id,
        source_token_account,
        mint,
        destination_token_account,
        new_source_decryptable_available_balance.into(),
        authority,
        multisig_signers,
        proof_data_location,
    )?];

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        proof_data_location
    {
        // This constructor appends the proof instruction right after the `Transfer`
        // instruction. This means that the proof instruction offset must be
        // always be 1. To use an arbitrary proof instruction offset, use the
        // `inner_transfer` constructor.
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != 1 {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        instructions.push(verify_transfer(None, proof_data));
    };

    Ok(instructions)
}

/// Create a inner `Transfer` instruction with fee
///
/// This instruction is suitable for use with a cross-program `invoke`
#[allow(clippy::too_many_arguments)]
pub fn inner_transfer_with_fee(
    token_program_id: &Pubkey,
    source_token_account: &Pubkey,
    mint: &Pubkey,
    destination_token_account: &Pubkey,
    new_source_decryptable_available_balance: DecryptableBalance,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data_location: ProofLocation<TransferWithFeeData>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*source_token_account, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*destination_token_account, false),
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
        ConfidentialTransferInstruction::Transfer,
        &TransferInstructionData {
            new_source_decryptable_available_balance,
            proof_instruction_offset,
        },
    ))
}

/// Create a `Transfer` instruction with fee proof
#[allow(clippy::too_many_arguments)]
pub fn transfer_with_fee(
    token_program_id: &Pubkey,
    source_token_account: &Pubkey,
    mint: &Pubkey,
    destination_token_account: &Pubkey,
    new_source_decryptable_available_balance: AeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data_location: ProofLocation<TransferWithFeeData>,
) -> Result<Vec<Instruction>, ProgramError> {
    let mut instructions = vec![inner_transfer_with_fee(
        token_program_id,
        source_token_account,
        destination_token_account,
        mint,
        new_source_decryptable_available_balance.into(),
        authority,
        multisig_signers,
        proof_data_location,
    )?];

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        proof_data_location
    {
        // This constructor appends the proof instruction right after the
        // `TransferWithFee` instruction. This means that the proof instruction
        // offset must be always be 1. To use an arbitrary proof instruction
        // offset, use the `inner_transfer_with_fee` constructor.
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != 1 {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        instructions.push(verify_transfer_with_fee(None, proof_data));
    };

    Ok(instructions)
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

/// Create a `TransferWithSplitProof` instruction without fee
#[allow(clippy::too_many_arguments)]
pub fn transfer_with_split_proofs(
    token_program_id: &Pubkey,
    source_token_account: &Pubkey,
    mint: &Pubkey,
    destination_token_account: &Pubkey,
    new_source_decryptable_available_balance: DecryptableBalance,
    source_account_authority: &Pubkey,
    context_accounts: TransferSplitContextStateAccounts,
    source_decrypt_handles: &SourceDecryptHandles,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*source_token_account, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*destination_token_account, false),
    ];

    let close_split_context_state_on_execution =
        if let Some(close_split_context_state_on_execution_accounts) =
            context_accounts.close_split_context_state_accounts
        {
            // If `close_split_context_state_accounts` is set, then all context state
            // accounts must be `writable`.
            accounts.push(AccountMeta::new(*context_accounts.equality_proof, false));
            accounts.push(AccountMeta::new(
                *context_accounts.ciphertext_validity_proof,
                false,
            ));
            accounts.push(AccountMeta::new(*context_accounts.range_proof, false));
            accounts.push(AccountMeta::new(
                *close_split_context_state_on_execution_accounts.lamport_destination,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(*context_accounts.authority, true));
            accounts.push(AccountMeta::new_readonly(
                *close_split_context_state_on_execution_accounts.zk_token_proof_program,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(*source_account_authority, true));
            true
        } else {
            // If `close_split_context_state_accounts` is not set, then context state
            // accounts can be read-only.
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
            accounts.push(AccountMeta::new_readonly(*source_account_authority, true));

            false
        };

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::TransferWithSplitProofs,
        &TransferWithSplitProofsInstructionData {
            new_source_decryptable_available_balance,
            no_op_on_uninitialized_split_context_state: context_accounts
                .no_op_on_uninitialized_split_context_state
                .into(),
            close_split_context_state_on_execution: close_split_context_state_on_execution.into(),
            source_decrypt_handles: *source_decrypt_handles,
        },
    ))
}

/// Create a `TransferWithSplitProof` instruction with fee
#[allow(clippy::too_many_arguments)]
pub fn transfer_with_fee_and_split_proofs(
    token_program_id: &Pubkey,
    source_token_account: &Pubkey,
    mint: &Pubkey,
    destination_token_account: &Pubkey,
    new_source_decryptable_available_balance: DecryptableBalance,
    source_account_authority: &Pubkey,
    context_accounts: TransferWithFeeSplitContextStateAccounts,
    source_decrypt_handles: &SourceDecryptHandles,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*source_token_account, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*destination_token_account, false),
    ];

    let close_split_context_state_on_execution =
        if let Some(close_split_context_state_on_execution_accounts) =
            context_accounts.close_split_context_state_accounts
        {
            // If `close_split_context_state_accounts` is set, then all context state
            // accounts must be `writable`.
            accounts.push(AccountMeta::new(*context_accounts.equality_proof, false));
            accounts.push(AccountMeta::new(
                *context_accounts.transfer_amount_ciphertext_validity_proof,
                false,
            ));
            accounts.push(AccountMeta::new(*context_accounts.fee_sigma_proof, false));
            accounts.push(AccountMeta::new(
                *context_accounts.fee_ciphertext_validity_proof,
                false,
            ));
            accounts.push(AccountMeta::new(*context_accounts.range_proof, false));
            accounts.push(AccountMeta::new(
                *close_split_context_state_on_execution_accounts.lamport_destination,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(*context_accounts.authority, true));
            accounts.push(AccountMeta::new_readonly(
                *close_split_context_state_on_execution_accounts.zk_token_proof_program,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(*source_account_authority, true));
            true
        } else {
            // If `close_split_context_state_accounts` is not set, then context state
            // accounts can be read-only.
            accounts.push(AccountMeta::new_readonly(
                *context_accounts.equality_proof,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(
                *context_accounts.transfer_amount_ciphertext_validity_proof,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(
                *context_accounts.fee_sigma_proof,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(
                *context_accounts.fee_ciphertext_validity_proof,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(
                *context_accounts.range_proof,
                false,
            ));
            accounts.push(AccountMeta::new_readonly(*source_account_authority, true));
            false
        };

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::TransferWithSplitProofs,
        &TransferWithSplitProofsInstructionData {
            new_source_decryptable_available_balance,
            no_op_on_uninitialized_split_context_state: context_accounts
                .no_op_on_uninitialized_split_context_state
                .into(),
            close_split_context_state_on_execution: close_split_context_state_on_execution.into(),
            source_decrypt_handles: *source_decrypt_handles,
        },
    ))
}
