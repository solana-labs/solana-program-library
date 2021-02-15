pub mod process_add_custom_single_signer_transaction;
pub mod process_add_signatory_mint;
pub mod process_add_signer;
pub mod process_add_voting_mint;
pub mod process_delete_timelock_set;
pub mod process_init_timelock_program;
pub mod process_init_timelock_set;
pub mod process_mint_voting_tokens;
pub mod process_remove_signer;
pub mod process_remove_transaction;
pub mod process_sign;
pub mod process_update_transaction_slot;
pub mod process_vote;

use crate::instruction::TimelockInstruction;
use process_add_custom_single_signer_transaction::process_add_custom_single_signer_transaction;
use process_add_signatory_mint::process_add_signatory_mint;
use process_add_signer::process_add_signer;
use process_add_voting_mint::process_add_voting_mint;
use process_delete_timelock_set::process_delete_timelock_set;
use process_init_timelock_program::process_init_timelock_program;
use process_init_timelock_set::process_init_timelock_set;
use process_mint_voting_tokens::process_mint_voting_tokens;
use process_remove_signer::process_remove_signer;
use process_remove_transaction::process_remove_transaction;
use process_sign::process_sign;
use process_update_transaction_slot::process_update_transaction_slot;
use process_vote::process_vote;
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
        TimelockInstruction::InitTimelockSet { config } => {
            msg!("Instruction: Init Timelock Set");
            process_init_timelock_set(program_id, accounts)
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
        } => {
            msg!("Instruction: Add Custom Single Signer Transaction");
            process_add_custom_single_signer_transaction(
                program_id,
                accounts,
                slot,
                instruction,
                position,
            )
        }
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
            voting_token_amount,
        } => {
            msg!("Instruction: Vote");
            process_vote(program_id, accounts, voting_token_amount)
        }
        TimelockInstruction::MintVotingTokens {
            voting_token_amount,
        } => {
            msg!("Instruction: Mint Voting Tokens");
            process_mint_voting_tokens(program_id, accounts, voting_token_amount)
        }
        TimelockInstruction::AddSignatoryMint => {
            msg!("Instruction: Adding Signatory Mint");
            process_add_signatory_mint(program_id, accounts)
        }
        TimelockInstruction::AddVotingMint => {
            msg!("Instruction: Adding Voting Mint");
            process_add_voting_mint(program_id, accounts)
        }
    }
}
