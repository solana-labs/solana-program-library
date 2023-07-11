#[cfg(not(target_os = "solana"))]
use solana_zk_token_sdk::encryption::{
    auth_encryption::{AeCiphertext, AeKey},
    elgamal::ElGamalSecretKey,
};
use {
    crate::{
        error::TokenError,
        extension::{Extension, ExtensionType},
        pod::*,
    },
    bytemuck::{Pod, Zeroable},
    solana_program::entrypoint::ProgramResult,
    solana_zk_token_sdk::zk_token_elgamal::pod::{
        AeCiphertext as PodAeCiphertext, ElGamalCiphertext, ElGamalPubkey,
    },
};

/// Maximum bit length of any deposit or transfer amount
///
/// Any deposit or transfer amount must be less than 2^48
pub const MAXIMUM_DEPOSIT_TRANSFER_AMOUNT: u64 = (u16::MAX as u64) + (2 << 16) * (u32::MAX as u64);

/// Bit length of the low bits of pending balance plaintext
pub const PENDING_BALANCE_LO_BIT_LENGTH: u32 = 16;

/// Confidential Transfer Extension instructions
pub mod instruction;

/// Confidential Transfer Extension processor
pub mod processor;

/// ElGamal ciphertext containing an account balance
pub type EncryptedBalance = ElGamalCiphertext;
/// Authenticated encryption containing an account balance
pub type DecryptableBalance = PodAeCiphertext;

/// Confidential transfer mint configuration
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ConfidentialTransferMint {
    /// Authority to modify the `ConfidentialTransferMint` configuration and to approve new
    /// accounts (if `auto_approve_new_accounts` is true)
    ///
    /// The legacy Token Multisig account is not supported as the authority
    pub authority: OptionalNonZeroPubkey,

    /// Indicate if newly configured accounts must be approved by the `authority` before they may be
    /// used by the user.
    ///
    /// * If `true`, no approval is required and new accounts may be used immediately
    /// * If `false`, the authority must approve newly configured accounts (see
    ///              `ConfidentialTransferInstruction::ConfigureAccount`)
    pub auto_approve_new_accounts: PodBool,

    /// Authority to decode any transfer amount in a confidential transafer.
    pub auditor_elgamal_pubkey: OptionalNonZeroElGamalPubkey,
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
    pub elgamal_pubkey: ElGamalPubkey,

    /// The low 16 bits of the pending balance (encrypted by `elgamal_pubkey`)
    pub pending_balance_lo: EncryptedBalance,

    /// The high 48 bits of the pending balance (encrypted by `elgamal_pubkey`)
    pub pending_balance_hi: EncryptedBalance,

    /// The available balance (encrypted by `encrypiton_pubkey`)
    pub available_balance: EncryptedBalance,

    /// The decryptable available balance
    pub decryptable_available_balance: DecryptableBalance,

    /// If `false`, the extended account rejects any incoming confidential transfers
    pub allow_confidential_credits: PodBool,

    /// If `false`, the base account rejects any incoming transfers
    pub allow_non_confidential_credits: PodBool,

    /// The total number of `Deposit` and `Transfer` instructions that have credited
    /// `pending_balance`
    pub pending_balance_credit_counter: PodU64,

    /// The maximum number of `Deposit` and `Transfer` instructions that can credit
    /// `pending_balance` before the `ApplyPendingBalance` instruction is executed
    pub maximum_pending_balance_credit_counter: PodU64,

    /// The `expected_pending_balance_credit_counter` value that was included in the last
    /// `ApplyPendingBalance` instruction
    pub expected_pending_balance_credit_counter: PodU64,

    /// The actual `pending_balance_credit_counter` when the last `ApplyPendingBalance` instruction
    /// was executed
    pub actual_pending_balance_credit_counter: PodU64,
}

impl Extension for ConfidentialTransferAccount {
    const TYPE: ExtensionType = ExtensionType::ConfidentialTransferAccount;
}

impl ConfidentialTransferAccount {
    /// Check if a `ConfidentialTransferAccount` has been approved for use.
    pub fn approved(&self) -> ProgramResult {
        if bool::from(&self.approved) {
            Ok(())
        } else {
            Err(TokenError::ConfidentialTransferAccountNotApproved.into())
        }
    }

    /// Check if a `ConfidentialTransferAccount` is in a closable state.
    pub fn closable(&self) -> ProgramResult {
        if self.pending_balance_lo == EncryptedBalance::zeroed()
            && self.pending_balance_hi == EncryptedBalance::zeroed()
            && self.available_balance == EncryptedBalance::zeroed()
        {
            Ok(())
        } else {
            Err(TokenError::ConfidentialTransferAccountHasBalance.into())
        }
    }

    /// Check if a base account of a `ConfidentialTransferAccount` accepts non-confidential
    /// transfers.
    pub fn non_confidential_transfer_allowed(&self) -> ProgramResult {
        if bool::from(&self.allow_non_confidential_credits) {
            Ok(())
        } else {
            Err(TokenError::NonConfidentialTransfersDisabled.into())
        }
    }

    /// Checks if a `ConfidentialTransferAccount` is configured to send funds.
    pub fn valid_as_source(&self) -> ProgramResult {
        self.approved()
    }

