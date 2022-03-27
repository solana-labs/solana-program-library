use {
    crate::{
        error::TokenError,
        extension::{Extension, ExtensionType},
        pod::*,
    },
    bytemuck::{Pod, Zeroable},
    solana_program::{entrypoint::ProgramResult, pubkey::Pubkey},
    solana_zk_token_sdk::zk_token_elgamal::pod,
};

/// Confidential Transfer Extension instructions
pub mod instruction;

/// Confidential Transfer Extension processor
pub mod processor;

/// ElGamal public key used for encryption
pub type EncryptionPubkey = pod::ElGamalPubkey;
/// ElGamal ciphertext containing an account balance
pub type EncryptedBalance = pod::ElGamalCiphertext;
/// Authenticated encryption containing an account balance
pub type DecryptableBalance = pod::AeCiphertext;
/// (aggregated) ElGamal ciphertext containing a transfer fee
pub type EncryptedFee = pod::FeeEncryption;
/// ElGamal ciphertext containing a withheld amount
pub type EncryptedWithheldAmount = pod::ElGamalCiphertext;

/// Confidential transfer mint configuration
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ConfidentialTransferMint {
    /// Authority to modify the `ConfidentialTransferMint` configuration and to approve new
    /// accounts (if `auto_approve_new_accounts` is true)
    ///
    /// Note that setting an authority of `Pubkey::default()` is the idiomatic way to disable
    /// future changes to the configuration.
    ///
    /// The legacy Token Multisig account is not supported as the authority
    pub authority: Pubkey,

    /// Indicate if newly configured accounts must be approved by the `authority` before they may be
    /// used by the user.
    ///
    /// * If `true`, no approval is required and new accounts may be used immediately
    /// * If `false`, the authority must approve newly configured accounts (see
    ///              `ConfidentialTransferInstruction::ConfigureAccount`)
    pub auto_approve_new_accounts: PodBool,

    /// * If non-zero, transfers must include ElGamal cypertext with this public key permitting the
    /// auditor to decode the transfer amount.
    /// * If all zero, auditing is currently disabled.
    pub auditor_pubkey: EncryptionPubkey,

    /// * If non-zero, transfers must include ElGamal cypertext of the transfer fee with this
    /// public key. If this is the case, but the base mint is not extended for fees, then any
    /// transfer will fail.
    /// * If all zero, transfer fee is disabled. If this is the case, but the base mint is extended
    /// for fees, then any transfer will fail.
    pub withdraw_withheld_authority_pubkey: EncryptionPubkey,

    /// Withheld transfer fee confidential tokens that have been moved to the mint for withdrawal.
    /// This will always be zero if fees are never enabled.
    pub withheld_amount: EncryptedWithheldAmount,
}

impl Extension for ConfidentialTransferMint {
    const TYPE: ExtensionType = ExtensionType::ConfidentialTransferMint;
}

/// Confidential account state
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ConfidentialTransferAccount {
    /// `true` if this account has been approved for use. All confidential transfer operations for
    /// the account will fail until approval is granted.
    pub approved: PodBool,

    /// The public key associated with ElGamal encryption
    pub encryption_pubkey: EncryptionPubkey,

    /// The pending balance (encrypted by `pubkey_elgamal`)
    pub pending_balance: EncryptedBalance,

    /// The available balance (encrypted by `pubkey_elgamal`)
    pub available_balance: EncryptedBalance,

    /// The decryptable available balance
    pub decryptable_available_balance: DecryptableBalance,

    /// `pending_balance` may only be credited by `Deposit` or `Transfer` instructions if `true`
    pub allow_balance_credits: PodBool,

    /// The total number of `Deposit` and `Transfer` instructions that have credited `pending_balance`
    pub pending_balance_credit_counter: PodU64,

    /// The `expected_pending_balance_credit_counter` value that was included in the last
    /// `ApplyPendingBalance` instruction
    pub expected_pending_balance_credit_counter: PodU64,

    /// The actual `pending_balance_credit_counter` when the last `ApplyPendingBalance` instruction was executed
    pub actual_pending_balance_credit_counter: PodU64,

    /// The withheld amount of fees. This will always be zero if fees are never enabled.
    pub withheld_amount: EncryptedWithheldAmount,
}

impl Extension for ConfidentialTransferAccount {
    const TYPE: ExtensionType = ExtensionType::ConfidentialTransferAccount;
}

impl ConfidentialTransferAccount {
    /// Check if a `ConfidentialTransferAccount` has been approved for use
    pub fn approved(&self) -> ProgramResult {
        if bool::from(&self.approved) {
            Ok(())
        } else {
            Err(TokenError::ConfidentialTransferAccountNotApproved.into())
        }
    }

    /// Check if a `ConfidentialTransferAccount` is in a closable state
    pub fn closable(&self) -> ProgramResult {
        if self.pending_balance == EncryptedBalance::zeroed()
            && self.available_balance == EncryptedBalance::zeroed()
            && self.withheld_amount == EncryptedWithheldAmount::zeroed()
        {
            Ok(())
        } else {
            Err(TokenError::ConfidentialTransferAccountHasBalance.into())
        }
    }
}
