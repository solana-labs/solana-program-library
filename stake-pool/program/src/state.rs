//! State transition types

use {
    crate::{error::StakePoolError, instruction::Fee, processor::Processor},
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::{account_info::AccountInfo, program_error::ProgramError, pubkey::Pubkey},
    spl_math::checked_ceil_div::CheckedCeilDiv,
    std::convert::TryFrom,
};

/// Enum representing the account type managed by the program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum AccountType {
    /// If the account has not been initialized, the enum will be 0
    Uninitialized,
    /// Stake pool
    StakePool,
    /// Validator stake list
    ValidatorStakeList,
}

impl Default for AccountType {
    fn default() -> Self {
        AccountType::Uninitialized
    }
}

/// Initialized program details.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct StakePool {
    /// Account type, must be StakePool currently
    pub account_type: AccountType,
    /// Owner authority
    /// allows for updating the staking authority
    pub owner: Pubkey,
    /// Deposit authority bump seed
    /// for `create_program_address(&[state::StakePool account, "deposit"])`
    pub deposit_bump_seed: u8,
    /// Withdrawal authority bump seed
    /// for `create_program_address(&[state::StakePool account, "withdrawal"])`
    pub withdraw_bump_seed: u8,
    /// Validator stake list storage account
    pub validator_stake_list: Pubkey,
    /// Pool Mint
    pub pool_mint: Pubkey,
    /// Owner fee account
    pub owner_fee_account: Pubkey,
    /// Pool token program id
    pub token_program_id: Pubkey,
    /// total stake under management
    pub stake_total: u64,
    /// total pool
    pub pool_total: u64,
    /// Last epoch stake_total field was updated
    pub last_update_epoch: u64,
    /// Fee applied to deposits
    pub fee: Fee,
}
impl StakePool {
    /// calculate the pool tokens that should be minted
    pub fn calc_pool_deposit_amount(&self, stake_lamports: u64) -> Option<u64> {
        if self.stake_total == 0 {
            return Some(stake_lamports);
        }
        u64::try_from(
            (stake_lamports as u128)
                .checked_mul(self.pool_total as u128)?
                .checked_div(self.stake_total as u128)?,
        )
        .ok()
    }
    /// calculate the pool tokens that should be withdrawn
    pub fn calc_pool_withdraw_amount(&self, stake_lamports: u64) -> Option<u64> {
        let (quotient, _) = (stake_lamports as u128)
            .checked_mul(self.pool_total as u128)?
            .checked_ceil_div(self.stake_total as u128)?;
        u64::try_from(quotient).ok()
    }
    /// calculate lamports amount on withdrawal
    pub fn calc_lamports_withdraw_amount(&self, pool_tokens: u64) -> Option<u64> {
        u64::try_from(
            (pool_tokens as u128)
                .checked_mul(self.stake_total as u128)?
                .checked_div(self.pool_total as u128)?,
        )
        .ok()
    }
    /// calculate the fee in pool tokens that goes to the owner
    pub fn calc_fee_amount(&self, pool_amount: u64) -> Option<u64> {
        if self.fee.denominator == 0 {
            return Some(0);
        }
        u64::try_from(
            (pool_amount as u128)
                .checked_mul(self.fee.numerator as u128)?
                .checked_div(self.fee.denominator as u128)?,
        )
        .ok()
    }

    /// Checks withdraw authority
    pub fn check_authority_withdraw(
        &self,
        authority_to_check: &Pubkey,
        program_id: &Pubkey,
        stake_pool_key: &Pubkey,
    ) -> Result<(), ProgramError> {
        Processor::check_authority(
            authority_to_check,
            program_id,
            stake_pool_key,
            Processor::AUTHORITY_WITHDRAW,
            self.withdraw_bump_seed,
        )
    }
    /// Checks deposit authority
    pub fn check_authority_deposit(
        &self,
        authority_to_check: &Pubkey,
        program_id: &Pubkey,
        stake_pool_key: &Pubkey,
    ) -> Result<(), ProgramError> {
        Processor::check_authority(
            authority_to_check,
            program_id,
            stake_pool_key,
            Processor::AUTHORITY_DEPOSIT,
            self.deposit_bump_seed,
        )
    }

    /// Check owner validity and signature
    pub fn check_owner(&self, owner_info: &AccountInfo) -> Result<(), ProgramError> {
        if *owner_info.key != self.owner {
            return Err(StakePoolError::WrongOwner.into());
        }
        if !owner_info.is_signer {
            return Err(StakePoolError::SignatureMissing.into());
        }
        Ok(())
    }

    /// Check if StakePool is actually initialized as a stake pool
    pub fn is_valid(&self) -> bool {
        self.account_type == AccountType::StakePool
    }

    /// Check if StakePool is currently uninitialized
    pub fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::Uninitialized
    }
}

/// Storage list for all validator stake accounts in the pool.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct ValidatorStakeList {
    /// Account type, must be ValidatorStakeList currently
    pub account_type: AccountType,

    /// Maximum allowable number of validators
    pub max_validators: u32,

    /// List of all validator stake accounts and their info
    pub validators: Vec<ValidatorStakeInfo>,
}

