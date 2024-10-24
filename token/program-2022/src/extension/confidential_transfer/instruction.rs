pub use solana_zk_sdk::zk_elgamal_proof_program::{
    instruction::ProofInstruction, proof_data::*, state::ProofContextState,
};
#[cfg(feature = "serde-traits")]
use {
    crate::serialization::aeciphertext_fromstr,
    serde::{Deserialize, Serialize},
};
use {
    crate::{
        check_program_account,
        extension::confidential_transfer::*,
        instruction::{encode_instruction, TokenInstruction},
    },
    bytemuck::Zeroable,
    num_enum::{IntoPrimitive, TryFromPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        system_program, sysvar,
    },
    spl_token_confidential_transfer_proof_extraction::instruction::{ProofData, ProofLocation},
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
    /// accompanied by the `VerifyPubkeyValidity` instruction of the
    /// `zk_elgamal_proof` program in the same transaction or the address of a
    /// context state account for the proof must be provided.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writeable]` The SPL Token account.
    ///   1. `[]` The corresponding SPL Token mint.
    ///   2. `[]` Instructions sysvar if `VerifyPubkeyValidity` is included in
    ///      the same transaction or context state account if
    ///      `VerifyPubkeyValidity` is pre-verified into a context state
    ///      account.
    ///   3. `[]` (Optional) Record account if the accompanying proof is to be
    ///      read from a record account.
    ///   4. `[signer]` The single source account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writeable]` The SPL Token account.
    ///   1. `[]` The corresponding SPL Token mint.
    ///   2. `[]` Instructions sysvar if `VerifyPubkeyValidity` is included in
    ///      the same transaction or context state account if
    ///      `VerifyPubkeyValidity` is pre-verified into a context state
    ///      account.
    ///   3. `[]` (Optional) Record account if the accompanying proof is to be
    ///      read from a record account.
    ///   4. `[]` The multisig source account owner.
    ///   5.. `[signer]` Required M signer accounts for the SPL Token Multisig
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
    ///   2. `[signer]` Confidential transfer mint authority.
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
    /// accompanied by the `VerifyZeroCiphertext` instruction of the
    /// `zk_elgamal_proof` program in the same transaction or the address of a
    /// context state account for the proof must be provided.
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` Instructions sysvar if `VerifyZeroCiphertext` is included in
    ///      the same transaction or context state account if
    ///      `VerifyZeroCiphertext` is pre-verified into a context state
    ///      account.
    ///   2. `[]` (Optional) Record account if the accompanying proof is to be
    ///      read from a record account.
    ///   3. `[signer]` The single account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` Instructions sysvar if `VerifyZeroCiphertext` is included in
    ///      the same transaction or context state account if
    ///      `VerifyZeroCiphertext` is pre-verified into a context state
    ///      account.
    ///   2. `[]` (Optional) Record account if the accompanying proof is to be
    ///      read from a record account.
    ///   3. `[]` The multisig account owner.
    ///   4.. `[signer]` Required M signer accounts for the SPL Token Multisig
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
    /// In order for this instruction to be successfully processed, it must be
    /// accompanied by the following list of `zk_elgamal_proof` program
    /// instructions:
    /// - `VerifyCiphertextCommitmentEquality`
    /// - `VerifyBatchedRangeProofU64`
    /// These instructions can be accompanied in the same transaction or can be
    /// pre-verified into a context state account, in which case, only their
    /// context state account address need to be provided.
    ///
    /// Fails if the source or destination accounts are frozen.
    /// Fails if the associated mint is extended as `NonTransferable`.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` The token mint.
    ///   2. `[]` (Optional) Instructions sysvar if at least one of the
    ///      `zk_elgamal_proof` instructions are included in the same
    ///      transaction.
    ///   3. `[]` (Optional) Equality proof record account or context state
    ///      account.
    ///   4. `[]` (Optional) Range proof record account or context state
    ///      account.
    ///   5. `[signer]` The single source account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` The token mint.
    ///   2. `[]` (Optional) Instructions sysvar if at least one of the
    ///      `zk_elgamal_proof` instructions are included in the same
    ///      transaction.
    ///   3. `[]` (Optional) Equality proof record account or context state
    ///      account.
    ///   4. `[]` (Optional) Range proof record account or context state
    ///      account.
    ///   5. `[]` The multisig  source account owner.
    ///   6.. `[signer]` Required M signer accounts for the SPL Token Multisig
    /// account.
    ///
    /// Data expected by this instruction:
    ///   `WithdrawInstructionData`
    Withdraw,

    /// Transfer tokens confidentially.
    ///
    /// In order for this instruction to be successfully processed, it must be
    /// accompanied by the following list of `zk_elgamal_proof` program
    /// instructions:
    /// - `VerifyCiphertextCommitmentEquality`
    /// - `VerifyBatchedGroupedCiphertext3HandlesValidity`
    /// - `VerifyBatchedRangeProofU128`
    /// These instructions can be accompanied in the same transaction or can be
    /// pre-verified into a context state account, in which case, only their
    /// context state account addresses need to be provided.
    ///
    /// Fails if the associated mint is extended as `NonTransferable`.
    ///
    ///   * Single owner/delegate
    ///   1. `[writable]` The source SPL Token account.
    ///   2. `[]` The token mint.
    ///   3. `[writable]` The destination SPL Token account.
    ///   4. `[]` (Optional) Instructions sysvar if at least one of the
    ///      `zk_elgamal_proof` instructions are included in the same
    ///      transaction.
    ///   5. `[]` (Optional) Equality proof record account or context state
    ///      account.
    ///   6. `[]` (Optional) Ciphertext validity proof record account or context
    ///      state account.
    ///   7. `[]` (Optional) Range proof record account or context state
    ///      account.
    ///   8. `[signer]` The single source account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   1. `[writable]` The source SPL Token account.
    ///   2. `[]` The token mint.
    ///   3. `[writable]` The destination SPL Token account.
    ///   4. `[]` (Optional) Instructions sysvar if at least one of the
    ///      `zk_elgamal_proof` instructions are included in the same
    ///      transaction.
    ///   5. `[]` (Optional) Equality proof record account or context state
    ///      account.
    ///   6. `[]` (Optional) Ciphertext validity proof record account or context
    ///      state account.
    ///   7. `[]` (Optional) Range proof record account or context state
    ///      account.
    ///   8. `[]` The multisig  source account owner.
    ///   9.. `[signer]` Required M signer accounts for the SPL Token Multisig
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

    /// Transfer tokens confidentially with fee.
    ///
    /// In order for this instruction to be successfully processed, it must be
    /// accompanied by the following list of `zk_elgamal_proof` program
    /// instructions:
    /// - `VerifyCiphertextCommitmentEquality`
    /// - `VerifyBatchedGroupedCiphertext3HandlesValidity` (transfer amount
    ///   ciphertext)
    /// - `VerifyPercentageWithFee`
    /// - `VerifyBatchedGroupedCiphertext2HandlesValidity` (fee ciphertext)
    /// - `VerifyBatchedRangeProofU256`
    /// These instructions can be accompanied in the same transaction or can be
    /// pre-verified into a context state account, in which case, only their
    /// context state account addresses need to be provided.
    ///
    /// The same restrictions for the `Transfer` applies to
    /// `TransferWithFee`. Namely, the instruction fails if the
    /// associated mint is extended as `NonTransferable`.
    ///
    ///   * Transfer without fee
    ///   1. `[writable]` The source SPL Token account.
    ///   2. `[]` The token mint.
    ///   3. `[writable]` The destination SPL Token account.
    ///   4. `[]` (Optional) Instructions sysvar if at least one of the
    ///      `zk_elgamal_proof` instructions are included in the same
    ///      transaction.
    ///   5. `[]` (Optional) Equality proof record account or context state
    ///      account.
    ///   6. `[]` (Optional) Transfer amount ciphertext validity proof record
    ///      account or context state account.
    ///   7. `[]` (Optional) Fee sigma proof record account or context state
    ///      account.
    ///   8. `[]` (Optional) Fee ciphertext validity proof record account or
    ///      context state account.
    ///   9. `[]` (Optional) Range proof record account or context state
    ///      account.
    ///   10. `[signer]` The source account owner.
    ///
    ///   * Transfer with fee
    ///   1. `[writable]` The source SPL Token account.
    ///   2. `[]` The token mint.
    ///   3. `[writable]` The destination SPL Token account.
    ///   4. `[]` (Optional) Instructions sysvar if at least one of the
    ///      `zk_elgamal_proof` instructions are included in the same
    ///      transaction.
    ///   5. `[]` (Optional) Equality proof record account or context state
    ///      account.
    ///   6. `[]` (Optional) Transfer amount ciphertext validity proof record
    ///      account or context state account.
    ///   7. `[]` (Optional) Fee sigma proof record account or context state
    ///      account.
    ///   8. `[]` (Optional) Fee ciphertext validity proof record account or
    ///      context state account.
    ///   9. `[]` (Optional) Range proof record account or context state
    ///      account.
    ///   10. `[]` The multisig  source account owner.
    ///   11.. `[signer]` Required M signer accounts for the SPL Token Multisig
    ///
    /// Data expected by this instruction:
    ///   `TransferWithFeeInstructionData`
    TransferWithFee,

    /// Configures confidential transfers for a token account.
    ///
    /// This instruction is identical to the `ConfigureAccount` account except
    /// that a valid `ElGamalRegistry` account is expected in place of the
    /// `VerifyPubkeyValidity` proof.
    ///
    /// An `ElGamalRegistry` account is valid if it shares the same owner with
    /// the token account. If a valid `ElGamalRegistry` account is provided,
    /// then the program skips the verification of the ElGamal pubkey
    /// validity proof as well as the token owner signature.
    ///
    /// If the token account is not large enough to include the new
    /// cconfidential transfer extension, then optionally reallocate the
    /// account to increase the data size. To reallocate, a payer account to
    /// fund the reallocation and the system account should be included in the
    /// instruction.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` The corresponding SPL Token mint.
    ///   2. `[]` The ElGamal registry account.
    ///   3. `[signer, writable]` (Optional) The payer account to fund
    ///      reallocation
    ///   4. `[]` (Optional) System program for reallocation funding
    ///
    /// Data expected by this instruction:
    ///   None
    ConfigureAccountWithRegistry,
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
    /// Relative location of the `ProofInstruction::ZeroCiphertextProof`
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
    /// Relative location of the
    /// `ProofInstruction::VerifyCiphertextCommitmentEquality` instruction
    /// to the `Withdraw` instruction in the transaction. If the offset is
    /// `0`, then use a context state account for the proof.
    pub equality_proof_instruction_offset: i8,
    /// Relative location of the `ProofInstruction::BatchedRangeProofU64`
    /// instruction to the `Withdraw` instruction in the transaction. If the
    /// offset is `0`, then use a context state account for the proof.
    pub range_proof_instruction_offset: i8,
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
    /// Relative location of the
    /// `ProofInstruction::VerifyCiphertextCommitmentEquality` instruction
    /// to the `Transfer` instruction in the transaction. If the offset is
    /// `0`, then use a context state account for the proof.
    pub equality_proof_instruction_offset: i8,
    /// Relative location of the
    /// `ProofInstruction::VerifyBatchedGroupedCiphertext3HandlesValidity`
    /// instruction to the `Transfer` instruction in the transaction. If the
    /// offset is `0`, then use a context state account for the proof.
    pub ciphertext_validity_proof_instruction_offset: i8,
    /// Relative location of the `ProofInstruction::BatchedRangeProofU128Data`
    /// instruction to the `Transfer` instruction in the transaction. If the
    /// offset is `0`, then use a context state account for the proof.
    pub range_proof_instruction_offset: i8,
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

/// Data expected by `ConfidentialTransferInstruction::TransferWithFee`
#[cfg_attr(feature = "serde-traits", derive(Serialize, Deserialize))]
#[derive(Clone, Copy, Debug, PartialEq, Pod, Zeroable)]
#[repr(C)]
pub struct TransferWithFeeInstructionData {
    /// The new source decryptable balance if the transfer succeeds
    #[cfg_attr(feature = "serde-traits", serde(with = "aeciphertext_fromstr"))]
    pub new_source_decryptable_available_balance: DecryptableBalance,
    /// Relative location of the
    /// `ProofInstruction::VerifyCiphertextCommitmentEquality` instruction
    /// to the `TransferWithFee` instruction in the transaction. If the offset
    /// is `0`, then use a context state account for the proof.
    pub equality_proof_instruction_offset: i8,
    /// Relative location of the
    /// `ProofInstruction::VerifyBatchedGroupedCiphertext3HandlesValidity`
    /// instruction to the `TransferWithFee` instruction in the transaction.
    /// If the offset is `0`, then use a context state account for the
    /// proof.
    pub transfer_amount_ciphertext_validity_proof_instruction_offset: i8,
    /// Relative location of the `ProofInstruction::VerifyPercentageWithFee`
    /// instruction to the `TransferWithFee` instruction in the transaction.
    /// If the offset is `0`, then use a context state account for the
    /// proof.
    pub fee_sigma_proof_instruction_offset: i8,
    /// Relative location of the
    /// `ProofInstruction::VerifyBatchedGroupedCiphertext2HandlesValidity`
    /// instruction to the `TransferWithFee` instruction in the transaction.
    /// If the offset is `0`, then use a context state account for the
    /// proof.
    pub fee_ciphertext_validity_proof_instruction_offset: i8,
    /// Relative location of the `ProofInstruction::BatchedRangeProofU256Data`
    /// instruction to the `TransferWithFee` instruction in the transaction.
    /// If the offset is `0`, then use a context state account for the
    /// proof.
    pub range_proof_instruction_offset: i8,
}

/// Create a `InitializeMint` instruction
pub fn initialize_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    authority: Option<Pubkey>,
    auto_approve_new_accounts: bool,
    auditor_elgamal_pubkey: Option<PodElGamalPubkey>,
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
    auditor_elgamal_pubkey: Option<PodElGamalPubkey>,
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
    decryptable_zero_balance: PodAeCiphertext,
    maximum_pending_balance_credit_counter: u64,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data_location: ProofLocation<PubkeyValidityProofData>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;

    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*mint, false),
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
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::ConfigureAccount,
        &ConfigureAccountInstructionData {
            decryptable_zero_balance,
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
    decryptable_zero_balance: PodAeCiphertext,
    maximum_pending_balance_credit_counter: u64,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data_location: ProofLocation<PubkeyValidityProofData>,
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
        match proof_data {
            ProofData::InstructionData(data) => instructions
                .push(ProofInstruction::VerifyPubkeyValidity.encode_verify_proof(None, data)),
            ProofData::RecordAccount(address, offset) => instructions.push(
                ProofInstruction::VerifyPubkeyValidity
                    .encode_verify_proof_from_account(None, address, offset),
            ),
        };
    }

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
    proof_data_location: ProofLocation<ZeroCiphertextProofData>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![AccountMeta::new(*token_account, false)];

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
    proof_data_location: ProofLocation<ZeroCiphertextProofData>,
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
        match proof_data {
            ProofData::InstructionData(data) => instructions
                .push(ProofInstruction::VerifyZeroCiphertext.encode_verify_proof(None, data)),
            ProofData::RecordAccount(address, offset) => instructions.push(
                ProofInstruction::VerifyZeroCiphertext
                    .encode_verify_proof_from_account(None, address, offset),
            ),
        };
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
    equality_proof_data_location: ProofLocation<CiphertextCommitmentEqualityProofData>,
    range_proof_data_location: ProofLocation<BatchedRangeProofU64Data>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*mint, false),
    ];

    // if at least one of the proof locations is an instruction offset, sysvar
    // account is needed
    if equality_proof_data_location.is_instruction_offset()
        || range_proof_data_location.is_instruction_offset()
    {
        accounts.push(AccountMeta::new_readonly(sysvar::instructions::id(), false));
    }

    let equality_proof_instruction_offset = match equality_proof_data_location {
        ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) => {
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

    let range_proof_instruction_offset = match range_proof_data_location {
        ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) => {
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
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::Withdraw,
        &WithdrawInstructionData {
            amount: amount.into(),
            decimals,
            new_decryptable_available_balance,
            equality_proof_instruction_offset,
            range_proof_instruction_offset,
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
    new_decryptable_available_balance: PodAeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    equality_proof_data_location: ProofLocation<CiphertextCommitmentEqualityProofData>,
    range_proof_data_location: ProofLocation<BatchedRangeProofU64Data>,
) -> Result<Vec<Instruction>, ProgramError> {
    let mut instructions = vec![inner_withdraw(
        token_program_id,
        token_account,
        mint,
        amount,
        decimals,
        new_decryptable_available_balance,
        authority,
        multisig_signers,
        equality_proof_data_location,
        range_proof_data_location,
    )?];

    let mut expected_instruction_offset = 1;

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        equality_proof_data_location
    {
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != expected_instruction_offset {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        match proof_data {
            ProofData::InstructionData(data) => instructions.push(
                ProofInstruction::VerifyCiphertextCommitmentEquality
                    .encode_verify_proof(None, data),
            ),
            ProofData::RecordAccount(address, offset) => instructions.push(
                ProofInstruction::VerifyCiphertextCommitmentEquality
                    .encode_verify_proof_from_account(None, address, offset),
            ),
        };

        expected_instruction_offset += 1;
    };

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        range_proof_data_location
    {
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != expected_instruction_offset {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        match proof_data {
            ProofData::InstructionData(data) => instructions
                .push(ProofInstruction::VerifyBatchedRangeProofU64.encode_verify_proof(None, data)),
            ProofData::RecordAccount(address, offset) => instructions.push(
                ProofInstruction::VerifyBatchedRangeProofU64
                    .encode_verify_proof_from_account(None, address, offset),
            ),
        };
    };

    Ok(instructions)
}

/// Create an inner `Transfer` instruction
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
    equality_proof_data_location: ProofLocation<CiphertextCommitmentEqualityProofData>,
    ciphertext_validity_proof_data_location: ProofLocation<
        BatchedGroupedCiphertext3HandlesValidityProofData,
    >,
    range_proof_data_location: ProofLocation<BatchedRangeProofU128Data>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*source_token_account, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*destination_token_account, false),
    ];

    // if at least one of the proof locations is an instruction offset, sysvar
    // account is needed
    if equality_proof_data_location.is_instruction_offset()
        || ciphertext_validity_proof_data_location.is_instruction_offset()
        || range_proof_data_location.is_instruction_offset()
    {
        accounts.push(AccountMeta::new_readonly(sysvar::instructions::id(), false));
    }

    let equality_proof_instruction_offset = match equality_proof_data_location {
        ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) => {
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

    let ciphertext_validity_proof_instruction_offset = match ciphertext_validity_proof_data_location
    {
        ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) => {
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

    let range_proof_instruction_offset = match range_proof_data_location {
        ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) => {
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
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::Transfer,
        &TransferInstructionData {
            new_source_decryptable_available_balance,
            equality_proof_instruction_offset,
            ciphertext_validity_proof_instruction_offset,
            range_proof_instruction_offset,
        },
    ))
}

/// Create a `Transfer` instruction
#[allow(clippy::too_many_arguments)]
pub fn transfer(
    token_program_id: &Pubkey,
    source_token_account: &Pubkey,
    mint: &Pubkey,
    destination_token_account: &Pubkey,
    new_source_decryptable_available_balance: DecryptableBalance,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    equality_proof_data_location: ProofLocation<CiphertextCommitmentEqualityProofData>,
    ciphertext_validity_proof_data_location: ProofLocation<
        BatchedGroupedCiphertext3HandlesValidityProofData,
    >,
    range_proof_data_location: ProofLocation<BatchedRangeProofU128Data>,
) -> Result<Vec<Instruction>, ProgramError> {
    let mut instructions = vec![inner_transfer(
        token_program_id,
        source_token_account,
        mint,
        destination_token_account,
        new_source_decryptable_available_balance,
        authority,
        multisig_signers,
        equality_proof_data_location,
        ciphertext_validity_proof_data_location,
        range_proof_data_location,
    )?];

    let mut expected_instruction_offset = 1;

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        equality_proof_data_location
    {
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != expected_instruction_offset {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        match proof_data {
            ProofData::InstructionData(data) => instructions.push(
                ProofInstruction::VerifyCiphertextCommitmentEquality
                    .encode_verify_proof(None, data),
            ),
            ProofData::RecordAccount(address, offset) => instructions.push(
                ProofInstruction::VerifyCiphertextCommitmentEquality
                    .encode_verify_proof_from_account(None, address, offset),
            ),
        };

        expected_instruction_offset += 1;
    }

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        ciphertext_validity_proof_data_location
    {
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != expected_instruction_offset {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        match proof_data {
            ProofData::InstructionData(data) => instructions.push(
                ProofInstruction::VerifyBatchedGroupedCiphertext3HandlesValidity
                    .encode_verify_proof(None, data),
            ),
            ProofData::RecordAccount(address, offset) => instructions.push(
                ProofInstruction::VerifyBatchedGroupedCiphertext3HandlesValidity
                    .encode_verify_proof_from_account(None, address, offset),
            ),
        };

        expected_instruction_offset += 1;
    }

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        range_proof_data_location
    {
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != expected_instruction_offset {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        match proof_data {
            ProofData::InstructionData(data) => instructions.push(
                ProofInstruction::VerifyBatchedRangeProofU128.encode_verify_proof(None, data),
            ),
            ProofData::RecordAccount(address, offset) => instructions.push(
                ProofInstruction::VerifyBatchedRangeProofU128
                    .encode_verify_proof_from_account(None, address, offset),
            ),
        };
    }

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
    new_decryptable_available_balance: PodAeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    inner_apply_pending_balance(
        token_program_id,
        token_account,
        pending_balance_instructions,
        new_decryptable_available_balance,
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

/// Create an inner `TransferWithFee` instruction
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
    equality_proof_data_location: ProofLocation<CiphertextCommitmentEqualityProofData>,
    transfer_amount_ciphertext_validity_proof_data_location: ProofLocation<
        BatchedGroupedCiphertext3HandlesValidityProofData,
    >,
    fee_sigma_proof_data_location: ProofLocation<PercentageWithCapProofData>,
    fee_ciphertext_validity_proof_data_location: ProofLocation<
        BatchedGroupedCiphertext2HandlesValidityProofData,
    >,
    range_proof_data_location: ProofLocation<BatchedRangeProofU256Data>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*source_token_account, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new(*destination_token_account, false),
    ];

    // if at least one of the proof locations is an instruction offset, sysvar
    // account is needed
    if equality_proof_data_location.is_instruction_offset()
        || transfer_amount_ciphertext_validity_proof_data_location.is_instruction_offset()
        || fee_sigma_proof_data_location.is_instruction_offset()
        || fee_ciphertext_validity_proof_data_location.is_instruction_offset()
        || range_proof_data_location.is_instruction_offset()
    {
        accounts.push(AccountMeta::new_readonly(sysvar::instructions::id(), false));
    }

    let equality_proof_instruction_offset = match equality_proof_data_location {
        ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) => {
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

    let transfer_amount_ciphertext_validity_proof_instruction_offset =
        match transfer_amount_ciphertext_validity_proof_data_location {
            ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) => {
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

    let fee_sigma_proof_instruction_offset = match fee_sigma_proof_data_location {
        ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) => {
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

    let fee_ciphertext_validity_proof_instruction_offset =
        match fee_ciphertext_validity_proof_data_location {
            ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) => {
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

    let range_proof_instruction_offset = match range_proof_data_location {
        ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) => {
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
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::TransferWithFee,
        &TransferWithFeeInstructionData {
            new_source_decryptable_available_balance,
            equality_proof_instruction_offset,
            transfer_amount_ciphertext_validity_proof_instruction_offset,
            fee_sigma_proof_instruction_offset,
            fee_ciphertext_validity_proof_instruction_offset,
            range_proof_instruction_offset,
        },
    ))
}

/// Create a `TransferWithFee` instruction
#[allow(clippy::too_many_arguments)]
pub fn transfer_with_fee(
    token_program_id: &Pubkey,
    source_token_account: &Pubkey,
    mint: &Pubkey,
    destination_token_account: &Pubkey,
    new_source_decryptable_available_balance: DecryptableBalance,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    equality_proof_data_location: ProofLocation<CiphertextCommitmentEqualityProofData>,
    transfer_amount_ciphertext_validity_proof_data_location: ProofLocation<
        BatchedGroupedCiphertext3HandlesValidityProofData,
    >,
    fee_sigma_proof_data_location: ProofLocation<PercentageWithCapProofData>,
    fee_ciphertext_validity_proof_data_location: ProofLocation<
        BatchedGroupedCiphertext2HandlesValidityProofData,
    >,
    range_proof_data_location: ProofLocation<BatchedRangeProofU256Data>,
) -> Result<Vec<Instruction>, ProgramError> {
    let mut instructions = vec![inner_transfer_with_fee(
        token_program_id,
        source_token_account,
        mint,
        destination_token_account,
        new_source_decryptable_available_balance,
        authority,
        multisig_signers,
        equality_proof_data_location,
        transfer_amount_ciphertext_validity_proof_data_location,
        fee_sigma_proof_data_location,
        fee_ciphertext_validity_proof_data_location,
        range_proof_data_location,
    )?];

    let mut expected_instruction_offset = 1;

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        equality_proof_data_location
    {
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != expected_instruction_offset {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        match proof_data {
            ProofData::InstructionData(data) => instructions.push(
                ProofInstruction::VerifyCiphertextCommitmentEquality
                    .encode_verify_proof(None, data),
            ),
            ProofData::RecordAccount(address, offset) => instructions.push(
                ProofInstruction::VerifyCiphertextCommitmentEquality
                    .encode_verify_proof_from_account(None, address, offset),
            ),
        };
        expected_instruction_offset += 1;
    }

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        transfer_amount_ciphertext_validity_proof_data_location
    {
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != expected_instruction_offset {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        match proof_data {
            ProofData::InstructionData(data) => instructions.push(
                ProofInstruction::VerifyBatchedGroupedCiphertext3HandlesValidity
                    .encode_verify_proof(None, data),
            ),
            ProofData::RecordAccount(address, offset) => instructions.push(
                ProofInstruction::VerifyBatchedGroupedCiphertext3HandlesValidity
                    .encode_verify_proof_from_account(None, address, offset),
            ),
        };
        expected_instruction_offset += 1;
    }

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        fee_sigma_proof_data_location
    {
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != expected_instruction_offset {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        match proof_data {
            ProofData::InstructionData(data) => instructions
                .push(ProofInstruction::VerifyPercentageWithCap.encode_verify_proof(None, data)),
            ProofData::RecordAccount(address, offset) => instructions.push(
                ProofInstruction::VerifyPercentageWithCap
                    .encode_verify_proof_from_account(None, address, offset),
            ),
        };
        expected_instruction_offset += 1;
    }

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        fee_ciphertext_validity_proof_data_location
    {
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != expected_instruction_offset {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        match proof_data {
            ProofData::InstructionData(data) => instructions.push(
                ProofInstruction::VerifyBatchedGroupedCiphertext2HandlesValidity
                    .encode_verify_proof(None, data),
            ),
            ProofData::RecordAccount(address, offset) => instructions.push(
                ProofInstruction::VerifyBatchedGroupedCiphertext2HandlesValidity
                    .encode_verify_proof_from_account(None, address, offset),
            ),
        };
        expected_instruction_offset += 1;
    }

    if let ProofLocation::InstructionOffset(proof_instruction_offset, proof_data) =
        range_proof_data_location
    {
        let proof_instruction_offset: i8 = proof_instruction_offset.into();
        if proof_instruction_offset != expected_instruction_offset {
            return Err(TokenError::InvalidProofInstructionOffset.into());
        }
        match proof_data {
            ProofData::InstructionData(data) => instructions.push(
                ProofInstruction::VerifyBatchedRangeProofU256.encode_verify_proof(None, data),
            ),
            ProofData::RecordAccount(address, offset) => instructions.push(
                ProofInstruction::VerifyBatchedRangeProofU256
                    .encode_verify_proof_from_account(None, address, offset),
            ),
        };
    }

    Ok(instructions)
}

/// Create a `ConfigureAccountWithRegistry` instruction
pub fn configure_account_with_registry(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    elgamal_registry_account: &Pubkey,
    payer: Option<&Pubkey>,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*elgamal_registry_account, false),
    ];
    if let Some(payer) = payer {
        accounts.push(AccountMeta::new(*payer, true));
        accounts.push(AccountMeta::new_readonly(system_program::id(), false));
    }

    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::ConfigureAccountWithRegistry,
        &(),
    ))
}
