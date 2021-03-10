//! Program state processor
use crate::{
    error::TimelockError,
    state::timelock_program::TimelockProgram,
    state::{
        custom_single_signer_timelock_transaction::{
            CustomSingleSignerTimelockTransaction,
            CUSTOM_SINGLE_SIGNER_TIMELOCK_TRANSACTION_VERSION, INSTRUCTION_LIMIT,
        },
        timelock_set::TimelockSet,
        timelock_state::TRANSACTION_SLOTS,
    },
    utils::{
        assert_draft, assert_initialized, assert_is_permissioned, assert_same_version_as_program,
        assert_uninitialized,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_pack::Pack,
    pubkey::Pubkey,
};

/// Create a new timelock txn
pub fn process_add_custom_single_signer_transaction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    slot: u64,
    instruction: [u8; INSTRUCTION_LIMIT],
    position: u8,
    instruction_end_index: u16,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let timelock_txn_account_info = next_account_info(account_info_iter)?;
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let signatory_account_info = next_account_info(account_info_iter)?;
    let signatory_validation_account_info = next_account_info(account_info_iter)?;
    let transfer_authority_info = next_account_info(account_info_iter)?;
    let timelock_mint_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;

    let mut timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;

    let mut timelock_txn: CustomSingleSignerTimelockTransaction =
        assert_uninitialized(timelock_txn_account_info)?;

    if position as usize >= TRANSACTION_SLOTS {
        return Err(TimelockError::TooHighPositionInTxnArrayError.into());
    }

    if instruction_end_index as usize >= INSTRUCTION_LIMIT as usize {
        return Err(TimelockError::InvalidInstructionEndIndex.into());
    }

    assert_same_version_as_program(&timelock_program, &timelock_set)?;
    assert_draft(&timelock_set)?;
    assert_is_permissioned(
        program_id,
        signatory_account_info,
        signatory_validation_account_info,
        timelock_program_account_info,
        token_program_account_info,
        transfer_authority_info,
        timelock_mint_authority_info,
    )?;

    timelock_txn.version = CUSTOM_SINGLE_SIGNER_TIMELOCK_TRANSACTION_VERSION;
    timelock_txn.slot = slot;
    timelock_txn.instruction = instruction;
    timelock_txn.instruction_end_index = instruction_end_index;
    timelock_set.state.timelock_transactions[position as usize] = *timelock_txn_account_info.key;

    TimelockSet::pack(
        timelock_set.clone(),
        &mut timelock_set_account_info.data.borrow_mut(),
    )?;

    CustomSingleSignerTimelockTransaction::pack(
        timelock_txn.clone(),
        &mut timelock_txn_account_info.data.borrow_mut(),
    )?;

    Ok(())
}
