//! State transition types

use {
    crate::{
        big_vec::BigVec, error::StakePoolError, MAX_WITHDRAWAL_FEE_INCREASE,
        WITHDRAWAL_BASELINE_FEE,
    },
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    bytemuck::{Pod, Zeroable},
    num_derive::{FromPrimitive, ToPrimitive},
    num_traits::{FromPrimitive, ToPrimitive},
    solana_program::{
        account_info::AccountInfo,
        borsh1::get_instance_packed_len,
        msg,
        program_error::ProgramError,
        program_memory::sol_memcmp,
        program_pack::{Pack, Sealed},
        pubkey::{Pubkey, PUBKEY_BYTES},
        stake::state::Lockup,
    },
    spl_pod::primitives::{PodU32, PodU64},
    spl_token_2022::{
        extension::{BaseStateWithExtensions, ExtensionType, StateWithExtensions},
        state::{Account, AccountState, Mint},
    },
    std::{borrow::Borrow, convert::TryFrom, fmt, matches},
};

/// Enum representing the account type managed by the program
#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum AccountType {
    /// If the account has not been initialized, the enum will be 0
    #[default]
    Uninitialized,
    /// Stake pool
    StakePool,
    /// Validator stake list
    ValidatorList,
}

/// Initialized program details.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct StakePool {
    /// Account type, must be StakePool currently
    pub account_type: AccountType,

    /// Manager authority, allows for updating the staker, manager, and fee
    /// account
    pub manager: Pubkey,

    /// Staker authority, allows for adding and removing validators, and
    /// managing stake distribution
    pub staker: Pubkey,

    /// Stake deposit authority
    ///
    /// If a depositor pubkey is specified on initialization, then deposits must
    /// be signed by this authority. If no deposit authority is specified,
    /// then the stake pool will default to the result of:
    /// `Pubkey::find_program_address(
    ///     &[&stake_pool_address.as_ref(), b"deposit"],
    ///     program_id,
    /// )`
    pub stake_deposit_authority: Pubkey,

    /// Stake withdrawal authority bump seed
    /// for `create_program_address(&[state::StakePool account, "withdrawal"])`
    pub stake_withdraw_bump_seed: u8,

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
    pub total_lamports: u64,

    /// Total supply of pool tokens (should always match the supply in the Pool
    /// Mint)
    pub pool_token_supply: u64,

    /// Last epoch the `total_lamports` field was updated
    pub last_update_epoch: u64,

    /// Lockup that all stakes in the pool must have
    pub lockup: Lockup,

    /// Fee taken as a proportion of rewards each epoch
    pub epoch_fee: Fee,

    /// Fee for next epoch
    pub next_epoch_fee: FutureEpoch<Fee>,

    /// Preferred deposit validator vote account pubkey
    pub preferred_deposit_validator_vote_address: Option<Pubkey>,

    /// Preferred withdraw validator vote account pubkey
    pub preferred_withdraw_validator_vote_address: Option<Pubkey>,

    /// Fee assessed on stake deposits
    pub stake_deposit_fee: Fee,

    /// Fee assessed on withdrawals
    pub stake_withdrawal_fee: Fee,

    /// Future stake withdrawal fee, to be set for the following epoch
    pub next_stake_withdrawal_fee: FutureEpoch<Fee>,

    /// Fees paid out to referrers on referred stake deposits.
    /// Expressed as a percentage (0 - 100) of deposit fees.
    /// i.e. `stake_deposit_fee`% of stake deposited is collected as deposit
    /// fees for every deposit and `stake_referral_fee`% of the collected
    /// stake deposit fees is paid out to the referrer
    pub stake_referral_fee: u8,

    /// Toggles whether the `DepositSol` instruction requires a signature from
    /// this `sol_deposit_authority`
    pub sol_deposit_authority: Option<Pubkey>,

    /// Fee assessed on SOL deposits
    pub sol_deposit_fee: Fee,

    /// Fees paid out to referrers on referred SOL deposits.
    /// Expressed as a percentage (0 - 100) of SOL deposit fees.
    /// i.e. `sol_deposit_fee`% of SOL deposited is collected as deposit fees
    /// for every deposit and `sol_referral_fee`% of the collected SOL
    /// deposit fees is paid out to the referrer
    pub sol_referral_fee: u8,

    /// Toggles whether the `WithdrawSol` instruction requires a signature from
    /// the `deposit_authority`
    pub sol_withdraw_authority: Option<Pubkey>,

    /// Fee assessed on SOL withdrawals
    pub sol_withdrawal_fee: Fee,

    /// Future SOL withdrawal fee, to be set for the following epoch
    pub next_sol_withdrawal_fee: FutureEpoch<Fee>,

    /// Last epoch's total pool tokens, used only for APR estimation
    pub last_epoch_pool_token_supply: u64,

    /// Last epoch's total lamports, used only for APR estimation
    pub last_epoch_total_lamports: u64,
}
impl StakePool {
    /// calculate the pool tokens that should be minted for a deposit of
    /// `stake_lamports`
    #[inline]
    pub fn calc_pool_tokens_for_deposit(&self, stake_lamports: u64) -> Option<u64> {
        if self.total_lamports == 0 || self.pool_token_supply == 0 {
            return Some(stake_lamports);
        }
        u64::try_from(
            (stake_lamports as u128)
                .checked_mul(self.pool_token_supply as u128)?
                .checked_div(self.total_lamports as u128)?,
        )
        .ok()
    }

    /// calculate lamports amount on withdrawal
    #[inline]
    pub fn calc_lamports_withdraw_amount(&self, pool_tokens: u64) -> Option<u64> {
        // `checked_div` returns `None` for a 0 quotient result, but in this
        // case, a return of 0 is valid for small amounts of pool tokens. So
        // we check for that separately
        let numerator = (pool_tokens as u128).checked_mul(self.total_lamports as u128)?;
        let denominator = self.pool_token_supply as u128;
        if numerator < denominator || denominator == 0 {
            Some(0)
        } else {
            u64::try_from(numerator.checked_div(denominator)?).ok()
        }
    }

