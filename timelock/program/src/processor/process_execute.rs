//! Program state processor
use crate::{error::TimelockError, state::timelock_config::TimelockConfig, state::timelock_program::TimelockProgram, state::{custom_single_signer_timelock_transaction::{
            CustomSingleSignerTimelockTransaction, MAX_ACCOUNTS_ALLOWED,
        }, enums::TimelockType, timelock_config::TIMELOCK_CONFIG_LEN, timelock_set::TimelockSet}, utils::{ExecuteParams, assert_account_equiv, assert_executing, assert_initialized, execute}};
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
    let timelock_config_account_info = next_account_info(account_info_iter)?;
    let timelock_program_account_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?;
    let timelock_set: TimelockSet = assert_initialized(timelock_set_account_info)?;
    let timelock_config: TimelockConfig = assert_initialized(timelock_config_account_info)?;
    let clock = &Clock::from_account_info(clock_info)?;
    assert_account_equiv(timelock_config_account_info, &timelock_set.config)?;

    let seeds = &[timelock_program_account_info.key.as_ref(), timelock_config.governance_mint.as_ref(), timelock_config.program.as_ref() ];
    let (governance_authority, bump_seed) =
        Pubkey::find_program_address(seeds, program_id);

    let mut account_infos: Vec<AccountInfo> = vec![];
    if number_of_extra_accounts > (MAX_ACCOUNTS_ALLOWED - 2) as u8 {
        return Err(TimelockError::TooManyAccountsInInstruction.into());
    }

    let mut added_authority = false;

    for _ in 0..number_of_extra_accounts {
        let next_account = next_account_info(account_info_iter)?.clone();
        if next_account.data_len() == TIMELOCK_CONFIG_LEN {
            // You better be initialized, and if you are, you better at least be mine...
            let _nefarious_config: TimelockConfig = assert_initialized(&next_account)?;
            assert_account_equiv(&next_account, &timelock_set.config)?;

            if timelock_config.timelock_type != TimelockType::Governance {
                return Err(TimelockError::CannotUseTimelockAuthoritiesInExecute.into());
            }

            added_authority = true;

            if next_account.key != &governance_authority {
                return Err(TimelockError::InvalidTimelockConfigKey.into());
            }
        }
        account_infos.push(next_account);
    }
    account_infos.push(program_to_invoke_info.clone());

    if timelock_config.timelock_type == TimelockType::Governance && !added_authority {

        if timelock_config_account_info.key != &governance_authority {
            return Err(TimelockError::InvalidTimelockConfigKey.into());
        }
        account_infos.push(timelock_config_account_info.clone());
    }


    // For now we assume all transactions are CustomSingleSignerTransactions even though
    // this will not always be the case...we need to solve that inheritance issue later.
    let mut transaction: CustomSingleSignerTimelockTransaction =
        assert_initialized(transaction_account_info)?;

    assert_executing(&timelock_set)?;

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

    match timelock_config.timelock_type {
        TimelockType::Governance => {
            execute(ExecuteParams {
                instruction,
                authority_signer_seeds: &[timelock_config_account_info.key.as_ref(), &[bump_seed]],
                account_infos,
            })?;
            
        }
        TimelockType::Committee => {
            execute(ExecuteParams {
                instruction,
                authority_signer_seeds: &[],
                account_infos,
            })?;
        }
    }
    
    transaction.executed = 1;

    CustomSingleSignerTimelockTransaction::pack(
        transaction.clone(),
        &mut transaction_account_info.data.borrow_mut(),
    )?;
    Ok(())
}
