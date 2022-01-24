#[cfg(not(target_arch = "bpf"))]
use solana_zk_token_sdk::encryption::{auth_encryption::AeCiphertext, elgamal::ElGamalPubkey};
pub use solana_zk_token_sdk::zk_token_proof_instruction::*;
use {
    crate::{
        extension::confidential_transfer::ConfidentialTransferMint, id,
        instruction::TokenInstruction, pod::*,
    },
    bytemuck::{Pod, Zeroable},
    num_derive::{FromPrimitive, ToPrimitive},
    num_traits::{FromPrimitive, ToPrimitive},
    solana_program::{
        instruction::{AccountMeta, Instruction},
        program_error::ProgramError,
        pubkey::Pubkey,
        sysvar,
    },
    solana_zk_token_sdk::zk_token_elgamal::pod,
};

/// Confidential Transfer extension instructions
#[derive(Clone, Copy, Debug, FromPrimitive, ToPrimitive)]
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
    ///   0. `[writable]` The SPL Token mint
    //
    /// Data expected by this instruction:
    ///   `ConfidentialTransferMint`
    ///
    InitializeMint,

    /// Updates the confidential transfer mint configuration for a mint.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token mint
    ///   1. `[signer]` Confidential transfer mint authority
    ///   2. `[signer]` New confidential transfer mint authority
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
    /// Upon success confidential deposits and transfers are disabled, use the
    /// `EnableBalanceCredits` instruction to enable them.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writeable]` The SPL Token account
    ///   1. `[]` The corresponding SPL Token mint
    ///   2. `[signer]` The single source account owner
    /// or:
    ///   2. `[]` The multisig source account owner
    ///   3.. `[signer]` Required M signer accounts for the SPL Token Multisig account
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
    ///   0. `[writable]` The SPL Token account to approve
    ///   1. `[]` The SPL Token mint
    ///   2. `[signer]` Confidential transfer auditor authority
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
    ///
    ///   0. `[writable]` The SPL Token account
    ///   1. `[]` Instructions sysvar
    ///   2. `[signer]` The single account owner
    /// or:
    ///   2. `[]` The multisig account owner
    ///   3.. `[signer]` Required M signer accounts for the SPL Token Multisig account
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
    ///   0. `[writable]` The source SPL Token account
    ///   1. `[writable]` The destination SPL Token account with confidential transfers configured
    ///   2. `[]` The token mint.
    ///   3. `[signer]` The single source account owner or delegate
    /// or:
    ///   3. `[]` The multisig source account owner or delegate.
    ///   4.. `[signer]` Required M signer accounts for the SPL Token Multisig account
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
    ///   0. `[writable]` The source SPL Token account with confidential transfers configured
    ///   1. `[writable]` The destination SPL Token account
    ///   2. `[]` The token mint.
    ///   3. `[]` Instructions sysvar
    ///   4. `[signer]` The single source account owner
    /// or:
    ///   4. `[]` The multisig  source account owner
    ///   5.. `[signer]` Required M signer accounts for the SPL Token Multisig account
    ///
    /// Data expected by this instruction:
    ///   `WithdrawInstructionData`
    ///
    Withdraw,

    /// Transfer tokens confidentially.
    ///
    ///   1. `[writable]` The source SPL Token account
    ///   2. `[writable]` The destination SPL Token account
    ///   3. `[]` The token mint
    ///   4. `[]` Instructions sysvar
    ///   5. `[signer]` The single source account owner
    /// or:
    ///   5. `[]` The multisig  source account owner
    ///   6.. `[signer]` Required M signer accounts for the SPL Token Multisig account
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
    ///   0. `[writable]` The SPL Token account
    ///   1. `[signer]` The single account owner
    /// or:
    ///   1. `[]` The multisig account owner
    ///   2.. `[signer]` Required M signer accounts for the SPL Token Multisig account
    ///
    /// Data expected by this instruction:
    ///   `ApplyPendingBalanceData`
    ///
    ApplyPendingBalance,

    /// Enable confidential transfer `Deposit` and `Transfer` instructions for a token account.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token account
    ///   1. `[signer]` Single authority
    /// or:
    ///   1. `[]` Multisig authority
    ///   2.. `[signer]` Required M signer accounts for the SPL Token Multisig account
    ///
    /// Data expected by this instruction:
    ///   None
    ///
    EnableBalanceCredits,

    /// Disable confidential transfer `Deposit` and `Transfer` instructions for a token account.
    ///
    /// Accounts expected by this instruction:
    ///
    ///   0. `[writable]` The SPL Token account
    ///   1. `[signer]` The single account owner
    /// or:
    ///   1. `[]` The multisig account owner
    ///   2.. `[signer]` Required M signer accounts for the SPL Token Multisig account
    ///
    /// Data expected by this instruction:
    ///   None
    ///
    DisableBalanceCredits,
}

