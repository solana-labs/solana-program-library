//! ProgramMetadata Account

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, clock::Slot, program_error::ProgramError,
    program_pack::IsInitialized, pubkey::Pubkey,
};
use spl_governance_tools::account::{get_account_data, AccountMaxSize};

use crate::state::enums::GovernanceAccountType;

/// Program metadata account. It stores information about the particular SPL-Governance program instance
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct ProgramMetadata {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// The slot when the metadata was captured
    pub updated_at: Slot,

    /// The version of the program
    /// Max 11 characters XXX.YYY.ZZZ
    pub version: String,

    /// Reserved
    pub reserved: [u8; 64],
}

impl AccountMaxSize for ProgramMetadata {
    fn get_max_size(&self) -> Option<usize> {
        Some(88)
    }
}

impl IsInitialized for ProgramMetadata {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::ProgramMetadata
    }
}

/// Returns ProgramMetadata PDA address
pub fn get_program_metadata_address(program_id: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&get_program_metadata_seeds(), program_id).0
}

/// Returns ProgramMetadata PDA seeds
pub fn get_program_metadata_seeds<'a>() -> [&'a [u8]; 1] {
    [b"metadata"]
}

/// Deserializes account and checks owner program
pub fn get_program_metadata_data(
    program_id: &Pubkey,
    program_metadata_info: &AccountInfo,
) -> Result<ProgramMetadata, ProgramError> {
    get_account_data::<ProgramMetadata>(program_id, program_metadata_info)
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_max_size() {
        let program_metadata_data = ProgramMetadata {
            account_type: GovernanceAccountType::TokenOwnerRecordV2,
            updated_at: 10,
            reserved: [0; 64],
            version: "111.122.155".to_string(),
        };

        let size = program_metadata_data.try_to_vec().unwrap().len();

        assert_eq!(program_metadata_data.get_max_size(), Some(size));
    }
}