/// Information about the singe validator stake account
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct ValidatorStakeInfo {
    /// Validator account pubkey
    pub validator_account: Pubkey,

    /// Account balance in lamports
    pub balance: u64,

    /// Last epoch balance field was updated
    pub last_update_epoch: u64,
}

impl ValidatorStakeList {
    /// Create an empty instance containing space for `max_validators`
    pub fn new_with_max_validators(max_validators: u32) -> Self {
        Self {
            account_type: AccountType::ValidatorStakeList,
            max_validators,
            validators: vec![ValidatorStakeInfo::default(); max_validators as usize],
        }
    }

    /// Calculate the number of validator entries that fit in the provided length
    pub fn calculate_max_validators(buffer_length: usize) -> usize {
        (buffer_length - 1 - 4 - 4) / 48
    }

    /// Check if contains validator with particular pubkey
    pub fn contains(&self, validator: &Pubkey) -> bool {
        self.validators
            .iter()
            .any(|x| x.validator_account == *validator)
    }

    /// Check if contains validator with particular pubkey (mutable)
    pub fn find_mut(&mut self, validator: &Pubkey) -> Option<&mut ValidatorStakeInfo> {
        self.validators
            .iter_mut()
            .find(|x| x.validator_account == *validator)
    }
    /// Check if contains validator with particular pubkey (immutable)
    pub fn find(&self, validator: &Pubkey) -> Option<&ValidatorStakeInfo> {
        self.validators
            .iter()
            .find(|x| x.validator_account == *validator)
    }

    /// Check if validator stake list is actually initialized as a validator stake list
    pub fn is_valid(&self) -> bool {
        self.account_type == AccountType::ValidatorStakeList
    }

    /// Check if the validator stake list is uninitialized
    pub fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::Uninitialized
    }
}

#[cfg(test)]
mod test {
    use {
        super::*,
        crate::borsh::{get_instance_packed_len, try_from_slice_unchecked},
        proptest::prelude::*,
        solana_program::borsh::get_packed_len,
    };

    #[test]
    fn test_state_packing() {
        let max_validators = 10_000;
        let size =
            get_instance_packed_len(&ValidatorStakeList::new_with_max_validators(max_validators))
                .unwrap();
        // Not initialized
        let stake_list = ValidatorStakeList {
            account_type: AccountType::Uninitialized,
            max_validators: 0,
            validators: vec![],
        };
        let mut byte_vec = vec![0u8; size];
        let mut bytes = byte_vec.as_mut_slice();
        stake_list.serialize(&mut bytes).unwrap();
        let stake_list_unpacked =
            try_from_slice_unchecked::<ValidatorStakeList>(&byte_vec).unwrap();
        assert_eq!(stake_list_unpacked, stake_list);

        // Empty
        let stake_list = ValidatorStakeList {
            account_type: AccountType::ValidatorStakeList,
            max_validators: 0,
            validators: vec![],
        };
        let mut byte_vec = vec![0u8; size];
        let mut bytes = byte_vec.as_mut_slice();
        stake_list.serialize(&mut bytes).unwrap();
        let stake_list_unpacked =
            try_from_slice_unchecked::<ValidatorStakeList>(&byte_vec).unwrap();
        assert_eq!(stake_list_unpacked, stake_list);

        // With several accounts
        let stake_list = ValidatorStakeList {
            account_type: AccountType::ValidatorStakeList,
            max_validators,
            validators: vec![
                ValidatorStakeInfo {
                    validator_account: Pubkey::new_from_array([1; 32]),
                    balance: 123456789,
                    last_update_epoch: 987654321,
                },
                ValidatorStakeInfo {
                    validator_account: Pubkey::new_from_array([2; 32]),
                    balance: 998877665544,
                    last_update_epoch: 11223445566,
                },
                ValidatorStakeInfo {
                    validator_account: Pubkey::new_from_array([3; 32]),
                    balance: 0,
                    last_update_epoch: 999999999999999,
                },
            ],
        };
        let mut byte_vec = vec![0u8; size];
        let mut bytes = byte_vec.as_mut_slice();
        stake_list.serialize(&mut bytes).unwrap();
        let stake_list_unpacked =
            try_from_slice_unchecked::<ValidatorStakeList>(&byte_vec).unwrap();
        assert_eq!(stake_list_unpacked, stake_list);
    }

    proptest! {
        #[test]
        fn stake_list_size_calculation(test_amount in 0..=100_000_u32) {
            let validators = ValidatorStakeList::new_with_max_validators(test_amount);
            let size = get_instance_packed_len(&validators).unwrap();
            assert_eq!(ValidatorStakeList::calculate_max_validators(size), test_amount as usize);
            assert_eq!(ValidatorStakeList::calculate_max_validators(size + 1), test_amount as usize);
            assert_eq!(ValidatorStakeList::calculate_max_validators(size + get_packed_len::<ValidatorStakeInfo>()), (test_amount + 1)as usize);
            assert_eq!(ValidatorStakeList::calculate_max_validators(size - 1), (test_amount - 1) as usize);
        }
    }
}
