//! Program processor

pub mod process_create_realm;
pub mod process_deposit_governing_tokens;
pub mod process_set_vote_authority;
pub mod process_withdraw_governing_tokens;

use core::panic;

use crate::instruction::GovernanceInstruction;
use borsh::BorshDeserialize;

use process_create_realm::process_create_realm;
use process_deposit_governing_tokens::process_deposit_governing_tokens;
use process_set_vote_authority::process_set_vote_authority;
use process_withdraw_governing_tokens::process_withdraw_governing_tokens;
use solana_program::{
    account_info::AccountInfo, entrypoint::ProgramResult, msg, program_error::ProgramError,
    pubkey::Pubkey,
};

/// Processes an instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = GovernanceInstruction::try_from_slice(input)
        .map_err(|_| ProgramError::InvalidInstructionData)?;

    msg!("Instruction: {:?}", instruction);

    match instruction {
        GovernanceInstruction::CreateRealm { name } => {
            process_create_realm(program_id, accounts, name)
        }

        GovernanceInstruction::DepositGoverningTokens {} => {
            process_deposit_governing_tokens(program_id, accounts)
        }

        GovernanceInstruction::WithdrawGoverningTokens {} => {
            process_withdraw_governing_tokens(program_id, accounts)
        }

        GovernanceInstruction::SetVoteAuthority {
            realm,
            governing_token_mint,
            vote_authority,
        } => process_set_vote_authority(
            program_id,
            accounts,
            &realm,
            &governing_token_mint,
            &vote_authority,
        ),
        _ => todo!("Instruction not implemented yet"),
    }
}
