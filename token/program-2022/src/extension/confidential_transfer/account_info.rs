use {
    crate::{
        error::TokenError,
        extension::confidential_transfer::{
            ciphertext_extraction::SourceDecryptHandles,
            split_proof_generation::transfer_split_proof_data, ConfidentialTransferAccount,
            DecryptableBalance, EncryptedBalance, PENDING_BALANCE_LO_BIT_LENGTH,
        },
    },
    bytemuck::{Pod, Zeroable},
    solana_zk_token_sdk::{
        encryption::{
            auth_encryption::{AeCiphertext, AeKey},
            elgamal::{ElGamalKeypair, ElGamalPubkey, ElGamalSecretKey},
        },
        instruction::{
            transfer::{FeeParameters, TransferData, TransferWithFeeData},
            withdraw::WithdrawData,
            zero_balance::ZeroBalanceProofData,
            BatchedGroupedCiphertext2HandlesValidityProofData, BatchedRangeProofU128Data,
            CiphertextCommitmentEqualityProofData,
        },
    },
    spl_pod::primitives::PodU64,
};

/// Confidential transfer extension information needed to construct an
/// `EmptyAccount` instruction.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct EmptyAccountAccountInfo {
    /// The available balance
    pub(crate) available_balance: EncryptedBalance,
}
impl EmptyAccountAccountInfo {
    /// Create the `EmptyAccount` instruction account information from
    /// `ConfidentialTransferAccount`.
    pub fn new(account: &ConfidentialTransferAccount) -> Self {
        Self {
            available_balance: account.available_balance,
        }
    }

    /// Create an empty account proof data.
    pub fn generate_proof_data(
        &self,
        elgamal_keypair: &ElGamalKeypair,
    ) -> Result<ZeroBalanceProofData, TokenError> {
        let available_balance = self
            .available_balance
            .try_into()
            .map_err(|_| TokenError::MalformedCiphertext)?;

        ZeroBalanceProofData::new(elgamal_keypair, &available_balance)
            .map_err(|_| TokenError::ProofGeneration)
    }
}

/// Confidential Transfer extension information needed to construct an
/// `ApplyPendingBalance` instruction.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ApplyPendingBalanceAccountInfo {
    /// The total number of `Deposit` and `Transfer` instructions that have
    /// credited `pending_balance`
    pub(crate) pending_balance_credit_counter: PodU64,
    /// The low 16 bits of the pending balance (encrypted by `elgamal_pubkey`)
    pub(crate) pending_balance_lo: EncryptedBalance,
    /// The high 48 bits of the pending balance (encrypted by `elgamal_pubkey`)
    pub(crate) pending_balance_hi: EncryptedBalance,
    /// The decryptable available balance
    pub(crate) decryptable_available_balance: DecryptableBalance,
}
impl ApplyPendingBalanceAccountInfo {
    /// Create the `ApplyPendingBalance` instruction account information from
    /// `ConfidentialTransferAccount`.
    pub fn new(account: &ConfidentialTransferAccount) -> Self {
        Self {
            pending_balance_credit_counter: account.pending_balance_credit_counter,
            pending_balance_lo: account.pending_balance_lo,
            pending_balance_hi: account.pending_balance_hi,
            decryptable_available_balance: account.decryptable_available_balance,
        }
    }

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
            .map_err(|_| TokenError::MalformedCiphertext)?;
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
            .map_err(|_| TokenError::MalformedCiphertext)?;
        elgamal_secret_key
            .decrypt_u32(&pending_balance_hi)
            .ok_or(TokenError::AccountDecryption)
    }

    fn decrypted_available_balance(&self, aes_key: &AeKey) -> Result<u64, TokenError> {
        let decryptable_available_balance = self
            .decryptable_available_balance
            .try_into()
            .map_err(|_| TokenError::MalformedCiphertext)?;
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

/// Confidential Transfer extension information needed to construct a `Withdraw`
/// instruction.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct WithdrawAccountInfo {
    /// The available balance (encrypted by `encrypiton_pubkey`)
    pub available_balance: EncryptedBalance,
    /// The decryptable available balance
    pub decryptable_available_balance: DecryptableBalance,
}
impl WithdrawAccountInfo {
    /// Create the `ApplyPendingBalance` instruction account information from
    /// `ConfidentialTransferAccount`.
    pub fn new(account: &ConfidentialTransferAccount) -> Self {
        Self {
            available_balance: account.available_balance,
            decryptable_available_balance: account.decryptable_available_balance,
        }
    }

