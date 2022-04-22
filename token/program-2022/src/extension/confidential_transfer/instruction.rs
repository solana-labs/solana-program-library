#[cfg(not(target_arch = "bpf"))]
use solana_zk_token_sdk::encryption::{auth_encryption::AeCiphertext, elgamal::ElGamalPubkey};
pub use solana_zk_token_sdk::zk_token_proof_instruction::*;
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
    solana_zk_token_sdk::zk_token_elgamal::pod,
    std::convert::TryFrom,
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
    ///   `ConfidentialTransferMint`
    ///
    InitializeMint,

    /// Updates the confidential transfer mint configuration for a mint.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token mint.
    ///   1. `[signer]` Confidential transfer mint authority.
    ///   2. `[signer]` New confidential transfer mint authority.
    ///
    /// Data expected by this instruction:
    ///   `ConfidentialTransferMint`
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
    /// Upon success confidential deposits and transfers are enabled, use the
    /// `DisableBalanceCredits` instruction to disable.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writeable]` The SPL Token account.
    ///   1. `[]` The corresponding SPL Token mint.
    ///   2. `[signer]` The single source account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writeable]` The SPL Token account.
    ///   1. `[]` The corresponding SPL Token mint.
    ///   2. `[]` The multisig source account owner.
    ///   3.. `[signer]` Required M signer accounts for the SPL Token Multisig account.
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

    /// Prepare a token account for closing.  The account must not hold any confidential tokens in
    /// its pending or available balances. Use
    /// `ConfidentialTransferInstruction::DisableBalanceCredits` to block balance credit changes
    /// first if necessary.
    ///
    /// Note that a newly configured account is always empty, so this instruction is not required
    /// prior to account closing if no instructions beyond
    /// `ConfidentialTransferInstruction::ConfigureAccount` have affected the token account.
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` Instructions sysvar.
    ///   2. `[signer]` The single account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The SPL Token account.
    ///   1. `[]` Instructions sysvar.
    ///   2. `[]` The multisig account owner.
    ///   3.. `[signer]` Required M signer accounts for the SPL Token Multisig account.
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
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The source SPL Token account.
    ///   1. `[writable]` The destination SPL Token account with confidential transfers configured.
    ///   2. `[]` The token mint.
    ///   3. `[signer]` The single source account owner or delegate.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The source SPL Token account.
    ///   1. `[writable]` The destination SPL Token account with confidential transfers configured.
    ///   2. `[]` The token mint.
    ///   3. `[]` The multisig source account owner or delegate.
    ///   4.. `[signer]` Required M signer accounts for the SPL Token Multisig account.
    ///
    /// Data expected by this instruction:
    ///   `DepositInstructionData`
    ///
    Deposit,

    /// Withdraw SPL Tokens from the available balance of a confidential token account.
    ///
    /// Fails if the source or destination accounts are frozen.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   * Single owner/delegate
    ///   0. `[writable]` The source SPL Token account with confidential transfers configured.
    ///   1. `[writable]` The destination SPL Token account.
    ///   2. `[]` The token mint.
    ///   3. `[]` Instructions sysvar.
    ///   4. `[signer]` The single source account owner.
    ///
    ///   * Multisignature owner/delegate
    ///   0. `[writable]` The source SPL Token account with confidential transfers configured.
    ///   1. `[writable]` The destination SPL Token account.
    ///   2. `[]` The token mint.
    ///   3. `[]` Instructions sysvar.
    ///   4. `[]` The multisig  source account owner.
    ///   5.. `[signer]` Required M signer accounts for the SPL Token Multisig account.
    ///
    /// Data expected by this instruction:
    ///   `WithdrawInstructionData`
    ///
    Withdraw,

    /// Transfer tokens confidentially.
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

    /// Transfer tokens confidentially with fee.
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
    ///   `TransferWithFeeInstructionData`
    ///
    TransferWithFee,

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

    /// Enable confidential transfer `Deposit` and `Transfer` instructions for a token account.
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
    EnableBalanceCredits,

    /// Disable confidential transfer `Deposit` and `Transfer` instructions for a token account.
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
    DisableBalanceCredits,

    /// Transfer all withheld confidential tokens in the mint to an account. Signed by the mint's
    /// withdraw withheld tokens authority.
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

/// Data expected by `ConfidentialTransferInstruction::ConfigureAccount`
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct ConfigureAccountInstructionData {
    /// The public key associated with the account
    pub encryption_pubkey: EncryptionPubkey,
    /// The decryptable balance (always 0) once the configure account succeeds
    pub decryptable_zero_balance: DecryptableBalance,
}

/// Data expected by `ConfidentialTransferInstruction::EmptyAccount`
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct EmptyAccountInstructionData {
    /// Relative location of the `ProofInstruction::VerifyCloseAccount` instruction to the
    /// `EmptyAccount` instruction in the transaction
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialTransferInstruction::Deposit`
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct DepositInstructionData {
    /// The amount of tokens to deposit
    pub amount: PodU64,
    /// Expected number of base 10 digits to the right of the decimal place
    pub decimals: u8,
}