    /// calculate pool tokens to be deducted as withdrawal fees
    #[inline]
    pub fn calc_pool_tokens_stake_withdrawal_fee(&self, pool_tokens: u64) -> Option<u64> {
        u64::try_from(self.stake_withdrawal_fee.apply(pool_tokens)?).ok()
    }

    /// calculate pool tokens to be deducted as withdrawal fees
    #[inline]
    pub fn calc_pool_tokens_sol_withdrawal_fee(&self, pool_tokens: u64) -> Option<u64> {
        u64::try_from(self.sol_withdrawal_fee.apply(pool_tokens)?).ok()
    }

    /// calculate pool tokens to be deducted as stake deposit fees
    #[inline]
    pub fn calc_pool_tokens_stake_deposit_fee(&self, pool_tokens_minted: u64) -> Option<u64> {
        u64::try_from(self.stake_deposit_fee.apply(pool_tokens_minted)?).ok()
    }

    /// calculate pool tokens to be deducted from deposit fees as referral fees
    #[inline]
    pub fn calc_pool_tokens_stake_referral_fee(&self, stake_deposit_fee: u64) -> Option<u64> {
        u64::try_from(
            (stake_deposit_fee as u128)
                .checked_mul(self.stake_referral_fee as u128)?
                .checked_div(100u128)?,
        )
        .ok()
    }

    /// calculate pool tokens to be deducted as SOL deposit fees
    #[inline]
    pub fn calc_pool_tokens_sol_deposit_fee(&self, pool_tokens_minted: u64) -> Option<u64> {
        u64::try_from(self.sol_deposit_fee.apply(pool_tokens_minted)?).ok()
    }

    /// calculate pool tokens to be deducted from SOL deposit fees as referral
    /// fees
    #[inline]
    pub fn calc_pool_tokens_sol_referral_fee(&self, sol_deposit_fee: u64) -> Option<u64> {
        u64::try_from(
            (sol_deposit_fee as u128)
                .checked_mul(self.sol_referral_fee as u128)?
                .checked_div(100u128)?,
        )
        .ok()
    }

    /// Calculate the fee in pool tokens that goes to the manager
    ///
    /// This function assumes that `reward_lamports` has not already been added
    /// to the stake pool's `total_lamports`
    #[inline]
    pub fn calc_epoch_fee_amount(&self, reward_lamports: u64) -> Option<u64> {
        if reward_lamports == 0 {
            return Some(0);
        }
        let total_lamports = (self.total_lamports as u128).checked_add(reward_lamports as u128)?;
        let fee_lamports = self.epoch_fee.apply(reward_lamports)?;
        if total_lamports == fee_lamports || self.pool_token_supply == 0 {
            Some(reward_lamports)
        } else {
            u64::try_from(
                (self.pool_token_supply as u128)
                    .checked_mul(fee_lamports)?
                    .checked_div(total_lamports.checked_sub(fee_lamports)?)?,
            )
            .ok()
        }
    }

    /// Get the current value of pool tokens, rounded up
    #[inline]
    pub fn get_lamports_per_pool_token(&self) -> Option<u64> {
        self.total_lamports
            .checked_add(self.pool_token_supply)?
            .checked_sub(1)?
            .checked_div(self.pool_token_supply)
    }

