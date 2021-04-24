//! Program state processor
use crate::{
    error::GovernanceError,
    state::governance::Governance,
    state::{
        custom_single_signer_transaction::{CustomSingleSignerTransaction, MAX_ACCOUNTS_ALLOWED},
        enums::ProposalStateStatus,
        governance::GOVERNANCE_LEN,
        proposal::Proposal,
        proposal_state::ProposalState,
    },
    utils::{assert_account_equiv, assert_executing, assert_initialized, execute, ExecuteParams},
    PROGRAM_AUTHORITY_SEED,
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

/// Execute an instruction
pub fn process_execute(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    number_of_extra_accounts: u8,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();
    let transaction_account_info = next_account_info(account_info_iter)?;
    let proposal_state_account_info = next_account_info(account_info_iter)?;
    let program_to_invoke_info = next_account_info(account_info_iter)?;
    let proposal_account_info = next_account_info(account_info_iter)?;
    let governance_account_info = next_account_info(account_info_iter)?;
    let clock_info = next_account_info(account_info_iter)?;

    let mut proposal_state: ProposalState = assert_initialized(proposal_state_account_info)?;
    let proposal: Proposal = assert_initialized(proposal_account_info)?;
    let governance: Governance = assert_initialized(governance_account_info)?;
    let clock = &Clock::from_account_info(clock_info)?;
    // For now we assume all transactions are CustomSingleSignerTransactions even though
    // this will not always be the case...we need to solve that inheritance issue later.
    let mut transaction: CustomSingleSignerTransaction =
        assert_initialized(transaction_account_info)?;

    let time_elapsed = match clock.slot.checked_sub(proposal_state.voting_ended_at) {
        Some(val) => val,
        None => return Err(GovernanceError::NumericalOverflow.into()),
    };

    if time_elapsed < transaction.slot {
        return Err(GovernanceError::TooEarlyToExecute.into());
    }

    assert_account_equiv(proposal_state_account_info, &proposal.state)?;
    assert_account_equiv(governance_account_info, &proposal.governance)?;

    let mut seeds = vec![PROGRAM_AUTHORITY_SEED, governance.program.as_ref()];

    let (governance_authority, bump_seed) = Pubkey::find_program_address(&seeds[..], program_id);
    let mut account_infos: Vec<AccountInfo> = vec![];
    if number_of_extra_accounts > (MAX_ACCOUNTS_ALLOWED - 2) as u8 {
        return Err(GovernanceError::TooManyAccountsInInstruction.into());
    }
    let mut added_authority = false;

    for _ in 0..number_of_extra_accounts {
        let next_account = next_account_info(account_info_iter)?.clone();
        if next_account.data_len() == GOVERNANCE_LEN {
            // You better be initialized, and if you are, you better at least be mine...
            let _nefarious_governance: Governance = assert_initialized(&next_account)?;
            assert_account_equiv(&next_account, &proposal.governance)?;
            added_authority = true;

            if next_account.key != &governance_authority {
                return Err(GovernanceError::InvalidGovernanceKey.into());
            }
        }
        account_infos.push(next_account);
    }

    account_infos.push(program_to_invoke_info.clone());

    if !added_authority {
        if governance_account_info.key != &governance_authority {
            return Err(GovernanceError::InvalidGovernanceKey.into());
        }
        account_infos.push(governance_account_info.clone());
    }

    assert_executing(&proposal_state)?;

    if transaction.executed == 1 {
        return Err(GovernanceError::ProposalTransactionAlreadyExecuted.into());
    }

    let message: Message = match bincode::deserialize::<Message>(
        &transaction.instruction[0..transaction.instruction_end_index as usize + 1],
    ) {
        Ok(val) => val,
        Err(_) => return Err(GovernanceError::InstructionUnpackError.into()),
    };
    let serialized_instructions = message.serialize_instructions();
    let instruction: Instruction =
        match Message::deserialize_instruction(0, &serialized_instructions) {
            Ok(val) => val,
            Err(_) => return Err(GovernanceError::InstructionUnpackError.into()),
        };

    let bump = &[bump_seed];
    seeds.push(bump);
    let authority_signer_seeds = &seeds[..];

    execute(ExecuteParams {
        instruction,
        authority_signer_seeds,
        account_infos,
    })?;

    transaction.executed = 1;

    CustomSingleSignerTransaction::pack(
        transaction,
        &mut transaction_account_info.data.borrow_mut(),
    )?;

    proposal_state.number_of_executed_transactions = match proposal_state
        .number_of_executed_transactions
        .checked_add(1)
    {
        Some(val) => val,
        None => return Err(GovernanceError::NumericalOverflow.into()),
    };

    if proposal_state.number_of_executed_transactions == proposal_state.number_of_transactions {
        proposal_state.status = ProposalStateStatus::Completed
    }

    ProposalState::pack(
        proposal_state,
        &mut proposal_state_account_info.data.borrow_mut(),
    )?;
    Ok(())
}