/// Data expected by `ConfidentialTransferInstruction::Withdraw`
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct WithdrawInstructionData {
    /// The amount of tokens to withdraw
    pub amount: PodU64,
    /// Expected number of base 10 digits to the right of the decimal place
    pub decimals: u8,
    /// The new decryptable balance if the withrawal succeeds
    pub new_decryptable_available_balance: DecryptableBalance,
    /// Relative location of the `ProofInstruction::VerifyWithdraw` instruction to the `Withdraw`
    /// instruction in the transaction
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialTransferInstruction::Transfer`
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct TransferInstructionData {
    /// The new source decryptable balance if the transfer succeeds
    pub new_source_decryptable_available_balance: DecryptableBalance,
    /// Relative location of the `ProofInstruction::VerifyTransfer` instruction to the
    /// `Transfer` instruction in the transaction
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialTransferInstruction::TransferWithFee`
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct TransferWithFeeInstructionData {
    /// The new source decryptable balance if the transfer succeeds
    pub new_source_decryptable_available_balance: DecryptableBalance,
    /// Relative location of the `ProofInstruction::VerifyTransfer` instruction to the
    /// `Transfer` instruction in the transaction
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialTransferInstruction::ApplyPendingBalance`
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct ApplyPendingBalanceData {
    /// The expected number of pending balance credits since the last successful
    /// `ApplyPendingBalance` instruction
    pub expected_pending_balance_credit_counter: PodU64,
    /// The new decryptable balance if the pending balance is applied successfully
    pub new_decryptable_available_balance: pod::AeCiphertext,
}

/// Data expected by `ConfidentialTransferInstruction::WithdrawWithheldTokensFromMint`
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct WithdrawWithheldTokensFromMintData {
    /// Relative location of the `ProofInstruction::VerifyWithdrawWithheld` instruction to the
    /// `WithdrawWithheldTokensFromMint` instruction in the transaction
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialTransferInstruction::WithdrawWithheldTokensFromAccounts`
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct WithdrawWithheldTokensFromAccountsData {
    /// Number of token accounts harvested
    pub num_token_accounts: u8,
    /// Relative location of the `ProofInstruction::VerifyWithdrawWithheld` instruction to the
    /// `VerifyWithdrawWithheldTokensFromAccounts` instruction in the transaction
    pub proof_instruction_offset: i8,
}

/// Create a `InitializeMint` instruction
pub fn initialize_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    ct_mint: &ConfidentialTransferMint,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![AccountMeta::new(*mint, false)];
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::InitializeMint,
        ct_mint,
    ))
}

/// Create a `UpdateMint` instruction
pub fn update_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    new_ct_mint: &ConfidentialTransferMint,
    authority: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(
            new_ct_mint.authority,
            new_ct_mint.authority != Pubkey::default(),
        ),
    ];
    Ok(encode_instruction(
        token_program_id,
        accounts,
        TokenInstruction::ConfidentialTransferExtension,
        ConfidentialTransferInstruction::UpdateMint,
        new_ct_mint,
    ))
}

/// Create a `ConfigureAccount` instruction
#[cfg(not(target_arch = "bpf"))]
pub fn configure_account(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    mint: &Pubkey,
    encryption_pubkey: ElGamalPubkey,
    decryptable_zero_balance: AeCiphertext,
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
        ConfidentialTransferInstruction::ConfigureAccount,
        &ConfigureAccountInstructionData {
            encryption_pubkey: encryption_pubkey.into(),
            decryptable_zero_balance: decryptable_zero_balance.into(),
        },
    ))
}

