//! Program state processor
use crate::{
    state::timelock_program::TimelockProgram,
    state::{
        custom_single_signer_timelock_transaction::INSTRUCTION_LIMIT, timelock_set::TimelockSet,
    },
    utils::{
        assert_draft, assert_initialized, assert_is_signatory, assert_same_version_as_program,
        assert_token_program_is_correct, spl_token_mint_to, TokenMintToParams,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

/// Create a new timelock program
pub fn process_add_custom_single_signer_transaction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    slot: u64,
    instruction: [u8; INSTRUCTION_LIMIT],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let timelock_txn_account_info = next_account_info(account_info_iter)?;
    let signatory_account_info = next_account_info(account_info_iter)?;
    let signatory_validation_account_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let token_program_account_info = next_account_info(account_info_iter)?;

    let timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;

    assert_same_version_as_program(&timelock_program, &timelock_set)?;
    assert_draft(&timelock_set)?;
    assert_is_signatory(
        signatory_account_info,
        signatory_validation_account_info,
        timelock_program_account_info,
        token_program_account_info,
    )?;

    Ok(())
}