    /// Checks if a confidential extension is configured to receive funds.
    ///
    /// A destination account can receive funds if the following conditions are satisfied:
    ///   1. The account is approved by the confidential transfer mint authority
    ///   2. The account is not disabled by the account owner
    ///   3. The number of credits into the account has reached the maximum credit counter
    pub fn valid_as_destination(&self) -> ProgramResult {
        self.approved()?;

        if !bool::from(self.allow_confidential_credits) {
            return Err(TokenError::ConfidentialTransferDepositsAndTransfersDisabled.into());
        }

        let new_destination_pending_balance_credit_counter =
            u64::from(self.pending_balance_credit_counter)
                .checked_add(1)
                .ok_or(TokenError::Overflow)?;
        if new_destination_pending_balance_credit_counter
            > u64::from(self.maximum_pending_balance_credit_counter)
        {
            return Err(TokenError::MaximumPendingBalanceCreditCounterExceeded.into());
        }

        Ok(())
    }

    /// Increments a confidential extension pending balance credit counter.
    pub fn increment_pending_balance_credit_counter(&mut self) -> ProgramResult {
        self.pending_balance_credit_counter = (u64::from(self.pending_balance_credit_counter)
            .checked_add(1)
            .ok_or(TokenError::Overflow)?)
        .into();
        Ok(())
    }

    /// Return the account information needed to construct an `ApplyPendingBalance` instruction.
    #[cfg(not(target_os = "solana"))]
    pub fn apply_pending_balance_account_info(&self) -> ApplyPendingBalanceAccountInfo {
        let pending_balance_credit_counter = self.pending_balance_credit_counter;
        let pending_balance_lo = self.pending_balance_lo;
        let pending_balance_hi = self.pending_balance_hi;
        let decryptable_available_balance = self.decryptable_available_balance;

        ApplyPendingBalanceAccountInfo {
            pending_balance_credit_counter,
            pending_balance_lo,
            pending_balance_hi,
            decryptable_available_balance,
        }
    }
}

/// Confidential Transfer extension information needed to construct an `ApplyPendingBalance`
/// instruction.
#[cfg(not(target_os = "solana"))]
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ApplyPendingBalanceAccountInfo {
    pending_balance_credit_counter: PodU64,
    pending_balance_lo: EncryptedBalance,
    pending_balance_hi: EncryptedBalance,
    decryptable_available_balance: DecryptableBalance,
}
#[cfg(not(target_os = "solana"))]
impl ApplyPendingBalanceAccountInfo {
    /// Return the pending balance credit counter of the account.
    pub fn pending_balance_credit_counter(&self) -> u64 {
        self.pending_balance_credit_counter.into()
    }

    fn decrypted_pending_balance_lo(
        &self,
        elgamal_secret_key: &ElGamalSecretKey,
    ) -> Result<u64, TokenError> {
        let pending_balance_lo = self
            .pending_balance_lo
            .try_into()
            .map_err(|_| TokenError::AccountDecryption)?;
        elgamal_secret_key
            .decrypt_u32(&pending_balance_lo)
            .ok_or(TokenError::AccountDecryption)
    }

    fn decrypted_pending_balance_hi(
        &self,
        elgamal_secret_key: &ElGamalSecretKey,
    ) -> Result<u64, TokenError> {
        let pending_balance_hi = self
            .pending_balance_hi
            .try_into()
            .map_err(|_| TokenError::AccountDecryption)?;
        elgamal_secret_key
            .decrypt_u32(&pending_balance_hi)
            .ok_or(TokenError::AccountDecryption)
    }

    fn decrypted_available_balance(&self, aes_key: &AeKey) -> Result<u64, TokenError> {
        let decryptable_available_balance = self
            .decryptable_available_balance
            .try_into()
            .map_err(|_| TokenError::AccountDecryption)?;
        aes_key
            .decrypt(&decryptable_available_balance)
            .ok_or(TokenError::AccountDecryption)
    }

    /// Update the decryptable available balance.
    pub fn new_decryptable_available_balance(
        &self,
        elgamal_secret_key: &ElGamalSecretKey,
        aes_key: &AeKey,
    ) -> Result<AeCiphertext, TokenError> {
        let decrypted_pending_balance_lo = self.decrypted_pending_balance_lo(elgamal_secret_key)?;
        let decrypted_pending_balance_hi = self.decrypted_pending_balance_hi(elgamal_secret_key)?;
        let pending_balance =
            combine_balances(decrypted_pending_balance_lo, decrypted_pending_balance_hi)
                .ok_or(TokenError::AccountDecryption)?;
        let current_available_balance = self.decrypted_available_balance(aes_key)?;
        let new_decrypted_available_balance = current_available_balance
            .checked_add(pending_balance)
            .unwrap(); // total balance cannot exceed `u64`

        Ok(aes_key.encrypt(new_decrypted_available_balance))
    }
}

#[cfg(not(target_os = "solana"))]
fn combine_balances(balance_lo: u64, balance_hi: u64) -> Option<u64> {
    balance_hi
        .checked_shl(PENDING_BALANCE_LO_BIT_LENGTH)?
        .checked_add(balance_lo)
}
