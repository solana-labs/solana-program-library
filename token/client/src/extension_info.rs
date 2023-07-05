use {
    crate::token::TokenError,
    bytemuck::{Pod, Zeroable},
    spl_token_2022::{
        extension::confidential_transfer::{
            ConfidentialTransferAccount, PENDING_BALANCE_LO_BIT_LENGTH,
        },
        pod::PodU64,
        solana_zk_token_sdk::{
            encryption::{
                auth_encryption::{AeCiphertext, AeKey},
                elgamal::ElGamalSecretKey,
            },
            zk_token_elgamal::pod,
        },
    },
};
#[cfg(feature = "proof-program")]
use {
    solana_sdk::epoch_info::EpochInfo,
    spl_token_2022::solana_zk_token_sdk::{
        encryption::{auth_encryption::*, elgamal::*},
        instruction::transfer_with_fee::FeeParameters,
    },
    std::convert::TryInto,
};

/// Confidential Transfer extension information needed to construct an `ApplyPendingBalance`
/// instruction.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, Pod, Zeroable)]
pub struct ApplyPendingBalanceExtensionInfo {
    pending_balance_credit_counter: PodU64,
    pending_balance_lo: pod::ElGamalCiphertext,
    pending_balance_hi: pod::ElGamalCiphertext,
    decryptable_available_balance: pod::AeCiphertext,
}
impl ApplyPendingBalanceExtensionInfo {
    pub fn new(confidential_transfer_account: &ConfidentialTransferAccount) -> Self {
        let pending_balance_credit_counter =
            confidential_transfer_account.pending_balance_credit_counter;
        let pending_balance_lo = confidential_transfer_account.pending_balance_lo;
        let pending_balance_hi = confidential_transfer_account.pending_balance_hi;
        let decryptable_available_balance =
            confidential_transfer_account.decryptable_available_balance;

        Self {
            pending_balance_credit_counter,
            pending_balance_lo,
            pending_balance_hi,
            decryptable_available_balance,
        }
    }

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

fn combine_balances(balance_lo: u64, balance_hi: u64) -> Option<u64> {
    balance_hi
        .checked_shl(PENDING_BALANCE_LO_BIT_LENGTH)?
        .checked_add(balance_lo)
}
