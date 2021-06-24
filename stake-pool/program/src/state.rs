//! State transition types

use {
    crate::error::StakePoolError,
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::{account_info::AccountInfo, msg, program_error::ProgramError, pubkey::Pubkey},
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
    ValidatorList,
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

    /// Manager authority, allows for updating the staker, manager, and fee account
    pub manager: Pubkey,

    /// Staker authority, allows for adding and removing validators, and managing stake
    /// distribution
    pub staker: Pubkey,

    /// Deposit authority
    ///
    /// If a depositor pubkey is specified on initialization, then deposits must be
    /// signed by this authority. If no deposit authority is specified,
    /// then the stake pool will default to the result of:
    /// `Pubkey::find_program_address(
    ///     &[&stake_pool_address.to_bytes()[..32], b"deposit"],
    ///     program_id,
    /// )`
    pub deposit_authority: Pubkey,

    /// Withdrawal authority bump seed
    /// for `create_program_address(&[state::StakePool account, "withdrawal"])`
    pub withdraw_bump_seed: u8,

    /// Validator stake list storage account
    pub validator_list: Pubkey,

    /// Reserve stake account, holds deactivated stake
    pub reserve_stake: Pubkey,

    /// Pool Mint
    pub pool_mint: Pubkey,

    /// Manager fee account
    pub manager_fee_account: Pubkey,

    /// Pool token program id
    pub token_program_id: Pubkey,

    /// Total stake under management.
    /// Note that if `last_update_epoch` does not match the current epoch then
    /// this field may not be accurate
    pub total_stake_lamports: u64,

    /// Total supply of pool tokens (should always match the supply in the Pool Mint)
    pub pool_token_supply: u64,

    /// Last epoch the `total_stake_lamports` field was updated
    pub last_update_epoch: u64,

    /// Fee taken as a proportion of rewards each epoch
    pub fee: Fee,

    /// Fee for next epoch
    pub next_epoch_fee: Option<Fee>,
}
impl StakePool {
    /// calculate the pool tokens that should be minted for a deposit of `stake_lamports`
    pub fn calc_pool_tokens_for_deposit(&self, stake_lamports: u64) -> Option<u64> {
        if self.total_stake_lamports == 0 || self.pool_token_supply == 0 {
            return Some(stake_lamports);
        }
        u64::try_from(
            (stake_lamports as u128)
                .checked_mul(self.pool_token_supply as u128)?
                .checked_div(self.total_stake_lamports as u128)?,
        )
        .ok()
    }

    /// calculate lamports amount on withdrawal
    pub fn calc_lamports_withdraw_amount(&self, pool_tokens: u64) -> Option<u64> {
        u64::try_from(
            (pool_tokens as u128)
                .checked_mul(self.total_stake_lamports as u128)?
                .checked_div(self.pool_token_supply as u128)?,
        )
        .ok()
    }

    /// Calculate the fee in pool tokens that goes to the manager
    ///
    /// This function assumes that `reward_lamports` has not already been added
    /// to the stake pool's `total_stake_lamports`
    pub fn calc_fee_amount(&self, reward_lamports: u64) -> Option<u64> {
        if self.fee.denominator == 0 || reward_lamports == 0 {
            return Some(0);
        }
        let total_stake_lamports =
            (self.total_stake_lamports as u128).checked_add(reward_lamports as u128)?;
        let fee_lamports = (reward_lamports as u128)
            .checked_mul(self.fee.numerator as u128)?
            .checked_div(self.fee.denominator as u128)?;
        u64::try_from(
            (self.pool_token_supply as u128)
                .checked_mul(fee_lamports)?
                .checked_div(total_stake_lamports.checked_sub(fee_lamports)?)?,
        )
        .ok()
    }

    /// Checks that the withdraw or deposit authority is valid
    fn check_authority(
        authority_address: &Pubkey,
        program_id: &Pubkey,
        stake_pool_address: &Pubkey,
        authority_seed: &[u8],
        bump_seed: u8,
    ) -> Result<(), ProgramError> {
        let expected_address = Pubkey::create_program_address(
            &[
                &stake_pool_address.to_bytes()[..32],
                authority_seed,
                &[bump_seed],
            ],
            program_id,
        )?;

        if *authority_address == expected_address {
            Ok(())
        } else {
            msg!(
                "Incorrect authority provided, expected {}, received {}",
                expected_address,
                authority_address
            );
            Err(StakePoolError::InvalidProgramAddress.into())
        }
    }