/// Create an `ApproveAccount` instruction
pub fn approve_account(
    token_program_id: &Pubkey,
    account_to_approve: &Pubkey,
    mint: &Pubkey,
    authority: &Pubkey,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let accounts = vec![
        AccountMeta::new(*account_to_approve, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*authority, true),
    ];
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
    proof_instruction_offset: i8,
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
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
    proof_data: &CloseAccountData,
) -> Result<Vec<Instruction>, ProgramError> {
    Ok(vec![
        verify_close_account(proof_data),
        inner_empty_account(
            token_program_id,
            token_account,
            authority,
            multisig_signers,
            -1,
        )?, // calls check_program_account
    ])
}

/// Create a `Deposit` instruction
#[allow(clippy::too_many_arguments)]
pub fn deposit(
    token_program_id: &Pubkey,
    source_token_account: &Pubkey,
    mint: &Pubkey,
    destination_token_account: &Pubkey,
    amount: u64,
    decimals: u8,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    check_program_account(token_program_id)?;
    let mut accounts = vec![
        AccountMeta::new(*source_token_account, false),
        AccountMeta::new(*destination_token_account, false),
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
    source_token_account: &Pubkey,
    destination_token_account: &Pubkey,
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
#[cfg(not(target_arch = "bpf"))]
pub fn withdraw(
    token_program_id: &Pubkey,
    source_token_account: &Pubkey,
    destination_token_account: &Pubkey,
    mint: &Pubkey,
    amount: u64,
    decimals: u8,
    new_decryptable_available_balance: AeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data: &WithdrawData,
) -> Result<Vec<Instruction>, ProgramError> {
    Ok(vec![
        verify_withdraw(proof_data),
        inner_withdraw(
            token_program_id,
            source_token_account,
            destination_token_account,
            mint,
            amount,
            decimals,
            new_decryptable_available_balance.into(),
            authority,
            multisig_signers,
            -1,
        )?, // calls check_program_account
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

/// Create a `Transfer` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_arch = "bpf"))]
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
        verify_transfer(proof_data),
        inner_transfer(
            token_program_id,
            source_token_account,
            destination_token_account,
            mint,
            new_source_decryptable_available_balance.into(),
            authority,
            multisig_signers,
            -1,
        )?, // calls check_program_account
    ])
}

/// Create a inner `TransferWithFee` instruction
///
/// This instruction is suitable for use with a cross-program `invoke`
#[allow(clippy::too_many_arguments)]
pub fn inner_transfer_with_fee(
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
        ConfidentialTransferInstruction::TransferWithFee,
        &TransferWithFeeInstructionData {
            new_source_decryptable_available_balance,
            proof_instruction_offset,
        },
    ))
}

/// Create a `Transfer` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_arch = "bpf"))]
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
        verify_transfer_with_fee(proof_data),
        inner_transfer_with_fee(
            token_program_id,
            source_token_account,
            destination_token_account,
            mint,
            new_source_decryptable_available_balance.into(),
            authority,
            multisig_signers,
            -1,
        )?, // calls check_program_account
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
#[cfg(not(target_arch = "bpf"))]
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

/// Create a `EnableBalanceCredits` instruction
pub fn enable_balance_credits(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    enable_or_disable_balance_credits(
        ConfidentialTransferInstruction::EnableBalanceCredits,
        token_program_id,
        token_account,
        authority,
        multisig_signers,
    )
}

/// Create a `DisableBalanceCredits` instruction
pub fn disable_balance_credits(
    token_program_id: &Pubkey,
    token_account: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Result<Instruction, ProgramError> {
    enable_or_disable_balance_credits(
        ConfidentialTransferInstruction::DisableBalanceCredits,
        token_program_id,
        token_account,
        authority,
        multisig_signers,
    )
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
pub fn withdraw_withheld_tokens_from_mint(
    token_program_id: &Pubkey,
    mint: &Pubkey,
    destination: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data: &WithdrawWithheldTokensData,
) -> Result<Vec<Instruction>, ProgramError> {
    Ok(vec![
        verify_withdraw_withheld_tokens(proof_data),
        inner_withdraw_withheld_tokens_from_mint(
            token_program_id,
            mint,
            destination,
            authority,
            multisig_signers,
            -1,
        )?,
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
        verify_withdraw_withheld_tokens(proof_data),
        inner_withdraw_withheld_tokens_from_accounts(
            token_program_id,
            mint,
            destination,
            authority,
            multisig_signers,
            sources,
            -1,
        )?,
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
