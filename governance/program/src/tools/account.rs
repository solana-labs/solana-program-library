//! General purpose account utility functions

use borsh::{BorshDeserialize, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, borsh::try_from_slice_unchecked, msg, program::invoke_signed,
    program_error::ProgramError, program_pack::IsInitialized, pubkey::Pubkey, rent::Rent,
    system_instruction::create_account,
};

use crate::error::GovernanceError;

/// Trait for accounts to return their max size
pub trait AccountMaxSize {
    /// Returns max account size or None if max size is not known and actual instance size should be used
    fn get_max_size(&self) -> Option<usize> {
        None
    }
}

/// Creates a new account and serializes data into it using the provided seeds to invoke signed CPI call
/// Note: This functions also checks the provided account PDA matches the supplied seeds
pub fn create_and_serialize_account_signed<'a, T: BorshSerialize + AccountMaxSize>(
    payer_info: &AccountInfo<'a>,
    account_info: &AccountInfo<'a>,
    account_data: &T,
    account_address_seeds: &[&[u8]],
    program_id: &Pubkey,
    system_info: &AccountInfo<'a>,
    rent: &Rent,
) -> Result<(), ProgramError> {
    // Get PDA and assert it's the same as the requested account address
    let (account_address, bump_seed) =
        Pubkey::find_program_address(account_address_seeds, program_id);

    if account_address != *account_info.key {
        msg!(
            "Create account with PDA: {:?} was requested while PDA: {:?} was expected",
            account_info.key,
            account_address
        );
        return Err(ProgramError::InvalidSeeds);
    }

    let (serialized_data, account_size) = if let Some(max_size) = account_data.get_max_size() {
        (None, max_size)
    } else {
        let serialized_data = account_data.try_to_vec()?;
        let account_size = serialized_data.len();
        (Some(serialized_data), account_size)
    };

    let create_account_instruction = create_account(
        payer_info.key,
        account_info.key,
        rent.minimum_balance(account_size),
        account_size as u64,
        program_id,
    );

    let mut signers_seeds = account_address_seeds.to_vec();
    let bump = &[bump_seed];
    signers_seeds.push(bump);

    invoke_signed(
        &create_account_instruction,
        &[
            payer_info.clone(),
            account_info.clone(),
            system_info.clone(),
        ],
        &[&signers_seeds[..]],
    )?;

    if let Some(serialized_data) = serialized_data {
        account_info
            .data
            .borrow_mut()
            .copy_from_slice(&serialized_data);
    } else {
        account_data.serialize(&mut *account_info.data.borrow_mut())?;
    }

    Ok(())
}

/// Deserializes account and checks it's initialized and owned by the specified program
pub fn get_account_data<T: BorshDeserialize + IsInitialized>(
    account_info: &AccountInfo,
    owner_program_id: &Pubkey,
) -> Result<T, ProgramError> {
    if account_info.data_is_empty() {
        return Err(GovernanceError::AccountDoesNotExist.into());
    }
    if account_info.owner != owner_program_id {
        return Err(GovernanceError::InvalidAccountOwner.into());
    }

    let account: T = try_from_slice_unchecked(&account_info.data.borrow())?;
    if !account.is_initialized() {
        Err(ProgramError::UninitializedAccount)
    } else {
        Ok(account)
    }
}

/// Asserts the given account is not empty, owned by the given program and of the expected type
/// Note: The function assumes the account type T is stored as the first element in the account data
pub fn assert_is_valid_account<T: BorshDeserialize + PartialEq>(
    account_info: &AccountInfo,
    expected_account_type: T,
    owner_program_id: &Pubkey,
) -> Result<(), ProgramError> {
    if account_info.owner != owner_program_id {
        return Err(GovernanceError::InvalidAccountOwner.into());
    }

    if account_info.data_is_empty() {
        return Err(GovernanceError::AccountDoesNotExist.into());
    }

    let account_type: T = try_from_slice_unchecked(&account_info.data.borrow())?;

    if account_type != expected_account_type {
        return Err(GovernanceError::InvalidAccountType.into());
    };

    Ok(())
}

/// Disposes account by transferring its lamports to the beneficiary account and zeros its data
// After transaction completes the runtime would remove the account with no lamports
pub fn dispose_account(account_info: &AccountInfo, beneficiary_info: &AccountInfo) {
    let account_lamports = account_info.lamports();
    **account_info.lamports.borrow_mut() = 0;

    **beneficiary_info.lamports.borrow_mut() = beneficiary_info
        .lamports()
        .checked_add(account_lamports)
        .unwrap();

    let mut account_data = account_info.data.borrow_mut();

    account_data.fill(0);
}
