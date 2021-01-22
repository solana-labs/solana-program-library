//! State transition types

use crate::error::StakePoolError;
use crate::instruction::{unpack, Fee};
use crate::processor::Processor;
use core::convert::TryInto;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, program_error::ProgramError,
    pubkey::Pubkey,
};
use std::convert::TryFrom;
use std::mem::size_of;

/// Initialized program details.
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct StakePool {
    /// Pool version
    pub version: u8,
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
        self.calc_pool_withdraw_amount(stake_lamports)
    }
    /// calculate the pool tokens that should be withdrawn
    pub fn calc_pool_withdraw_amount(&self, stake_lamports: u64) -> Option<u64> {
        u64::try_from(
            (stake_lamports as u128)
                .checked_mul(self.pool_total as u128)?
                .checked_div(self.stake_total as u128)?,
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
}

/// Program states.
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq)]
#[allow(clippy::large_enum_variant)]
pub enum State {
    /// Unallocated state, may be initialized into another state.
    Unallocated,
    /// Initialized state.
    Init(StakePool),
}

impl State {
    /// Length of state data when serialized
    pub const LEN: usize = size_of::<u8>() + size_of::<StakePool>();
    /// Deserializes a byte buffer into a [State](struct.State.html).
    /// TODO efficient unpacking here
    pub fn deserialize(input: &[u8]) -> Result<State, ProgramError> {
        if input.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        Ok(match input[0] {
            0 => State::Unallocated,
            1 => {
                // We send whole input here, because unpack skips the first byte
                let swap: &StakePool = unpack(&input)?;
                State::Init(*swap)
            }
            _ => return Err(ProgramError::InvalidAccountData),
        })
    }

    /// Serializes [State](struct.State.html) into a byte buffer.
    /// TODO efficient packing here
    pub fn serialize(&self, output: &mut [u8]) -> ProgramResult {
        if output.len() < size_of::<u8>() {
            return Err(ProgramError::InvalidAccountData);
        }
        match self {
            Self::Unallocated => output[0] = 0,
            Self::Init(swap) => {
                if output.len() < size_of::<u8>() + size_of::<StakePool>() {
                    return Err(ProgramError::InvalidAccountData);
                }
                output[0] = 1;
                #[allow(clippy::cast_ptr_alignment)]
                let value = unsafe { &mut *(&mut output[1] as *mut u8 as *mut StakePool) };
                *value = *swap;
            }
        }
        Ok(())
    }
    /// Gets the `StakePool` from `State`
    pub fn stake_pool(&self) -> Result<StakePool, ProgramError> {
        if let State::Init(swap) = &self {
            Ok(*swap)
        } else {
            Err(StakePoolError::InvalidState.into())
        }
    }
}

const MAX_VALIDATOR_STAKE_ACCOUNTS: usize = 1000;

/// Storage list for all validator stake accounts in the pool.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ValidatorStakeList {
    /// False if not yet initialized
    pub is_initialized: bool,
    /// List of all validator stake accounts and their info
    pub validators: Vec<ValidatorStakeInfo>,
}

/// Information about the singe validator stake account
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub struct ValidatorStakeInfo {
    /// Validator account pubkey
    pub validator_account: Pubkey,

    /// Account balance in lamports
    pub balance: u64,

    /// Last epoch balance field was updated
    pub last_update_epoch: u64,
}

impl ValidatorStakeList {
    /// Length of ValidatorStakeList data when serialized
    pub const LEN: usize =
        Self::HEADER_LEN + ValidatorStakeInfo::LEN * MAX_VALIDATOR_STAKE_ACCOUNTS;

    /// Header length
    pub const HEADER_LEN: usize = size_of::<u8>() + size_of::<u16>();

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

