//! Program state processor
use crate::{
    error::TimelockError,
    state::timelock_program::TimelockProgram,
    state::{
        custom_single_signer_timelock_transaction::CustomSingleSignerTimelockTransaction,
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
    program_pack::Pack,
    pubkey::Pubkey,
    sysvar::Sysvar,
};
extern crate base64;

/// Execute an instruction
pub fn process_execute(program_id: &Pubkey, accounts: &[AccountInfo]) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let transaction_account_info = next_account_info(account_info_iter)?;
    let timelock_set_account_info = next_account_info(account_info_iter)?;
    let program_to_invoke_info = next_account_info(account_info_iter)?;
    let timelock_program_authority_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;

    let timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_program: TimelockProgram = assert_initialized(timelock_program_account_info)?;
    let clock = &Clock::from_account_info(next_account_info(account_info_iter)?)?;
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

    // instructions is an array of u8s representing base64 characters
    // which we got by toBase64-ing an array of u8s representing serialized message.
    // So we need to take u8s array, turn it into base64 string, then decode that, then take that string
    // and turn it into an array of u8s. That array goes into bin deserialize.
    let base64_str = match std::str::from_utf8(
        &transaction.instruction[0..transaction.instruction_end_index as usize + 1],
    ) {
        Ok(val) => val,
        Err(_) => return Err(TimelockError::InstructionUnpackError.into()),
    };
    let decoded_msg_vec = base64::decode(base64_str).unwrap();
    let message: Message = match bincode::deserialize::<Message>(&decoded_msg_vec.as_slice()) {
        Ok(val) => val,
        Err(_) => return Err(TimelockError::InstructionUnpackError.into()),
    };
    let serialized_instructions = message.serialize_instructions();
    let instruction: Instruction =
        match Message::deserialize_instruction(0, &serialized_instructions) {
            Ok(val) => val,
            Err(_) => return Err(TimelockError::InstructionUnpackError.into()),
        };

    execute(ExecuteParams {
        instruction,
        program_to_invoke_info: program_to_invoke_info.clone(),
        timelock_program_authority_info: timelock_program_authority_info.clone(),
        authority_signer_seeds,
    })?;

    transaction.executed = 1;

    CustomSingleSignerTimelockTransaction::pack(
        transaction.clone(),
        &mut transaction_account_info.data.borrow_mut(),
    )?;
    Ok(())
}