    /// Checks that the withdraw or deposit authority is valid
    fn check_program_derived_authority(
        authority_address: &Pubkey,
        program_id: &Pubkey,
        stake_pool_address: &Pubkey,
        authority_seed: &[u8],
        bump_seed: u8,
    ) -> Result<(), ProgramError> {
        let expected_address = Pubkey::create_program_address(
            &[stake_pool_address.as_ref(), authority_seed, &[bump_seed]],
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

    /// Check if the manager fee info is a valid token program account
    /// capable of receiving tokens from the mint.
    pub(crate) fn check_manager_fee_info(
        &self,
        manager_fee_info: &AccountInfo,
    ) -> Result<(), ProgramError> {
        let account_data = manager_fee_info.try_borrow_data()?;
        let token_account = StateWithExtensions::<Account>::unpack(&account_data)?;
        if manager_fee_info.owner != &self.token_program_id
            || token_account.base.state != AccountState::Initialized
            || token_account.base.mint != self.pool_mint
        {
            msg!("Manager fee account is not owned by token program, is not initialized, or does not match stake pool's mint");
            return Err(StakePoolError::InvalidFeeAccount.into());
        }
        let extensions = token_account.get_extension_types()?;
        if extensions
            .iter()
            .any(|x| !is_extension_supported_for_fee_account(x))
        {
            return Err(StakePoolError::UnsupportedFeeAccountExtension.into());
        }
        Ok(())
    }

    /// Checks that the withdraw authority is valid
    #[inline]
    pub(crate) fn check_authority_withdraw(
        &self,
        withdraw_authority: &Pubkey,
        program_id: &Pubkey,
        stake_pool_address: &Pubkey,
    ) -> Result<(), ProgramError> {
        Self::check_program_derived_authority(
            withdraw_authority,
            program_id,
            stake_pool_address,
            crate::AUTHORITY_WITHDRAW,
            self.stake_withdraw_bump_seed,
        )
    }
    /// Checks that the deposit authority is valid
    #[inline]
    pub(crate) fn check_stake_deposit_authority(
        &self,
        stake_deposit_authority: &Pubkey,
    ) -> Result<(), ProgramError> {
        if self.stake_deposit_authority == *stake_deposit_authority {
            Ok(())
        } else {
            Err(StakePoolError::InvalidStakeDepositAuthority.into())
        }
    }

    /// Checks that the deposit authority is valid
    /// Does nothing if `sol_deposit_authority` is currently not set
    #[inline]
    pub(crate) fn check_sol_deposit_authority(
        &self,
        maybe_sol_deposit_authority: Result<&AccountInfo, ProgramError>,
    ) -> Result<(), ProgramError> {
        if let Some(auth) = self.sol_deposit_authority {
            let sol_deposit_authority = maybe_sol_deposit_authority?;
            if auth != *sol_deposit_authority.key {
                msg!("Expected {}, received {}", auth, sol_deposit_authority.key);
                return Err(StakePoolError::InvalidSolDepositAuthority.into());
            }
            if !sol_deposit_authority.is_signer {
                msg!("SOL Deposit authority signature missing");
                return Err(StakePoolError::SignatureMissing.into());
            }
        }
        Ok(())
    }

    /// Checks that the sol withdraw authority is valid
    /// Does nothing if `sol_withdraw_authority` is currently not set
    #[inline]
    pub(crate) fn check_sol_withdraw_authority(
        &self,
        maybe_sol_withdraw_authority: Result<&AccountInfo, ProgramError>,
    ) -> Result<(), ProgramError> {
        if let Some(auth) = self.sol_withdraw_authority {
            let sol_withdraw_authority = maybe_sol_withdraw_authority?;
            if auth != *sol_withdraw_authority.key {
                return Err(StakePoolError::InvalidSolWithdrawAuthority.into());
            }
            if !sol_withdraw_authority.is_signer {
                msg!("SOL withdraw authority signature missing");
                return Err(StakePoolError::SignatureMissing.into());
            }
        }
        Ok(())
    }

    /// Check mint is correct
    #[inline]
    pub(crate) fn check_mint(&self, mint_info: &AccountInfo) -> Result<u8, ProgramError> {
        if *mint_info.key != self.pool_mint {
            Err(StakePoolError::WrongPoolMint.into())
        } else {
            let mint_data = mint_info.try_borrow_data()?;
            let mint = StateWithExtensions::<Mint>::unpack(&mint_data)?;
            Ok(mint.base.decimals)
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

    /// Check the reserve stake is valid
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

    /// Updates one of the StakePool's fees.
    pub fn update_fee(&mut self, fee: &FeeType) -> Result<(), StakePoolError> {
        match fee {
            FeeType::SolReferral(new_fee) => self.sol_referral_fee = *new_fee,
            FeeType::StakeReferral(new_fee) => self.stake_referral_fee = *new_fee,
            FeeType::Epoch(new_fee) => self.next_epoch_fee = FutureEpoch::new(*new_fee),
            FeeType::StakeWithdrawal(new_fee) => {
                new_fee.check_withdrawal(&self.stake_withdrawal_fee)?;
                self.next_stake_withdrawal_fee = FutureEpoch::new(*new_fee)
            }
            FeeType::SolWithdrawal(new_fee) => {
                new_fee.check_withdrawal(&self.sol_withdrawal_fee)?;
                self.next_sol_withdrawal_fee = FutureEpoch::new(*new_fee)
            }
            FeeType::SolDeposit(new_fee) => self.sol_deposit_fee = *new_fee,
            FeeType::StakeDeposit(new_fee) => self.stake_deposit_fee = *new_fee,
        };
        Ok(())
    }
}

/// Checks if the given extension is supported for the stake pool mint
pub fn is_extension_supported_for_mint(extension_type: &ExtensionType) -> bool {
    const SUPPORTED_EXTENSIONS: [ExtensionType; 8] = [
        ExtensionType::Uninitialized,
        ExtensionType::TransferFeeConfig,
        ExtensionType::ConfidentialTransferMint,
        ExtensionType::ConfidentialTransferFeeConfig,
        ExtensionType::DefaultAccountState, // ok, but a freeze authority is not
        ExtensionType::InterestBearingConfig,
        ExtensionType::MetadataPointer,
        ExtensionType::TokenMetadata,
    ];
    if !SUPPORTED_EXTENSIONS.contains(extension_type) {
        msg!(
            "Stake pool mint account cannot have the {:?} extension",
            extension_type
        );
        false
    } else {
        true
    }
}

/// Checks if the given extension is supported for the stake pool's fee account
pub fn is_extension_supported_for_fee_account(extension_type: &ExtensionType) -> bool {
    // Note: this does not include the `ConfidentialTransferAccount` extension
    // because it is possible to block non-confidential transfers with the
    // extension enabled.
    const SUPPORTED_EXTENSIONS: [ExtensionType; 4] = [
        ExtensionType::Uninitialized,
        ExtensionType::TransferFeeAmount,
        ExtensionType::ImmutableOwner,
        ExtensionType::CpiGuard,
    ];
    if !SUPPORTED_EXTENSIONS.contains(extension_type) {
        msg!("Fee account cannot have the {:?} extension", extension_type);
        false
    } else {
        true
    }
}

/// Storage list for all validator stake accounts in the pool.
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct ValidatorList {
    /// Data outside of the validator list, separated out for cheaper
    /// deserializations
    pub header: ValidatorListHeader,

    /// List of stake info for each validator in the pool
    pub validators: Vec<ValidatorStakeInfo>,
}

/// Helper type to deserialize just the start of a ValidatorList
#[repr(C)]
#[derive(Clone, Debug, Default, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct ValidatorListHeader {
    /// Account type, must be ValidatorList currently
    pub account_type: AccountType,

    /// Maximum allowable number of validators
    pub max_validators: u32,
}

/// Status of the stake account in the validator list, for accounting
#[derive(
    ToPrimitive,
    FromPrimitive,
    Copy,
    Clone,
    Debug,
    PartialEq,
    BorshDeserialize,
    BorshSerialize,
    BorshSchema,
)]
pub enum StakeStatus {
    /// Stake account is active, there may be a transient stake as well
    Active,
    /// Only transient stake account exists, when a transient stake is
    /// deactivating during validator removal
    DeactivatingTransient,
    /// No more validator stake accounts exist, entry ready for removal during
    /// `UpdateStakePoolBalance`
    ReadyForRemoval,
    /// Only the validator stake account is deactivating, no transient stake
    /// account exists
    DeactivatingValidator,
    /// Both the transient and validator stake account are deactivating, when
    /// a validator is removed with a transient stake active
    DeactivatingAll,
}
impl Default for StakeStatus {
    fn default() -> Self {
        Self::Active
    }
}

/// Wrapper struct that can be `Pod`, containing a byte that *should* be a valid
/// `StakeStatus` underneath.
#[repr(transparent)]
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Pod,
    Zeroable,
    BorshDeserialize,
    BorshSerialize,
    BorshSchema,
)]
pub struct PodStakeStatus(u8);
impl PodStakeStatus {
    /// Downgrade the status towards ready for removal by removing the validator
    /// stake
    pub fn remove_validator_stake(&mut self) -> Result<(), ProgramError> {
        let status = StakeStatus::try_from(*self)?;
        let new_self = match status {
            StakeStatus::Active
            | StakeStatus::DeactivatingTransient
            | StakeStatus::ReadyForRemoval => status,
            StakeStatus::DeactivatingAll => StakeStatus::DeactivatingTransient,
            StakeStatus::DeactivatingValidator => StakeStatus::ReadyForRemoval,
        };
        *self = new_self.into();
        Ok(())
    }
    /// Downgrade the status towards ready for removal by removing the transient
    /// stake
    pub fn remove_transient_stake(&mut self) -> Result<(), ProgramError> {
        let status = StakeStatus::try_from(*self)?;
        let new_self = match status {
            StakeStatus::Active
            | StakeStatus::DeactivatingValidator
            | StakeStatus::ReadyForRemoval => status,
            StakeStatus::DeactivatingAll => StakeStatus::DeactivatingValidator,
            StakeStatus::DeactivatingTransient => StakeStatus::ReadyForRemoval,
        };
        *self = new_self.into();
        Ok(())
    }
}
impl TryFrom<PodStakeStatus> for StakeStatus {
    type Error = ProgramError;
    fn try_from(pod: PodStakeStatus) -> Result<Self, Self::Error> {
        FromPrimitive::from_u8(pod.0).ok_or(ProgramError::InvalidAccountData)
    }
}
impl From<StakeStatus> for PodStakeStatus {
    fn from(status: StakeStatus) -> Self {
        // unwrap is safe here because the variants of `StakeStatus` fit very
        // comfortably within a `u8`
        PodStakeStatus(status.to_u8().unwrap())
    }
}

