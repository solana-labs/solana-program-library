//! Program state processor
use crate::{
    error::TimelockError,
    state::timelock_program::TimelockProgram,
    state::{
        custom_single_signer_timelock_transaction::{
            CustomSingleSignerTimelockTransaction, MAX_ACCOUNTS_ALLOWED,
        },
        timelock_set::TimelockSet,
    },
    utils::{
        assert_executing, assert_initialized, assert_same_version_as_program, execute,
        ExecuteParams,
    },
};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    clock::Clock,
    entrypoint::ProgramResult,
    instruction::Instruction,
    message::Message,
    msg,
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::Sysvar,
};

/// Execute an instruction
pub fn process_execute(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    number_of_extra_accounts: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let transaction_account_info = next_account_info(account_info_iter)?;
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let program_to_invoke_info = next_account_info(account_info_iter)?;
    let timelock_program_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?;
    let mut account_infos: Vec<AccountInfo> = vec![];
    if number_of_extra_accounts > (MAX_ACCOUNTS_ALLOWED - 2) as u8 {
        return Err(TimelockError::TooManyAccountsInInstruction.into());
    }
    for n in 0..number_of_extra_accounts {
        account_infos.push(next_account_info(account_info_iter)?.clone())
    }
    account_infos.push(program_to_invoke_info.clone());
    account_infos.push(timelock_program_authority_info.clone());

    let timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;
    let clock = &Clock::from_account_info(clock_info)?;

    // For now we assume all transactions are CustomSingleSignerTransactions even though
    // this will not always be the case...we need to solve that inheritance issue later.
    let mut transaction: CustomSingleSignerTimelockTransaction =
        assert_initialized(transaction_account_info)?;

    assert_same_version_as_program(&timelock_program, &timelock_set)?;
    assert_executing(&timelock_set)?;

    let (authority_key, bump_seed) =
        Pubkey::find_program_address(&[timelock_program_account_info.key.as_ref()], program_id);
    if timelock_program_authority_info.key != &authority_key {
        return Err(TimelockError::InvalidTimelockAuthority.into());
    }
    let authority_signer_seeds = &[timelock_program_account_info.key.as_ref(), &[bump_seed]];
    if transaction.executed == 1 {
        return Err(TimelockError::TimelockTransactionAlreadyExecuted.into());
    }

    if clock.slot < transaction.slot {
        return Err(TimelockError::TooEarlyToExecute.into());
    }

    let message: Message = match bincode::deserialize::<Message>(&transaction.instruction[0..transaction.instruction_end_index as usize + 1]) {
        Ok(val) => val,
        Err(_) => return Err(TimelockError::InstructionUnpackError.into()),
    };
    let serialized_instructions = message.serialize_instructions();
    let instruction: Instruction =
        match Message::deserialize_instruction(0, &serialized_instructions) {
            Ok(val) => val,
            Err(_) => return Err(TimelockError::InstructionUnpackError.into()),
        };
    //msg!("Data is {:?}", instruction.data);

    execute(ExecuteParams {
        instruction,
        authority_signer_seeds,
        account_infos,
    })?;

    transaction.executed = 1;

    CustomSingleSignerTimelockTransaction::pack(
        transaction.clone(),
        &mut transaction_account_info.data.borrow_mut(),
    )?;
    Ok(())
}
