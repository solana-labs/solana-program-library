//! Program processor

mod process_add_signatory;
mod process_create_account_governance;
mod process_create_program_governance;
mod process_create_proposal;
mod process_create_realm;
mod process_deposit_governing_tokens;
mod process_remove_signatory;
mod process_set_governance_delegate;
mod process_sign_off_proposal;
mod process_withdraw_governing_tokens;

use crate::instruction::GovernanceInstruction;
use borsh::BorshDeserialize;

use process_add_signatory::*;
use process_create_account_governance::*;
use process_create_program_governance::*;
use process_create_proposal::*;
use process_create_realm::*;
use process_deposit_governing_tokens::*;
use process_remove_signatory::*;
use process_set_governance_delegate::*;
use process_sign_off_proposal::*;
use process_withdraw_governing_tokens::*;

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

    msg!("GOVERNANCE-INSTRUCTION: {:?}", instruction);

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

        GovernanceInstruction::SetGovernanceDelegate {
            new_governance_delegate,
        } => process_set_governance_delegate(accounts, &new_governance_delegate),
        GovernanceInstruction::CreateProgramGovernance {
            config,
            transfer_upgrade_authority,
        } => process_create_program_governance(
            program_id,
            accounts,
            config,
            transfer_upgrade_authority,
        ),
        GovernanceInstruction::CreateAccountGovernance { config } => {
            process_create_account_governance(program_id, accounts, config)
        }
        GovernanceInstruction::CreateProposal {
            name,
            description_link,
            governing_token_mint,
        } => process_create_proposal(
            program_id,
            accounts,
            name,
            description_link,
            governing_token_mint,
        ),
        GovernanceInstruction::AddSignatory { signatory } => {
            process_add_signatory(program_id, accounts, signatory)
        }
        GovernanceInstruction::RemoveSignatory { signatory } => {
            process_remove_signatory(program_id, accounts, signatory)
        }
        GovernanceInstruction::SignOffProposal {} => {
            process_sign_off_proposal(program_id, accounts)
        }
        _ => todo!("Instruction not implemented yet"),
    }
}