/// Data expected by `ConfidentialTransferInstruction::ConfigureAccount`
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct ConfigureAccountInstructionData {
    /// The public key associated with the account
    pub elgamal_pk: pod::ElGamalPubkey,
    /// The decryptable balance (always 0) once the configure account succeeds
    pub decryptable_zero_balance: pod::AeCiphertext,
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
    pub new_decryptable_available_balance: pod::AeCiphertext,
    /// Relative location of the `ProofInstruction::VerifyWithdraw` instruction to the `Withdraw`
    /// instruction in the transaction
    pub proof_instruction_offset: i8,
}

/// Data expected by `ConfidentialTransferInstruction::Transfer`
#[derive(Clone, Copy, Pod, Zeroable)]
#[repr(C)]
pub struct TransferInstructionData {
    /// The new source decryptable balance if the transfer succeeds
    pub new_source_decryptable_available_balance: pod::AeCiphertext,
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

pub(crate) fn decode_instruction_type(
    input: &[u8],
) -> Result<ConfidentialTransferInstruction, ProgramError> {
    if input.is_empty() {
        Err(ProgramError::InvalidInstructionData)
    } else {
        FromPrimitive::from_u8(input[0]).ok_or(ProgramError::InvalidInstructionData)
    }
}

pub(crate) fn decode_instruction_data<T: Pod>(input: &[u8]) -> Result<&T, ProgramError> {
    if input.is_empty() {
        Err(ProgramError::InvalidInstructionData)
    } else {
        pod_from_bytes(&input[1..])
    }
}

fn encode_instruction<T: Pod>(
    accounts: Vec<AccountMeta>,
    instruction_type: ConfidentialTransferInstruction,
    instruction_data: &T,
) -> Instruction {
    let mut data = TokenInstruction::ConfidentialTransferExtension.pack();
    data.push(ToPrimitive::to_u8(&instruction_type).unwrap());
    data.extend_from_slice(bytemuck::bytes_of(instruction_data));
    Instruction {
        program_id: id(),
        accounts,
        data,
    }
}

/// Create a `InitializeMint` instruction
pub fn initialize_mint(mint: &Pubkey, auditor: &ConfidentialTransferMint) -> Instruction {
    let accounts = vec![AccountMeta::new(*mint, false)];
    encode_instruction(
        accounts,
        ConfidentialTransferInstruction::InitializeMint,
        auditor,
    )
}
/// Create a `UpdateMint` instruction
pub fn update_mint(
    mint: &Pubkey,
    new_auditor: &ConfidentialTransferMint,
    authority: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*mint, false),
        AccountMeta::new_readonly(*authority, true),
        AccountMeta::new_readonly(
            new_auditor.authority,
            new_auditor.authority != Pubkey::default(),
        ),
    ];
    encode_instruction(
        accounts,
        ConfidentialTransferInstruction::UpdateMint,
        new_auditor,
    )
}

/// Create a `ConfigureAccount` instruction
#[cfg(not(target_arch = "bpf"))]
pub fn configure_account(
    token_account: &Pubkey,
    mint: &Pubkey,
    elgamal_pk: ElGamalPubkey,
    decryptable_zero_balance: AeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Vec<Instruction> {
    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    vec![encode_instruction(
        accounts,
        ConfidentialTransferInstruction::ConfigureAccount,
        &ConfigureAccountInstructionData {
            elgamal_pk: elgamal_pk.into(),
            decryptable_zero_balance: decryptable_zero_balance.into(),
        },
    )]
}

/// Create an `ApproveAccount` instruction
pub fn approve_account(
    mint: &Pubkey,
    account_to_approve: &Pubkey,
    authority: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*account_to_approve, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*authority, true),
    ];
    encode_instruction(
        accounts,
        ConfidentialTransferInstruction::ApproveAccount,
        &(),
    )
}

/// Create an inner `EmptyAccount` instruction
///
/// This instruction is suitable for use with a cross-program `invoke`
pub fn inner_empty_account(
    token_account: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_instruction_offset: i8,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*token_account, false),
        AccountMeta::new_readonly(sysvar::instructions::id(), false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    encode_instruction(
        accounts,
        ConfidentialTransferInstruction::EmptyAccount,
        &EmptyAccountInstructionData {
            proof_instruction_offset,
        },
    )
}

/// Create a `EmptyAccount` instruction
pub fn empty_account(
    token_account: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data: &CloseAccountData,
) -> Vec<Instruction> {
    vec![
        verify_close_account(proof_data),
        inner_empty_account(token_account, authority, multisig_signers, -1),
    ]
}

