//! State transition types

use {
    crate::error::SinglePoolError,
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::{
        account_info::AccountInfo, borsh::try_from_slice_unchecked, program_error::ProgramError,
        pubkey::Pubkey,
    },
};

/// Single-Validator Stake Pool account type
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum SinglePoolAccountType {
    /// Uninitialized account
    Uninitialized,
    /// Main pool account
    Pool,
}
// TODO derive default when on a rust version where its stabilized
impl Default for SinglePoolAccountType {
    fn default() -> Self {
        SinglePoolAccountType::Uninitialized
    }
}

/// Single-Validator Stake Pool account, used to derive all PDAs
#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct SinglePool {
    /// Pool account type, reserved for future compat
    pub account_type: SinglePoolAccountType,
    /// The vote account this pool is mapped to
    pub vote_account_address: Pubkey,
}
impl SinglePool {
    /// Create a SinglePool struct from its account info
    pub fn from_account_info(
        program_id: &Pubkey,
        account_info: &AccountInfo,
    ) -> Result<Self, ProgramError> {
        if account_info.data_len() == 0 || account_info.owner != program_id {
            return Err(SinglePoolError::InvalidPoolAccount.into());
        }

        let pool = try_from_slice_unchecked::<SinglePool>(&account_info.data.borrow())?;
        if pool.account_type != SinglePoolAccountType::Pool {
            return Err(SinglePoolError::InvalidPoolAccount.into());
        }

        Ok(pool)
    }
}