    /// Checks that the withdraw authority is valid
    pub(crate) fn check_authority_withdraw(
        &self,
        withdraw_authority: &Pubkey,
        program_id: &Pubkey,
        stake_pool_address: &Pubkey,
    ) -> Result<(), ProgramError> {
        Self::check_authority(
            withdraw_authority,
            program_id,
            stake_pool_address,
            crate::AUTHORITY_WITHDRAW,
            self.withdraw_bump_seed,
        )
    }
    /// Checks that the deposit authority is valid
    pub(crate) fn check_deposit_authority(
        &self,
        deposit_authority: &Pubkey,
    ) -> Result<(), ProgramError> {
        if self.deposit_authority == *deposit_authority {
            Ok(())
        } else {
            Err(StakePoolError::InvalidProgramAddress.into())
        }
    }

    /// Check staker validity and signature
    pub(crate) fn check_mint(&self, mint_info: &AccountInfo) -> Result<(), ProgramError> {
        if *mint_info.key != self.pool_mint {
            Err(StakePoolError::WrongPoolMint.into())
        } else {
            Ok(())
        }
    }

    /// Check manager validity and signature
    pub(crate) fn check_manager(&self, manager_info: &AccountInfo) -> Result<(), ProgramError> {
        if *manager_info.key != self.manager {
            msg!(
                "Incorrect manager provided, expected {}, received {}",
                self.manager,
                manager_info.key
            );
            return Err(StakePoolError::WrongManager.into());
        }
        if !manager_info.is_signer {
            msg!("Manager signature missing");
            return Err(StakePoolError::SignatureMissing.into());
        }
        Ok(())
    }

    /// Check staker validity and signature
    pub(crate) fn check_staker(&self, staker_info: &AccountInfo) -> Result<(), ProgramError> {
        if *staker_info.key != self.staker {
            msg!(
                "Incorrect staker provided, expected {}, received {}",
                self.staker,
                staker_info.key
            );
            return Err(StakePoolError::WrongStaker.into());
        }
        if !staker_info.is_signer {
            msg!("Staker signature missing");
            return Err(StakePoolError::SignatureMissing.into());
        }
        Ok(())
    }

    /// Check the validator list is valid
    pub fn check_validator_list(
        &self,
        validator_list_info: &AccountInfo,
    ) -> Result<(), ProgramError> {
        if *validator_list_info.key != self.validator_list {
            msg!(
                "Invalid validator list provided, expected {}, received {}",
                self.validator_list,
                validator_list_info.key
            );
            Err(StakePoolError::InvalidValidatorStakeList.into())
        } else {
            Ok(())
        }
    }

    /// Check the validator list is valid
    pub fn check_reserve_stake(
        &self,
        reserve_stake_info: &AccountInfo,
    ) -> Result<(), ProgramError> {
        if *reserve_stake_info.key != self.reserve_stake {
            msg!(
                "Invalid reserve stake provided, expected {}, received {}",
                self.reserve_stake,
                reserve_stake_info.key
            );
            Err(StakePoolError::InvalidProgramAddress.into())
        } else {
            Ok(())
        }
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
pub struct ValidatorList {
    /// Account type, must be ValidatorList currently
    pub account_type: AccountType,

    /// Preferred deposit validator vote account pubkey
    pub preferred_deposit_validator_vote_address: Option<Pubkey>,

    /// Preferred withdraw validator vote account pubkey
    pub preferred_withdraw_validator_vote_address: Option<Pubkey>,

    /// Maximum allowable number of validators
    pub max_validators: u32,

    /// List of stake info for each validator in the pool
    pub validators: Vec<ValidatorStakeInfo>,
}

/// Status of the stake account in the validator list, for accounting
#[derive(Copy, Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum StakeStatus {
    /// Stake account is active, there may be a transient stake as well
    Active,
    /// Only transient stake account exists, when a transient stake is
    /// deactivating during validator removal
    DeactivatingTransient,
    /// No more validator stake accounts exist, entry ready for removal during
    /// `UpdateStakePoolBalance`
    ReadyForRemoval,
}

impl Default for StakeStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Information about the singe validator stake account
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct ValidatorStakeInfo {
    /// Status of the validator stake account
    pub status: StakeStatus,

    /// Validator vote account address
    pub vote_account_address: Pubkey,

    /// Amount of active stake delegated to this validator
    /// Note that if `last_update_epoch` does not match the current epoch then
    /// this field may not be accurate
    pub active_stake_lamports: u64,

    /// Amount of transient stake delegated to this validator
    /// Note that if `last_update_epoch` does not match the current epoch then
    /// this field may not be accurate
    pub transient_stake_lamports: u64,

    /// Last epoch the active and transient stake lamports fields were updated
    pub last_update_epoch: u64,
}

impl ValidatorStakeInfo {
    /// Get the total lamports delegated to this validator (active and transient)
    pub fn stake_lamports(&self) -> u64 {
        self.active_stake_lamports
            .checked_add(self.transient_stake_lamports)
            .unwrap()
    }
}

impl ValidatorList {
    /// Create an empty instance containing space for `max_validators` and preferred validator keys
    pub fn new(max_validators: u32) -> Self {
        Self {
            account_type: AccountType::ValidatorList,
            preferred_deposit_validator_vote_address: Some(Pubkey::default()),
            preferred_withdraw_validator_vote_address: Some(Pubkey::default()),
            max_validators,
            validators: vec![ValidatorStakeInfo::default(); max_validators as usize],
        }
    }