/// Withdrawal type, figured out during process_withdraw_stake
#[derive(Debug, PartialEq)]
pub(crate) enum StakeWithdrawSource {
    /// Some of an active stake account, but not all
    Active,
    /// Some of a transient stake account
    Transient,
    /// Take a whole validator stake account
    ValidatorRemoval,
}

/// Information about a validator in the pool
///
/// NOTE: ORDER IS VERY IMPORTANT HERE, PLEASE DO NOT RE-ORDER THE FIELDS UNLESS
/// THERE'S AN EXTREMELY GOOD REASON.
///
/// To save on BPF instructions, the serialized bytes are reinterpreted with a
/// bytemuck transmute, which means that this structure cannot have any
/// undeclared alignment-padding in its representation.
#[repr(C)]
#[derive(
    Clone,
    Copy,
    Debug,
    Default,
    PartialEq,
    Pod,
    Zeroable,
    BorshDeserialize,
    BorshSerialize,
    BorshSchema,
)]
pub struct ValidatorStakeInfo {
    /// Amount of lamports on the validator stake account, including rent
    ///
    /// Note that if `last_update_epoch` does not match the current epoch then
    /// this field may not be accurate
    pub active_stake_lamports: PodU64,

    /// Amount of transient stake delegated to this validator
    ///
    /// Note that if `last_update_epoch` does not match the current epoch then
    /// this field may not be accurate
    pub transient_stake_lamports: PodU64,

    /// Last epoch the active and transient stake lamports fields were updated
    pub last_update_epoch: PodU64,

    /// Transient account seed suffix, used to derive the transient stake
    /// account address
    pub transient_seed_suffix: PodU64,

    /// Unused space, initially meant to specify the end of seed suffixes
    pub unused: PodU32,

    /// Validator account seed suffix
    pub validator_seed_suffix: PodU32, // really `Option<NonZeroU32>` so 0 is `None`

    /// Status of the validator stake account
    pub status: PodStakeStatus,

    /// Validator vote account address
    pub vote_account_address: Pubkey,
}

impl ValidatorStakeInfo {
    /// Get the total lamports on this validator (active and transient)
    pub fn stake_lamports(&self) -> Result<u64, StakePoolError> {
        u64::from(self.active_stake_lamports)
            .checked_add(self.transient_stake_lamports.into())
            .ok_or(StakePoolError::CalculationFailure)
    }

    /// Performs a very cheap comparison, for checking if this validator stake
    /// info matches the vote account address
    pub fn memcmp_pubkey(data: &[u8], vote_address: &Pubkey) -> bool {
        sol_memcmp(
            &data[41..41_usize.saturating_add(PUBKEY_BYTES)],
            vote_address.as_ref(),
            PUBKEY_BYTES,
        ) == 0
    }

    /// Performs a comparison, used to check if this validator stake
    /// info has more active lamports than some limit
    pub fn active_lamports_greater_than(data: &[u8], lamports: &u64) -> bool {
        // without this unwrap, compute usage goes up significantly
        u64::try_from_slice(&data[0..8]).unwrap() > *lamports
    }

    /// Performs a comparison, used to check if this validator stake
    /// info has more transient lamports than some limit
    pub fn transient_lamports_greater_than(data: &[u8], lamports: &u64) -> bool {
        // without this unwrap, compute usage goes up significantly
        u64::try_from_slice(&data[8..16]).unwrap() > *lamports
    }

    /// Check that the validator stake info is valid
    pub fn is_not_removed(data: &[u8]) -> bool {
        FromPrimitive::from_u8(data[40]) != Some(StakeStatus::ReadyForRemoval)
    }
}

impl Sealed for ValidatorStakeInfo {}

impl Pack for ValidatorStakeInfo {
    const LEN: usize = 73;
    fn pack_into_slice(&self, data: &mut [u8]) {
        // Removing this unwrap would require changing from `Pack` to some other
        // trait or `bytemuck`, so it stays in for now
        borsh::to_writer(data, self).unwrap();
    }
    fn unpack_from_slice(src: &[u8]) -> Result<Self, ProgramError> {
        let unpacked = Self::try_from_slice(src)?;
        Ok(unpacked)
    }
}

