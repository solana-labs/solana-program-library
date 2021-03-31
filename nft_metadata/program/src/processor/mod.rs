pub mod process_create_nft_metadata_accounts;
pub mod process_init_nft_metadata_accounts;
pub mod process_update_nft_metadata_accounts;
use crate::instruction::NFTMetadataInstruction;
use process_create_nft_metadata_accounts::process_create_nft_metadata_accounts;
use process_init_nft_metadata_accounts::process_init_nft_metadata_accounts;
use process_update_nft_metadata_accounts::process_update_nft_metadata_accounts;
use solana_program::{account_info::AccountInfo, entrypoint::ProgramResult, msg, pubkey::Pubkey};

/// Processes an instruction
pub fn process_instruction(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    input: &[u8],
) -> ProgramResult {
    let instruction = NFTMetadataInstruction::unpack(input)?;
    match instruction {
        NFTMetadataInstruction::CreateNFTMetadataAccounts { name, symbol } => {
            msg!("Instruction: Create NFTMetadata Accounts");
            process_create_nft_metadata_accounts(program_id, accounts, name, symbol)
        }
        NFTMetadataInstruction::InitNFTMetadataAccounts {
            name,
            symbol,
            uri,
            category,
            creator,
        } => {
            msg!("Instruction: Init NFTMetadata Accounts");
            process_init_nft_metadata_accounts(
                program_id, accounts, name, symbol, uri, category, creator,
            )
        }
        NFTMetadataInstruction::UpdateNFTMetadataAccounts {
            uri,
            category,
            creator,
        } => {
            msg!("Instruction: Update NFTMetadata Accounts");
            process_update_nft_metadata_accounts(program_id, accounts, uri, category, creator)
        }
    }
}