    /// Calculate the number of validator entries that fit in the provided length
    pub fn calculate_max_validators(buffer_length: usize) -> usize {
        let header_size = 1 + 4 + 4 + 33 + 33;
        buffer_length.saturating_sub(header_size) / 57
    }

    /// Check if contains validator with particular pubkey
    pub fn contains(&self, vote_account_address: &Pubkey) -> bool {
        self.validators
            .iter()
            .any(|x| x.vote_account_address == *vote_account_address)
    }

    /// Check if contains validator with particular pubkey
    pub fn find_mut(&mut self, vote_account_address: &Pubkey) -> Option<&mut ValidatorStakeInfo> {
        self.validators
            .iter_mut()
            .find(|x| x.vote_account_address == *vote_account_address)
    }
    /// Check if contains validator with particular pubkey
    pub fn find(&self, vote_account_address: &Pubkey) -> Option<&ValidatorStakeInfo> {
        self.validators
            .iter()
            .find(|x| x.vote_account_address == *vote_account_address)
    }

    /// Check if validator stake list is actually initialized as a validator stake list
    pub fn is_valid(&self) -> bool {
        self.account_type == AccountType::ValidatorList
    }

    /// Check if the validator stake list is uninitialized
    pub fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::Uninitialized
    }

    /// Check if the list has any active stake
    pub fn has_active_stake(&self) -> bool {
        self.validators.iter().any(|x| x.active_stake_lamports > 0)
    }
}

/// Fee rate as a ratio, minted on `UpdateStakePoolBalance` as a proportion of
/// the rewards
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct Fee {
    /// denominator of the fee ratio
    pub denominator: u64,
    /// numerator of the fee ratio
    pub numerator: u64,
}

#[cfg(test)]
mod test {
    use {
        super::*,
        proptest::prelude::*,
        solana_program::borsh::{
            get_instance_packed_len, get_packed_len, try_from_slice_unchecked,
        },
        solana_program::native_token::LAMPORTS_PER_SOL,
    };

    fn uninitialized_validator_list() -> ValidatorList {
        ValidatorList {
            account_type: AccountType::Uninitialized,
            preferred_deposit_validator_vote_address: None,
            preferred_withdraw_validator_vote_address: None,
            max_validators: 0,
            validators: vec![],
        }
    }

    fn test_validator_list(max_validators: u32) -> ValidatorList {
        ValidatorList {
            account_type: AccountType::ValidatorList,
            preferred_deposit_validator_vote_address: Some(Pubkey::new_unique()),
            preferred_withdraw_validator_vote_address: Some(Pubkey::new_unique()),
            max_validators,
            validators: vec![
                ValidatorStakeInfo {
                    status: StakeStatus::Active,
                    vote_account_address: Pubkey::new_from_array([1; 32]),
                    active_stake_lamports: 123456789,
                    transient_stake_lamports: 1111111,
                    last_update_epoch: 987654321,
                },
                ValidatorStakeInfo {
                    status: StakeStatus::DeactivatingTransient,
                    vote_account_address: Pubkey::new_from_array([2; 32]),
                    active_stake_lamports: 998877665544,
                    transient_stake_lamports: 222222222,
                    last_update_epoch: 11223445566,
                },
                ValidatorStakeInfo {
                    status: StakeStatus::ReadyForRemoval,
                    vote_account_address: Pubkey::new_from_array([3; 32]),
                    active_stake_lamports: 0,
                    transient_stake_lamports: 0,
                    last_update_epoch: 999999999999999,
                },
            ],
        }
    }

    #[test]
    fn state_packing() {
        let max_validators = 10_000;
        let size = get_instance_packed_len(&ValidatorList::new(max_validators)).unwrap();
        let stake_list = uninitialized_validator_list();
        let mut byte_vec = vec![0u8; size];
        let mut bytes = byte_vec.as_mut_slice();
        stake_list.serialize(&mut bytes).unwrap();
        let stake_list_unpacked = try_from_slice_unchecked::<ValidatorList>(&byte_vec).unwrap();
        assert_eq!(stake_list_unpacked, stake_list);

        // Empty, one preferred key
        let stake_list = ValidatorList {
            account_type: AccountType::ValidatorList,
            preferred_deposit_validator_vote_address: Some(Pubkey::new_unique()),
            preferred_withdraw_validator_vote_address: None,
            max_validators: 0,
            validators: vec![],
        };
        let mut byte_vec = vec![0u8; size];
        let mut bytes = byte_vec.as_mut_slice();
        stake_list.serialize(&mut bytes).unwrap();
        let stake_list_unpacked = try_from_slice_unchecked::<ValidatorList>(&byte_vec).unwrap();
        assert_eq!(stake_list_unpacked, stake_list);

        // With several accounts
        let stake_list = test_validator_list(max_validators);
        let mut byte_vec = vec![0u8; size];
        let mut bytes = byte_vec.as_mut_slice();
        stake_list.serialize(&mut bytes).unwrap();
        let stake_list_unpacked = try_from_slice_unchecked::<ValidatorList>(&byte_vec).unwrap();
        assert_eq!(stake_list_unpacked, stake_list);
    }