impl ValidatorList {
    /// Create an empty instance containing space for `max_validators` and
    /// preferred validator keys
    pub fn new(max_validators: u32) -> Self {
        Self {
            header: ValidatorListHeader {
                account_type: AccountType::ValidatorList,
                max_validators,
            },
            validators: vec![ValidatorStakeInfo::default(); max_validators as usize],
        }
    }

    /// Calculate the number of validator entries that fit in the provided
    /// length
    pub fn calculate_max_validators(buffer_length: usize) -> usize {
        let header_size = ValidatorListHeader::LEN.saturating_add(4);
        buffer_length
            .saturating_sub(header_size)
            .saturating_div(ValidatorStakeInfo::LEN)
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

    /// Check if the list has any active stake
    pub fn has_active_stake(&self) -> bool {
        self.validators
            .iter()
            .any(|x| u64::from(x.active_stake_lamports) > 0)
    }
}

impl ValidatorListHeader {
    const LEN: usize = 1 + 4;

    /// Check if validator stake list is actually initialized as a validator
    /// stake list
    pub fn is_valid(&self) -> bool {
        self.account_type == AccountType::ValidatorList
    }

    /// Check if the validator stake list is uninitialized
    pub fn is_uninitialized(&self) -> bool {
        self.account_type == AccountType::Uninitialized
    }

    /// Extracts a slice of ValidatorStakeInfo types from the vec part
    /// of the ValidatorList
    pub fn deserialize_mut_slice<'a>(
        big_vec: &'a mut BigVec,
        skip: usize,
        len: usize,
    ) -> Result<&'a mut [ValidatorStakeInfo], ProgramError> {
        big_vec.deserialize_mut_slice::<ValidatorStakeInfo>(skip, len)
    }

    /// Extracts the validator list into its header and internal BigVec
    pub fn deserialize_vec(data: &mut [u8]) -> Result<(Self, BigVec), ProgramError> {
        let mut data_mut = data.borrow();
        let header = ValidatorListHeader::deserialize(&mut data_mut)?;
        let length = get_instance_packed_len(&header)?;

        let big_vec = BigVec {
            data: &mut data[length..],
        };
        Ok((header, big_vec))
    }
}

/// Wrapper type that "counts down" epochs, which is Borsh-compatible with the
/// native `Option`
#[repr(C)]
#[derive(Clone, Copy, Debug, PartialEq, BorshSerialize, BorshDeserialize, BorshSchema)]
pub enum FutureEpoch<T> {
    /// Nothing is set
    None,
    /// Value is ready after the next epoch boundary
    One(T),
    /// Value is ready after two epoch boundaries
    Two(T),
}
impl<T> Default for FutureEpoch<T> {
    fn default() -> Self {
        Self::None
    }
}
impl<T> FutureEpoch<T> {
    /// Create a new value to be unlocked in a two epochs
    pub fn new(value: T) -> Self {
        Self::Two(value)
    }
}
impl<T: Clone> FutureEpoch<T> {
    /// Update the epoch, to be done after `get`ting the underlying value
    pub fn update_epoch(&mut self) {
        match self {
            Self::None => {}
            Self::One(_) => {
                // The value has waited its last epoch
                *self = Self::None;
            }
            // The value still has to wait one more epoch after this
            Self::Two(v) => {
                *self = Self::One(v.clone());
            }
        }
    }

    /// Get the value if it's ready, which is only at `One` epoch remaining
    pub fn get(&self) -> Option<&T> {
        match self {
            Self::None | Self::Two(_) => None,
            Self::One(v) => Some(v),
        }
    }
}
impl<T> From<FutureEpoch<T>> for Option<T> {
    fn from(v: FutureEpoch<T>) -> Option<T> {
        match v {
            FutureEpoch::None => None,
            FutureEpoch::One(inner) | FutureEpoch::Two(inner) => Some(inner),
        }
    }
}

/// Fee rate as a ratio, minted on `UpdateStakePoolBalance` as a proportion of
/// the rewards
/// If either the numerator or the denominator is 0, the fee is considered to be
/// 0
#[repr(C)]
#[derive(Clone, Copy, Debug, Default, PartialEq, BorshSerialize, BorshDeserialize, BorshSchema)]
pub struct Fee {
    /// denominator of the fee ratio
    pub denominator: u64,
    /// numerator of the fee ratio
    pub numerator: u64,
}

impl Fee {
    /// Applies the Fee's rates to a given amount, `amt`
    /// returning the amount to be subtracted from it as fees
    /// (0 if denominator is 0 or amt is 0),
    /// or None if overflow occurs
    #[inline]
    pub fn apply(&self, amt: u64) -> Option<u128> {
        if self.denominator == 0 {
            return Some(0);
        }
        let numerator = (amt as u128).checked_mul(self.numerator as u128)?;
        // ceiling the calculation by adding (denominator - 1) to the numerator
        let denominator = self.denominator as u128;
        numerator
            .checked_add(denominator)?
            .checked_sub(1)?
            .checked_div(denominator)
    }

