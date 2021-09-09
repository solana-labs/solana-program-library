//! General purpose account utility functions

use borsh::BorshSerialize;
use solana_program::{
    account_info::AccountInfo, program::invoke, program_error::ProgramError, pubkey::Pubkey,
    rent::Rent, system_instruction::create_account, system_program, sysvar::Sysvar,
};
use spl_governance::tools::account::AccountMaxSize;

use crate::error::GovernanceChatError;

/// Creates a new account and serializes data into it using AccountMaxSize to determine the account's size
pub fn create_and_serialize_account<'a, T: BorshSerialize + AccountMaxSize>(
    payer_info: &AccountInfo<'a>,
    account_info: &AccountInfo<'a>,
    account_data: &T,
    program_id: &Pubkey,
    system_info: &AccountInfo<'a>,
) -> Result<(), ProgramError> {
    // Assert the account is not initialized yet
    if !(account_info.data_is_empty() && *account_info.owner == system_program::id()) {
        return Err(GovernanceChatError::AccountAlreadyInitialized.into());
    }

    let (serialized_data, account_size) = if let Some(max_size) = account_data.get_max_size() {
        (None, max_size)
    } else {
        let serialized_data = account_data.try_to_vec()?;
        let account_size = serialized_data.len();
        (Some(serialized_data), account_size)
    };

    let rent = Rent::get()?;

    let create_account_instruction = create_account(
        payer_info.key,
        account_info.key,
        rent.minimum_balance(account_size),
        account_size as u64,
        program_id,
    );

    invoke(
        &create_account_instruction,
        &[
            payer_info.clone(),
            account_info.clone(),
            system_info.clone(),
        ],
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
