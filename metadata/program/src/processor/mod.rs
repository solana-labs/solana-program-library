pub mod process_create_metadata_accounts;
pub mod process_init_metadata_accounts;
pub mod process_update_metadata_accounts;
use crate::instruction::MetadataInstruction;
use process_create_metadata_accounts::process_create_metadata_accounts;
use process_init_metadata_accounts::process_init_metadata_accounts;
use process_update_metadata_accounts::process_update_metadata_accounts;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

/// Processes an instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = MetadataInstruction::unpack(input)?;
    match instruction {
        MetadataInstruction::CreateMetadataAccounts { name, symbol } => {
            msg!("Instruction: Create Metadata Accounts");
            process_create_metadata_accounts(program_id, accounts, name, symbol)
        }
        MetadataInstruction::InitMetadataAccounts { name, symbol, uri } => {
            msg!("Instruction: Init Metadata Accounts");
            process_init_metadata_accounts(program_id, accounts, name, symbol, uri)
        }
        MetadataInstruction::UpdateMetadataAccounts { uri } => {
            msg!("Instruction: Update Metadata Accounts");
            process_update_metadata_accounts(program_id, accounts, uri)
        }
    }
}