    /// Withdrawal fees have some additional restrictions,
    /// this fn checks if those are met, returning an error if not.
    /// Does nothing and returns Ok if fee type is not withdrawal
    pub fn check_withdrawal(&self, old_withdrawal_fee: &Fee) -> Result<(), StakePoolError> {
        // If the previous withdrawal fee was 0, we allow the fee to be set to a
        // maximum of (WITHDRAWAL_BASELINE_FEE * MAX_WITHDRAWAL_FEE_INCREASE)
        let (old_num, old_denom) =
            if old_withdrawal_fee.denominator == 0 || old_withdrawal_fee.numerator == 0 {
                (
                    WITHDRAWAL_BASELINE_FEE.numerator,
                    WITHDRAWAL_BASELINE_FEE.denominator,
                )
            } else {
                (old_withdrawal_fee.numerator, old_withdrawal_fee.denominator)
            };

        // Check that new_fee / old_fee <= MAX_WITHDRAWAL_FEE_INCREASE
        // Program fails if provided numerator or denominator is too large, resulting in
        // overflow
        if (old_num as u128)
            .checked_mul(self.denominator as u128)
            .map(|x| x.checked_mul(MAX_WITHDRAWAL_FEE_INCREASE.numerator as u128))
            .ok_or(StakePoolError::CalculationFailure)?
            < (self.numerator as u128)
                .checked_mul(old_denom as u128)
                .map(|x| x.checked_mul(MAX_WITHDRAWAL_FEE_INCREASE.denominator as u128))
                .ok_or(StakePoolError::CalculationFailure)?
        {
            msg!(
                "Fee increase exceeds maximum allowed, proposed increase factor ({} / {})",
                self.numerator.saturating_mul(old_denom),
                old_num.saturating_mul(self.denominator),
            );
            return Err(StakePoolError::FeeIncreaseTooHigh);
        }
        Ok(())
    }
}

impl fmt::Display for Fee {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        if self.numerator > 0 && self.denominator > 0 {
            write!(f, "{}/{}", self.numerator, self.denominator)
        } else {
            write!(f, "none")
        }
    }
}

/// The type of fees that can be set on the stake pool
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum FeeType {
    /// Referral fees for SOL deposits
    SolReferral(u8),
    /// Referral fees for stake deposits
    StakeReferral(u8),
    /// Management fee paid per epoch
    Epoch(Fee),
    /// Stake withdrawal fee
    StakeWithdrawal(Fee),
    /// Deposit fee for SOL deposits
    SolDeposit(Fee),
    /// Deposit fee for stake deposits
    StakeDeposit(Fee),
    /// SOL withdrawal fee
    SolWithdrawal(Fee),
}

impl FeeType {
    /// Checks if the provided fee is too high, returning an error if so
    pub fn check_too_high(&self) -> Result<(), StakePoolError> {
        let too_high = match self {
            Self::SolReferral(pct) => *pct > 100u8,
            Self::StakeReferral(pct) => *pct > 100u8,
            Self::Epoch(fee) => fee.numerator > fee.denominator,
            Self::StakeWithdrawal(fee) => fee.numerator > fee.denominator,
            Self::SolWithdrawal(fee) => fee.numerator > fee.denominator,
            Self::SolDeposit(fee) => fee.numerator > fee.denominator,
            Self::StakeDeposit(fee) => fee.numerator > fee.denominator,
        };
        if too_high {
            msg!("Fee greater than 100%: {:?}", self);
            return Err(StakePoolError::FeeTooHigh);
        }
        Ok(())
    }

    /// Returns if the contained fee can only be updated earliest on the next
    /// epoch
    #[inline]
    pub fn can_only_change_next_epoch(&self) -> bool {
        matches!(
            self,
            Self::StakeWithdrawal(_) | Self::SolWithdrawal(_) | Self::Epoch(_)
        )
    }
}

#[cfg(test)]
mod test {
    #![allow(clippy::arithmetic_side_effects)]
    use {
        super::*,
        proptest::prelude::*,
        solana_program::{
            borsh1::{get_packed_len, try_from_slice_unchecked},
            clock::{DEFAULT_SLOTS_PER_EPOCH, DEFAULT_S_PER_SLOT, SECONDS_PER_DAY},
            native_token::LAMPORTS_PER_SOL,
        },
    };

    fn uninitialized_validator_list() -> ValidatorList {
        ValidatorList {
            header: ValidatorListHeader {
                account_type: AccountType::Uninitialized,
                max_validators: 0,
            },
            validators: vec![],
        }
    }