    fn decrypted_available_balance(&self, aes_key: &AeKey) -> Result<u64, TokenError> {
        let decryptable_available_balance = self
            .decryptable_available_balance
            .try_into()
            .map_err(|_| TokenError::MalformedCiphertext)?;
        aes_key
            .decrypt(&decryptable_available_balance)
            .ok_or(TokenError::AccountDecryption)
    }

    /// Create a withdraw proof data.
    pub fn generate_proof_data(
        &self,
        withdraw_amount: u64,
        elgamal_keypair: &ElGamalKeypair,
        aes_key: &AeKey,
    ) -> Result<WithdrawData, TokenError> {
        let current_available_balance = self
            .available_balance
            .try_into()
            .map_err(|_| TokenError::MalformedCiphertext)?;
        let current_decrypted_available_balance = self.decrypted_available_balance(aes_key)?;

        WithdrawData::new(
            withdraw_amount,
            elgamal_keypair,
            current_decrypted_available_balance,
            &current_available_balance,
        )
        .map_err(|_| TokenError::ProofGeneration)
    }

    /// Update the decryptable available balance.
    pub fn new_decryptable_available_balance(
        &self,
        withdraw_amount: u64,
        aes_key: &AeKey,
    ) -> Result<AeCiphertext, TokenError> {
        let current_decrypted_available_balance = self.decrypted_available_balance(aes_key)?;
        let new_decrypted_available_balance = current_decrypted_available_balance
            .checked_sub(withdraw_amount)
            .ok_or(TokenError::InsufficientFunds)?;

        Ok(aes_key.encrypt(new_decrypted_available_balance))
    }
}

/// Confidential Transfer extension information needed to construct a `Transfer`
/// instruction.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct TransferAccountInfo {
    /// The available balance (encrypted by `encrypiton_pubkey`)
    pub available_balance: EncryptedBalance,
    /// The decryptable available balance
    pub decryptable_available_balance: DecryptableBalance,
}
impl TransferAccountInfo {
    /// Create the `Transfer` instruction account information from
    /// `ConfidentialTransferAccount`.
    pub fn new(account: &ConfidentialTransferAccount) -> Self {
        Self {
            available_balance: account.available_balance,
            decryptable_available_balance: account.decryptable_available_balance,
        }
    }

    fn decrypted_available_balance(&self, aes_key: &AeKey) -> Result<u64, TokenError> {
        let decryptable_available_balance = self
            .decryptable_available_balance
            .try_into()
            .map_err(|_| TokenError::MalformedCiphertext)?;
        aes_key
            .decrypt(&decryptable_available_balance)
            .ok_or(TokenError::AccountDecryption)
    }

    /// Create a transfer proof data.
    pub fn generate_transfer_proof_data(
        &self,
        transfer_amount: u64,
        elgamal_keypair: &ElGamalKeypair,
        aes_key: &AeKey,
        destination_elgamal_pubkey: &ElGamalPubkey,
        auditor_elgamal_pubkey: Option<&ElGamalPubkey>,
    ) -> Result<TransferData, TokenError> {
        let current_source_available_balance = self
            .available_balance
            .try_into()
            .map_err(|_| TokenError::MalformedCiphertext)?;
        let current_source_decrypted_available_balance =
            self.decrypted_available_balance(aes_key)?;

        let default_auditor_pubkey = ElGamalPubkey::default();
        let auditor_elgamal_pubkey = auditor_elgamal_pubkey.unwrap_or(&default_auditor_pubkey);

        TransferData::new(
            transfer_amount,
            (
                current_source_decrypted_available_balance,
                &current_source_available_balance,
            ),
            elgamal_keypair,
            (destination_elgamal_pubkey, auditor_elgamal_pubkey),
        )
        .map_err(|_| TokenError::ProofGeneration)
    }