    /// Deserializes a byte buffer into a ValidatorStakeList.
    pub fn deserialize(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        if input[0] == 0 {
            return Ok(ValidatorStakeList {
                is_initialized: false,
                validators: vec![],
            });
        }

        let number_of_validators: usize = u16::from_le_bytes(
            input[1..3]
                .try_into()
                .or(Err(ProgramError::InvalidAccountData))?,
        ) as usize;
        if number_of_validators > MAX_VALIDATOR_STAKE_ACCOUNTS {
            return Err(ProgramError::InvalidAccountData);
        }
        let mut validators: Vec<ValidatorStakeInfo> = Vec::with_capacity(number_of_validators);

        let mut from = Self::HEADER_LEN;
        let mut to = from + ValidatorStakeInfo::LEN;
        for _ in 0..number_of_validators {
            validators.push(ValidatorStakeInfo::deserialize(&input[from..to])?);
            from += ValidatorStakeInfo::LEN;
            to += ValidatorStakeInfo::LEN;
        }
        Ok(ValidatorStakeList {
            is_initialized: true,
            validators,
        })
    }

    /// Serializes ValidatorStakeList into a byte buffer.
    pub fn serialize(&self, output: &mut [u8]) -> ProgramResult {
        if output.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        if self.validators.len() > MAX_VALIDATOR_STAKE_ACCOUNTS {
            return Err(ProgramError::InvalidAccountData);
        }
        output[0] = if self.is_initialized { 1 } else { 0 };
        output[1..3].copy_from_slice(&u16::to_le_bytes(self.validators.len() as u16));
        let mut from = Self::HEADER_LEN;
        let mut to = from + ValidatorStakeInfo::LEN;
        for validator in &self.validators {
            validator.serialize(&mut output[from..to])?;
            from += ValidatorStakeInfo::LEN;
            to += ValidatorStakeInfo::LEN;
        }
        Ok(())
    }
}

impl ValidatorStakeInfo {
    /// Length of ValidatorStakeInfo data when serialized
    pub const LEN: usize = size_of::<ValidatorStakeInfo>();

    /// Deserializes a byte buffer into a ValidatorStakeInfo.
    pub fn deserialize(input: &[u8]) -> Result<Self, ProgramError> {
        if input.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }
        #[allow(clippy::cast_ptr_alignment)]
        let stake_info: &ValidatorStakeInfo =
            unsafe { &*(&input[0] as *const u8 as *const ValidatorStakeInfo) };
        Ok(*stake_info)
    }

    /// Serializes ValidatorStakeInfo into a byte buffer.
    pub fn serialize(&self, output: &mut [u8]) -> ProgramResult {
        if output.len() < Self::LEN {
            return Err(ProgramError::InvalidAccountData);
        }

        #[allow(clippy::cast_ptr_alignment)]
        let value = unsafe { &mut *(&mut output[0] as *mut u8 as *mut ValidatorStakeInfo) };
        *value = *self;
        Ok(())
    }
}

#[cfg(test)]
mod test {
    use super::*;

    #[test]
    fn test_state_packing() {
        // Not initialized
        let stake_list = ValidatorStakeList {
            is_initialized: false,
            validators: vec![],
        };
        let mut bytes: [u8; ValidatorStakeList::LEN] = [0; ValidatorStakeList::LEN];
        stake_list.serialize(&mut bytes).unwrap();
        let stake_list_unpacked = ValidatorStakeList::deserialize(&bytes).unwrap();
        assert_eq!(stake_list_unpacked, stake_list);

        // Empty
        let stake_list = ValidatorStakeList {
            is_initialized: true,
            validators: vec![],
        };
        let mut bytes: [u8; ValidatorStakeList::LEN] = [0; ValidatorStakeList::LEN];
        stake_list.serialize(&mut bytes).unwrap();
        let stake_list_unpacked = ValidatorStakeList::deserialize(&bytes).unwrap();
        assert_eq!(stake_list_unpacked, stake_list);

        // With several accounts
        let stake_list = ValidatorStakeList {
            is_initialized: true,
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
        let mut bytes: [u8; ValidatorStakeList::LEN] = [0; ValidatorStakeList::LEN];
        stake_list.serialize(&mut bytes).unwrap();
        let stake_list_unpacked = ValidatorStakeList::deserialize(&bytes).unwrap();
        assert_eq!(stake_list_unpacked, stake_list);
    }
}