    #[test]
    fn validator_list_active_stake() {
        let max_validators = 10_000;
        let mut validator_list = test_validator_list(max_validators);
        assert!(validator_list.has_active_stake());
        for validator in validator_list.validators.iter_mut() {
            validator.active_stake_lamports = 0;
        }
        assert!(!validator_list.has_active_stake());
    }

    proptest! {
        #[test]
        fn stake_list_size_calculation(test_amount in 0..=100_000_u32) {
            let validators = ValidatorList::new(test_amount);
            let size = get_instance_packed_len(&validators).unwrap();
            assert_eq!(ValidatorList::calculate_max_validators(size), test_amount as usize);
            assert_eq!(ValidatorList::calculate_max_validators(size.saturating_add(1)), test_amount as usize);
            assert_eq!(ValidatorList::calculate_max_validators(size.saturating_add(get_packed_len::<ValidatorStakeInfo>())), (test_amount + 1)as usize);
            assert_eq!(ValidatorList::calculate_max_validators(size.saturating_sub(1)), (test_amount.saturating_sub(1)) as usize);
        }
    }

    prop_compose! {
        fn fee()(denominator in 1..=u16::MAX)(
            denominator in Just(denominator),
            numerator in 0..=denominator,
        ) -> (u64, u64) {
            (numerator as u64, denominator as u64)
        }
    }

    prop_compose! {
        fn total_stake_and_rewards()(total_stake_lamports in 1..u64::MAX)(
            total_stake_lamports in Just(total_stake_lamports),
            rewards in 0..=total_stake_lamports,
        ) -> (u64, u64) {
            (total_stake_lamports - rewards, rewards)
        }
    }

    #[test]
    fn specific_fee_calculation() {
        // 10% of 10 SOL in rewards should be 1 SOL in fees
        let fee = Fee {
            numerator: 1,
            denominator: 10,
        };
        let mut stake_pool = StakePool {
            total_stake_lamports: 100 * LAMPORTS_PER_SOL,
            pool_token_supply: 100 * LAMPORTS_PER_SOL,
            fee,
            ..StakePool::default()
        };
        let reward_lamports = 10 * LAMPORTS_PER_SOL;
        let pool_token_fee = stake_pool.calc_fee_amount(reward_lamports).unwrap();

        stake_pool.total_stake_lamports += reward_lamports;
        stake_pool.pool_token_supply += pool_token_fee;

        let fee_lamports = stake_pool
            .calc_lamports_withdraw_amount(pool_token_fee)
            .unwrap();
        assert_eq!(fee_lamports, LAMPORTS_PER_SOL - 1); // lose 1 lamport of precision
    }

    proptest! {
        #[test]
        fn fee_calculation(
            (numerator, denominator) in fee(),
            (total_stake_lamports, reward_lamports) in total_stake_and_rewards(),
        ) {
            let fee = Fee { denominator, numerator };
            let mut stake_pool = StakePool {
                total_stake_lamports,
                pool_token_supply: total_stake_lamports,
                fee,
                ..StakePool::default()
            };
            let pool_token_fee = stake_pool.calc_fee_amount(reward_lamports).unwrap();

            stake_pool.total_stake_lamports += reward_lamports;
            stake_pool.pool_token_supply += pool_token_fee;

            let fee_lamports = stake_pool.calc_lamports_withdraw_amount(pool_token_fee).unwrap();
            let max_fee_lamports = u64::try_from((reward_lamports as u128) * (fee.numerator as u128) / (fee.denominator as u128)).unwrap();
            assert!(max_fee_lamports >= fee_lamports,
                "Max possible fee must always be greater than or equal to what is actually withdrawn, max {} actual {}",
                max_fee_lamports,
                fee_lamports);

            // since we do two "flooring" conversions, the max epsilon should be
            // correct up to 2 lamports (one for each floor division), plus a
            // correction for huge discrepancies between rewards and total stake
            let epsilon = 2 + reward_lamports / total_stake_lamports;
            assert!(max_fee_lamports - fee_lamports <= epsilon,
                "Max expected fee in lamports {}, actually receive {}, epsilon {}",
                max_fee_lamports, fee_lamports, epsilon);
        }
    }
}
