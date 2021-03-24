//! Program state processor
use crate::{
    state::timelock_program::TimelockProgram,
    state::timelock_set::TimelockSet,
    state::timelock_state::TimelockState,
    utils::{
        assert_account_equiv, assert_draft, assert_initialized, assert_is_permissioned,
        assert_token_program_is_correct, assert_txn_in_state,
    },
};
use arrayref::array_mut_ref;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

/// Updates transaction slot on a txn
pub fn process_update_transaction_slot(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_slot: u64,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_txn_account_info = next_account_info(account_info_iter)?;
    let signatory_account_info = next_account_info(account_info_iter)?;
    let signatory_validation_account_info = next_account_info(account_info_iter)?;
    let timelock_state_account_info = next_account_info(account_info_iter)?;
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let timelock_authority_account_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;

    let timelock_state: TimelockState = assert_initialized(timelock_state_account_info)?;
    let timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;
    assert_token_program_is_correct(&timelock_program, token_program_account_info)?;
    assert_account_equiv(timelock_state_account_info, &timelock_set.state)?;
    assert_account_equiv(
        signatory_validation_account_info,
        &timelock_set.signatory_validation,
    )?;

    assert_draft(&timelock_state)?;
    assert_is_permissioned(
        program_id,
        signatory_account_info,
        signatory_validation_account_info,
        timelock_program_account_info,
        token_program_account_info,
        transfer_authority_info,
        timelock_authority_account_info,
    )?;
    assert_txn_in_state(&timelock_state, timelock_txn_account_info)?;

    // All transactions have slot as first byte, adjust it.
    let mut mutable_data = timelock_txn_account_info.data.borrow_mut();
    let original_slot_slice = array_mut_ref![mutable_data, 0, 8];
    original_slot_slice.copy_from_slice(&new_slot.to_le_bytes());

    Ok(())
}