    /// Create a transfer proof data that is split into equality, ciphertext
    /// validity, and range proofs.
    pub fn generate_split_transfer_proof_data(
        &self,
        transfer_amount: u64,
        source_elgamal_keypair: &ElGamalKeypair,
        aes_key: &AeKey,
        destination_elgamal_pubkey: &ElGamalPubkey,
        auditor_elgamal_pubkey: Option<&ElGamalPubkey>,
    ) -> Result<
        (
            CiphertextCommitmentEqualityProofData,
            BatchedGroupedCiphertext2HandlesValidityProofData,
            BatchedRangeProofU128Data,
            SourceDecryptHandles,
        ),
        TokenError,
    > {
        let current_available_balance = self
            .available_balance
            .try_into()
            .map_err(|_| TokenError::MalformedCiphertext)?;
        let current_decryptable_available_balance = self
            .decryptable_available_balance
            .try_into()
            .map_err(|_| TokenError::MalformedCiphertext)?;

        transfer_split_proof_data(
            &current_available_balance,
            &current_decryptable_available_balance,
            transfer_amount,
            source_elgamal_keypair,
            aes_key,
            destination_elgamal_pubkey,
            auditor_elgamal_pubkey,
        )
    }

    /// Create a transfer with fee proof data
    #[allow(clippy::too_many_arguments)]
    pub fn generate_transfer_with_fee_proof_data(
        &self,
        transfer_amount: u64,
        elgamal_keypair: &ElGamalKeypair,
        aes_key: &AeKey,
        destination_elgamal_pubkey: &ElGamalPubkey,
        auditor_elgamal_pubkey: Option<&ElGamalPubkey>,
        withdraw_withheld_authority_elgamal_pubkey: &ElGamalPubkey,
        fee_rate_basis_points: u16,
        maximum_fee: u64,
    ) -> Result<TransferWithFeeData, TokenError> {
        let current_source_available_balance = self
            .available_balance
            .try_into()
            .map_err(|_| TokenError::MalformedCiphertext)?;
        let current_source_decrypted_available_balance =
            self.decrypted_available_balance(aes_key)?;

        let default_auditor_pubkey = ElGamalPubkey::default();
        let auditor_elgamal_pubkey = auditor_elgamal_pubkey.unwrap_or(&default_auditor_pubkey);

        let fee_parameters = FeeParameters {
            fee_rate_basis_points,
            maximum_fee,
        };

        TransferWithFeeData::new(
            transfer_amount,
            (
                current_source_decrypted_available_balance,
                &current_source_available_balance,
            ),
            elgamal_keypair,
            (destination_elgamal_pubkey, auditor_elgamal_pubkey),
            fee_parameters,
            withdraw_withheld_authority_elgamal_pubkey,
        )
        .map_err(|_| TokenError::ProofGeneration)
    }

    /// Update the decryptable available balance.
    pub fn new_decryptable_available_balance(
        &self,
        transfer_amount: u64,
        aes_key: &AeKey,
    ) -> Result<AeCiphertext, TokenError> {
        let current_decrypted_available_balance = self.decrypted_available_balance(aes_key)?;
        let new_decrypted_available_balance = current_decrypted_available_balance
            .checked_sub(transfer_amount)
            .ok_or(TokenError::InsufficientFunds)?;

        Ok(aes_key.encrypt(new_decrypted_available_balance))
    }
}

fn combine_balances(balance_lo: u64, balance_hi: u64) -> Option<u64> {
    balance_hi
        .checked_shl(PENDING_BALANCE_LO_BIT_LENGTH)?
        .checked_add(balance_lo)
}
