//! Program state processor

use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
    rent::Rent,
    sysvar::Sysvar,
};
use spl_governance_tools::account::create_and_serialize_account_signed;

use crate::state::{
    enums::GovernanceAccountType,
    program_metadata::{get_program_metadata_data, get_program_metadata_seeds, ProgramMetadata},
};

/// Processes UpdateProgramMetadata instruction
pub fn process_update_program_metadata(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let program_metadata_info = next_account_info(account_info_iter)?; // 0
    let payer_info = next_account_info(account_info_iter)?; // 1
    let system_info = next_account_info(account_info_iter)?; // 2
    let rent = Rent::get().unwrap();

    let version = 210; // 2.1

    if program_metadata_info.data_is_empty() {
        let program_metadata_data = ProgramMetadata {
            account_type: GovernanceAccountType::ProgramMetadata,
            version,
            reserved: [0; 128],
        };

        create_and_serialize_account_signed(
            payer_info,
            program_metadata_info,
            &program_metadata_data,
            &get_program_metadata_seeds(),
            program_id,
            system_info,
            &rent,
        )?;
    } else {
        let mut program_metadata_data =
            get_program_metadata_data(program_id, program_metadata_info)?;
        program_metadata_data.version = version;

        program_metadata_data.serialize(&mut *program_metadata_info.data.borrow_mut())?;
    }

    Ok(())
}
