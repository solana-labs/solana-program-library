//! Program processor
mod process_deposit;
mod process_add_voter;
use process_deposit::*
use process_add_voter::*

use borsh::BorshDeserialize;
use spl_governance::{
    addins::voter_weight::{VoterWeightAccountType, VoterWeightRecord},
    state::token_owner_record::get_token_owner_record_data_for_realm_and_governing_mint,
};

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    msg,
    program_error::ProgramError,
    pubkey::Pubkey,
};
use spl_governance_tools::account::create_and_serialize_account;

use crate::instruction::VoterWeightAddinInstruction;

/// Processes an instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = VoterWeightAddinInstruction::try_from_slice(input)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    msg!("GOVERNANCE-VOTER-WEIGHT-INSTRUCTION: {:?}", instruction);

    match instruction {
        VoterWeightAddinInstruction::CreateVoteProfil {address, amount} => {
            process_add_voter(program_id, accounts, address, amount)
        }
        VoterWeightAddinInstruction::Deposit { amount } => {
            process_deposit(program_id, accounts, amount)
        }
        VoterWeightAddinInstruction::Withdraw {} => Ok(()),
        VoterWeightAddinInstruction::Revise {} => Ok(()),

    }
}