/// Create a `Deposit` instruction
pub fn deposit(
    source_token_account: &Pubkey,
    mint: &Pubkey,
    destination_token_account: &Pubkey,
    amount: u64,
    decimals: u8,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Vec<Instruction> {
    let mut accounts = vec![
        AccountMeta::new(*source_token_account, false),
        AccountMeta::new(*destination_token_account, false),
        AccountMeta::new_readonly(*mint, false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    vec![encode_instruction(
        accounts,
        ConfidentialTransferInstruction::Deposit,
        &DepositInstructionData {
            amount: amount.into(),
            decimals,
        },
    )]
}

/// Create a inner `Withdraw` instruction
///
/// This instruction is suitable for use with a cross-program `invoke`
#[allow(clippy::too_many_arguments)]
pub fn inner_withdraw(
    source_token_account: &Pubkey,
    destination_token_account: &Pubkey,
    mint: &Pubkey,
    amount: u64,
    decimals: u8,
    new_decryptable_available_balance: pod::AeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_instruction_offset: i8,
) -> Instruction {
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

    encode_instruction(
        accounts,
        ConfidentialTransferInstruction::Withdraw,
        &WithdrawInstructionData {
            amount: amount.into(),
            decimals,
            new_decryptable_available_balance,
            proof_instruction_offset,
        },
    )
}

/// Create a `Withdraw` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_arch = "bpf"))]
pub fn withdraw(
    source_token_account: &Pubkey,
    destination_token_account: &Pubkey,
    mint: &Pubkey,
    amount: u64,
    decimals: u8,
    new_decryptable_available_balance: AeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data: &WithdrawData,
) -> Vec<Instruction> {
    vec![
        verify_withdraw(proof_data),
        inner_withdraw(
            source_token_account,
            destination_token_account,
            mint,
            amount,
            decimals,
            new_decryptable_available_balance.into(),
            authority,
            multisig_signers,
            -1,
        ),
    ]
}

/// Create a inner `Transfer` instruction
///
/// This instruction is suitable for use with a cross-program `invoke`
#[allow(clippy::too_many_arguments)]
pub fn inner_transfer(
    source_token_account: &Pubkey,
    destination_token_account: &Pubkey,
    mint: &Pubkey,
    new_source_decryptable_available_balance: pod::AeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_instruction_offset: i8,
) -> Instruction {
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

    encode_instruction(
        accounts,
        ConfidentialTransferInstruction::Transfer,
        &TransferInstructionData {
            new_source_decryptable_available_balance,
            proof_instruction_offset,
        },
    )
}

/// Create a `Transfer` instruction
#[allow(clippy::too_many_arguments)]
#[cfg(not(target_arch = "bpf"))]
pub fn transfer(
    source_token_account: &Pubkey,
    destination_token_account: &Pubkey,
    mint: &Pubkey,
    new_source_decryptable_available_balance: AeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
    proof_data: &TransferData,
) -> Vec<Instruction> {
    vec![
        verify_transfer(proof_data),
        inner_transfer(
            source_token_account,
            destination_token_account,
            mint,
            new_source_decryptable_available_balance.into(),
            authority,
            multisig_signers,
            -1,
        ),
    ]
}

/// Create a inner `ApplyPendingBalance` instruction
///
/// This instruction is suitable for use with a cross-program `invoke`
#[allow(clippy::too_many_arguments)]
pub fn inner_apply_pending_balance(
    token_account: &Pubkey,
    expected_pending_balance_credit_counter: u64,
    new_decryptable_available_balance: pod::AeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    encode_instruction(
        accounts,
        ConfidentialTransferInstruction::ApplyPendingBalance,
        &ApplyPendingBalanceData {
            expected_pending_balance_credit_counter: expected_pending_balance_credit_counter.into(),
            new_decryptable_available_balance,
        },
    )
}

/// Create a `ApplyPendingBalance` instruction
#[cfg(not(target_arch = "bpf"))]
pub fn apply_pending_balance(
    token_account: &Pubkey,
    pending_balance_instructions: u64,
    new_decryptable_available_balance: AeCiphertext,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Vec<Instruction> {
    vec![inner_apply_pending_balance(
        token_account,
        pending_balance_instructions,
        new_decryptable_available_balance.into(),
        authority,
        multisig_signers,
    )]
}

/// Create a `EnableBalanceCredits` instruction
pub fn enable_balance_credits(
    token_account: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Vec<Instruction> {
    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    vec![encode_instruction(
        accounts,
        ConfidentialTransferInstruction::EnableBalanceCredits,
        &(),
    )]
}

/// Create a `DisableBalanceCredits` instruction
#[cfg(not(target_arch = "bpf"))]
pub fn disable_balance_credits(
    token_account: &Pubkey,
    authority: &Pubkey,
    multisig_signers: &[&Pubkey],
) -> Vec<Instruction> {
    let mut accounts = vec![
        AccountMeta::new(*token_account, false),
        AccountMeta::new_readonly(*authority, multisig_signers.is_empty()),
    ];

    for multisig_signer in multisig_signers.iter() {
        accounts.push(AccountMeta::new_readonly(**multisig_signer, true));
    }

    vec![encode_instruction(
        accounts,
        ConfidentialTransferInstruction::DisableBalanceCredits,
        &(),
    )]
}