    fn test_validator_list(max_validators: u32) -> ValidatorList {
        ValidatorList {
            header: ValidatorListHeader {
                account_type: AccountType::ValidatorList,
                max_validators,
            },
            validators: vec![
                ValidatorStakeInfo {
                    status: StakeStatus::Active.into(),
                    vote_account_address: Pubkey::new_from_array([1; 32]),
                    active_stake_lamports: u64::from_le_bytes([255; 8]).into(),
                    transient_stake_lamports: u64::from_le_bytes([128; 8]).into(),
                    last_update_epoch: u64::from_le_bytes([64; 8]).into(),
                    transient_seed_suffix: 0.into(),
                    unused: 0.into(),
                    validator_seed_suffix: 0.into(),
                },
                ValidatorStakeInfo {
                    status: StakeStatus::DeactivatingTransient.into(),
                    vote_account_address: Pubkey::new_from_array([2; 32]),
                    active_stake_lamports: 998877665544.into(),
                    transient_stake_lamports: 222222222.into(),
                    last_update_epoch: 11223445566.into(),
                    transient_seed_suffix: 0.into(),
                    unused: 0.into(),
                    validator_seed_suffix: 0.into(),
                },
                ValidatorStakeInfo {
                    status: StakeStatus::ReadyForRemoval.into(),
                    vote_account_address: Pubkey::new_from_array([3; 32]),
                    active_stake_lamports: 0.into(),
                    transient_stake_lamports: 0.into(),
                    last_update_epoch: 999999999999999.into(),
                    transient_seed_suffix: 0.into(),
                    unused: 0.into(),
                    validator_seed_suffix: 0.into(),
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
        let bytes = byte_vec.as_mut_slice();
        borsh::to_writer(bytes, &stake_list).unwrap();
        let stake_list_unpacked = try_from_slice_unchecked::<ValidatorList>(&byte_vec).unwrap();
        assert_eq!(stake_list_unpacked, stake_list);

        // Empty, one preferred key
        let stake_list = ValidatorList {
            header: ValidatorListHeader {
                account_type: AccountType::ValidatorList,
                max_validators: 0,
            },
            validators: vec![],
        };
        let mut byte_vec = vec![0u8; size];
        let bytes = byte_vec.as_mut_slice();
        borsh::to_writer(bytes, &stake_list).unwrap();
        let stake_list_unpacked = try_from_slice_unchecked::<ValidatorList>(&byte_vec).unwrap();
        assert_eq!(stake_list_unpacked, stake_list);

        // With several accounts
        let stake_list = test_validator_list(max_validators);
        let mut byte_vec = vec![0u8; size];
        let bytes = byte_vec.as_mut_slice();
        borsh::to_writer(bytes, &stake_list).unwrap();
        let stake_list_unpacked = try_from_slice_unchecked::<ValidatorList>(&byte_vec).unwrap();
        assert_eq!(stake_list_unpacked, stake_list);
    }

    #[test]
    fn validator_list_active_stake() {
        let max_validators = 10_000;
        let mut validator_list = test_validator_list(max_validators);
        assert!(validator_list.has_active_stake());
        for validator in validator_list.validators.iter_mut() {
            validator.active_stake_lamports = 0.into();
        }
        assert!(!validator_list.has_active_stake());
    }

    #[test]
    fn validator_list_deserialize_mut_slice() {
        let max_validators = 10;
        let stake_list = test_validator_list(max_validators);
        let mut serialized = borsh::to_vec(&stake_list).unwrap();
        let (header, mut big_vec) = ValidatorListHeader::deserialize_vec(&mut serialized).unwrap();
        let list = ValidatorListHeader::deserialize_mut_slice(
            &mut big_vec,
            0,
            stake_list.validators.len(),
        )
        .unwrap();
        assert_eq!(header.account_type, AccountType::ValidatorList);
        assert_eq!(header.max_validators, max_validators);
        assert!(list
            .iter()
            .zip(stake_list.validators.iter())
            .all(|(a, b)| a == b));

        let list = ValidatorListHeader::deserialize_mut_slice(&mut big_vec, 1, 2).unwrap();
        assert!(list
            .iter()
            .zip(stake_list.validators[1..].iter())
            .all(|(a, b)| a == b));
        let list = ValidatorListHeader::deserialize_mut_slice(&mut big_vec, 2, 1).unwrap();
        assert!(list
            .iter()
            .zip(stake_list.validators[2..].iter())
            .all(|(a, b)| a == b));
        let list = ValidatorListHeader::deserialize_mut_slice(&mut big_vec, 0, 2).unwrap();
        assert!(list
            .iter()
            .zip(stake_list.validators[..2].iter())
            .all(|(a, b)| a == b));

        assert_eq!(
            ValidatorListHeader::deserialize_mut_slice(&mut big_vec, 0, 4).unwrap_err(),
            ProgramError::AccountDataTooSmall
        );
        assert_eq!(
            ValidatorListHeader::deserialize_mut_slice(&mut big_vec, 1, 3).unwrap_err(),
            ProgramError::AccountDataTooSmall
        );
    }

    #[test]
    fn validator_list_iter() {
        let max_validators = 10;
        let stake_list = test_validator_list(max_validators);
        let mut serialized = borsh::to_vec(&stake_list).unwrap();
        let (_, big_vec) = ValidatorListHeader::deserialize_vec(&mut serialized).unwrap();
        for (a, b) in big_vec
            .deserialize_slice::<ValidatorStakeInfo>(0, big_vec.len() as usize)
            .unwrap()
            .iter()
            .zip(stake_list.validators.iter())
        {
            assert_eq!(a, b);
        }
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
        fn total_stake_and_rewards()(total_lamports in 1..u64::MAX)(
            total_lamports in Just(total_lamports),
            rewards in 0..=total_lamports,
        ) -> (u64, u64) {
            (total_lamports - rewards, rewards)
        }
    }

    #[test]
    fn specific_fee_calculation() {
        // 10% of 10 SOL in rewards should be 1 SOL in fees
        let epoch_fee = Fee {
            numerator: 1,
            denominator: 10,
        };
        let mut stake_pool = StakePool {
            total_lamports: 100 * LAMPORTS_PER_SOL,
            pool_token_supply: 100 * LAMPORTS_PER_SOL,
            epoch_fee,
            ..StakePool::default()
        };
        let reward_lamports = 10 * LAMPORTS_PER_SOL;
        let pool_token_fee = stake_pool.calc_epoch_fee_amount(reward_lamports).unwrap();

        stake_pool.total_lamports += reward_lamports;
        stake_pool.pool_token_supply += pool_token_fee;

        let fee_lamports = stake_pool
            .calc_lamports_withdraw_amount(pool_token_fee)
            .unwrap();
        assert_eq!(fee_lamports, LAMPORTS_PER_SOL - 1); // off-by-one due to
                                                        // truncation
    }

    #[test]
    fn zero_withdraw_calculation() {
        let epoch_fee = Fee {
            numerator: 0,
            denominator: 1,
        };
        let stake_pool = StakePool {
            epoch_fee,
            ..StakePool::default()
        };
        let fee_lamports = stake_pool.calc_lamports_withdraw_amount(0).unwrap();
        assert_eq!(fee_lamports, 0);
    }

    #[test]
    fn divide_by_zero_fee() {
        let stake_pool = StakePool {
            total_lamports: 0,
            epoch_fee: Fee {
                numerator: 1,
                denominator: 10,
            },
            ..StakePool::default()
        };
        let rewards = 10;
        let fee = stake_pool.calc_epoch_fee_amount(rewards).unwrap();
        assert_eq!(fee, rewards);
    }

    #[test]
    fn approximate_apr_calculation() {
        // 8% / year means roughly .044% / epoch
        let stake_pool = StakePool {
            last_epoch_total_lamports: 100_000,
            last_epoch_pool_token_supply: 100_000,
            total_lamports: 100_044,
            pool_token_supply: 100_000,
            ..StakePool::default()
        };
        let pool_token_value =
            stake_pool.total_lamports as f64 / stake_pool.pool_token_supply as f64;
        let last_epoch_pool_token_value = stake_pool.last_epoch_total_lamports as f64
            / stake_pool.last_epoch_pool_token_supply as f64;
        let epoch_rate = pool_token_value / last_epoch_pool_token_value - 1.0;
        const SECONDS_PER_EPOCH: f64 = DEFAULT_SLOTS_PER_EPOCH as f64 * DEFAULT_S_PER_SLOT;
        const EPOCHS_PER_YEAR: f64 = SECONDS_PER_DAY as f64 * 365.25 / SECONDS_PER_EPOCH;
        const EPSILON: f64 = 0.00001;
        let yearly_rate = epoch_rate * EPOCHS_PER_YEAR;
        assert!((yearly_rate - 0.080355).abs() < EPSILON);
    }

    proptest! {
        #[test]
        fn fee_calculation(
            (numerator, denominator) in fee(),
            (total_lamports, reward_lamports) in total_stake_and_rewards(),
        ) {
            let epoch_fee = Fee { denominator, numerator };
            let mut stake_pool = StakePool {
                total_lamports,
                pool_token_supply: total_lamports,
                epoch_fee,
                ..StakePool::default()
            };
            let pool_token_fee = stake_pool.calc_epoch_fee_amount(reward_lamports).unwrap();

            stake_pool.total_lamports += reward_lamports;
            stake_pool.pool_token_supply += pool_token_fee;

            let fee_lamports = stake_pool.calc_lamports_withdraw_amount(pool_token_fee).unwrap();
            let max_fee_lamports = u64::try_from((reward_lamports as u128) * (epoch_fee.numerator as u128) / (epoch_fee.denominator as u128)).unwrap();
            assert!(max_fee_lamports >= fee_lamports,
                "Max possible fee must always be greater than or equal to what is actually withdrawn, max {} actual {}",
                max_fee_lamports,
                fee_lamports);

            // since we do two "flooring" conversions, the max epsilon should be
            // correct up to 2 lamports (one for each floor division), plus a
            // correction for huge discrepancies between rewards and total stake
            let epsilon = 2 + reward_lamports / total_lamports;
            assert!(max_fee_lamports - fee_lamports <= epsilon,
                "Max expected fee in lamports {}, actually receive {}, epsilon {}",
                max_fee_lamports, fee_lamports, epsilon);
        }
    }

    prop_compose! {
        fn total_tokens_and_deposit()(total_lamports in 1..u64::MAX)(
            total_lamports in Just(total_lamports),
            pool_token_supply in 1..=total_lamports,
            deposit_lamports in 1..total_lamports,
        ) -> (u64, u64, u64) {
            (total_lamports - deposit_lamports, pool_token_supply.saturating_sub(deposit_lamports).max(1), deposit_lamports)
        }
    }

    proptest! {
        #[test]
        fn deposit_and_withdraw(
            (total_lamports, pool_token_supply, deposit_stake) in total_tokens_and_deposit()
        ) {
            let mut stake_pool = StakePool {
                total_lamports,
                pool_token_supply,
                ..StakePool::default()
            };
            let deposit_result = stake_pool.calc_pool_tokens_for_deposit(deposit_stake).unwrap();
            prop_assume!(deposit_result > 0);
            stake_pool.total_lamports += deposit_stake;
            stake_pool.pool_token_supply += deposit_result;
            let withdraw_result = stake_pool.calc_lamports_withdraw_amount(deposit_result).unwrap();
            assert!(withdraw_result <= deposit_stake);

            // also test splitting the withdrawal in two operations
            if deposit_result >= 2 {
                let first_half_deposit = deposit_result / 2;
                let first_withdraw_result = stake_pool.calc_lamports_withdraw_amount(first_half_deposit).unwrap();
                stake_pool.total_lamports -= first_withdraw_result;
                stake_pool.pool_token_supply -= first_half_deposit;
                let second_half_deposit = deposit_result - first_half_deposit; // do the whole thing
                let second_withdraw_result = stake_pool.calc_lamports_withdraw_amount(second_half_deposit).unwrap();
                assert!(first_withdraw_result + second_withdraw_result <= deposit_stake);
            }
        }
    }

    #[test]
    fn specific_split_withdrawal() {
        let total_lamports = 1_100_000_000_000;
        let pool_token_supply = 1_000_000_000_000;
        let deposit_stake = 3;
        let mut stake_pool = StakePool {
            total_lamports,
            pool_token_supply,
            ..StakePool::default()
        };
        let deposit_result = stake_pool
            .calc_pool_tokens_for_deposit(deposit_stake)
            .unwrap();
        assert!(deposit_result > 0);
        stake_pool.total_lamports += deposit_stake;
        stake_pool.pool_token_supply += deposit_result;
        let withdraw_result = stake_pool
            .calc_lamports_withdraw_amount(deposit_result / 2)
            .unwrap();
        assert!(withdraw_result * 2 <= deposit_stake);
    }

    #[test]
    fn withdraw_all() {
        let total_lamports = 1_100_000_000_000;
        let pool_token_supply = 1_000_000_000_000;
        let mut stake_pool = StakePool {
            total_lamports,
            pool_token_supply,
            ..StakePool::default()
        };
        // take everything out at once
        let withdraw_result = stake_pool
            .calc_lamports_withdraw_amount(pool_token_supply)
            .unwrap();
        assert_eq!(stake_pool.total_lamports, withdraw_result);

        // take out 1, then the rest
        let withdraw_result = stake_pool.calc_lamports_withdraw_amount(1).unwrap();
        stake_pool.total_lamports -= withdraw_result;
        stake_pool.pool_token_supply -= 1;
        let withdraw_result = stake_pool
            .calc_lamports_withdraw_amount(stake_pool.pool_token_supply)
            .unwrap();
        assert_eq!(stake_pool.total_lamports, withdraw_result);

        // take out all except 1, then the rest
        let mut stake_pool = StakePool {
            total_lamports,
            pool_token_supply,
            ..StakePool::default()
        };
        let withdraw_result = stake_pool
            .calc_lamports_withdraw_amount(pool_token_supply - 1)
            .unwrap();
        stake_pool.total_lamports -= withdraw_result;
        stake_pool.pool_token_supply = 1;
        assert_ne!(stake_pool.total_lamports, 0);

        let withdraw_result = stake_pool.calc_lamports_withdraw_amount(1).unwrap();
        assert_eq!(stake_pool.total_lamports, withdraw_result);
    }
}
