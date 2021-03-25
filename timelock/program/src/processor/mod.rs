pub mod process_add_custom_single_signer_transaction;
pub mod process_add_signer;
pub mod process_create_empty_timelock_config;
pub mod process_delete_timelock_set;
pub mod process_deposit_source_tokens;
pub mod process_execute;
pub mod process_init_timelock_config;
pub mod process_init_timelock_program;
pub mod process_init_timelock_set;
pub mod process_remove_signer;
pub mod process_remove_transaction;
pub mod process_sign;
pub mod process_update_transaction_slot;
pub mod process_vote;
pub mod process_withdraw_voting_tokens;

use crate::instruction::TimelockInstruction;
use process_add_custom_single_signer_transaction::process_add_custom_single_signer_transaction;
use process_add_signer::process_add_signer;
use process_create_empty_timelock_config::process_create_empty_timelock_config;
use process_delete_timelock_set::process_delete_timelock_set;
use process_deposit_source_tokens::process_deposit_source_tokens;
use process_execute::process_execute;
use process_init_timelock_config::process_init_timelock_config;
use process_init_timelock_program::process_init_timelock_program;
use process_init_timelock_set::process_init_timelock_set;
use process_remove_signer::process_remove_signer;
use process_remove_transaction::process_remove_transaction;
use process_sign::process_sign;
use process_update_transaction_slot::process_update_transaction_slot;
use process_vote::process_vote;
use process_withdraw_voting_tokens::process_withdraw_voting_tokens;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

/// Processes an instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = TimelockInstruction::unpack(input)?;
    match instruction {
        TimelockInstruction::InitTimelockProgram => {
            msg!("Instruction: Init Timelock Program");
            process_init_timelock_program(program_id, accounts)
        }
        TimelockInstruction::InitTimelockSet { name, desc_link } => {
            msg!("Instruction: Init Timelock Set");
            process_init_timelock_set(program_id, accounts, name, desc_link)
        }
        TimelockInstruction::AddSigner => {
            msg!("Instruction: Add Signer");
            process_add_signer(program_id, accounts)
        }
        TimelockInstruction::RemoveSigner => {
            msg!("Instruction: Remove Signer");
            process_remove_signer(program_id, accounts)
        }
        TimelockInstruction::AddCustomSingleSignerTransaction {
            slot,
            instruction,
            position,
            instruction_end_index,
        } => process_add_custom_single_signer_transaction(
            program_id,
            accounts,
            slot,
            instruction,
            position,
            instruction_end_index,
        ),
        TimelockInstruction::RemoveTransaction => {
            msg!("Instruction: Remove Transaction");
            process_remove_transaction(program_id, accounts)
        }
        TimelockInstruction::UpdateTransactionSlot { slot } => {
            msg!("Instruction: Update Transaction Slot");
            process_update_transaction_slot(program_id, accounts, slot)
        }
        TimelockInstruction::DeleteTimelockSet => {
            msg!("Instruction: Delete Timelock Set");
            process_delete_timelock_set(program_id, accounts)
        }
        TimelockInstruction::Sign => {
            msg!("Instruction: Sign");
            process_sign(program_id, accounts)
        }
        TimelockInstruction::Vote {
            yes_voting_token_amount,
            no_voting_token_amount,
        } => {
            msg!("Instruction: Vote");
            process_vote(
                program_id,
                accounts,
                yes_voting_token_amount,
                no_voting_token_amount,
            )
        }
        TimelockInstruction::InitTimelockConfig {
            consensus_algorithm,
            execution_type,
            timelock_type,
            voting_entry_rule,
            minimum_slot_waiting_period,
            time_limit,
            name,
        } => {
            msg!("Instruction: Initialize Timelock Config");
            process_init_timelock_config(
                program_id,
                accounts,
                consensus_algorithm,
                execution_type,
                timelock_type,
                voting_entry_rule,
                minimum_slot_waiting_period,
                time_limit,
                name,
            )
        }
        TimelockInstruction::Ping => {
            msg!("Ping!");
            Ok(())
        }
        TimelockInstruction::Execute {
            number_of_extra_accounts,
        } => {
            msg!("Instruction: Execute");
            process_execute(program_id, accounts, number_of_extra_accounts)
        }
        TimelockInstruction::DepositSourceTokens {
            voting_token_amount,
        } => {
            msg!("Instruction: Deposit Source Tokens");
            process_deposit_source_tokens(program_id, accounts, voting_token_amount)
        }
        TimelockInstruction::WithdrawVotingTokens {
            voting_token_amount,
        } => {
            msg!("Instruction: Withdraw Voting Tokens");
            process_withdraw_voting_tokens(program_id, accounts, voting_token_amount)
        }
        TimelockInstruction::CreateEmptyTimelockConfig => {
            msg!("Instruction: Create Empty Timelock Config");
            process_create_empty_timelock_config(program_id, accounts)
        }
    }
}
