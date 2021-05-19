//! General purpose bpf_loader utility functions

use bincode::deserialize;
use solana_program::{
    account_info::AccountInfo,
    bpf_loader_upgradeable,
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
    pubkey::Pubkey,
};

use serde_derive::{Deserialize, Serialize};

use num_derive::FromPrimitive;
use thiserror::Error;

/// SPL Token Tools errors  
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum BpfLoaderToolsError {
    /// Invalid ProgramData account address
    #[error("Invalid ProgramData account address")]
    InvalidProgramDataAccountAddress,

    /// Invalid ProgramData account data
    #[error("Invalid ProgramData account Data")]
    InvalidProgramDataAccountData,

    /// Provided upgrade authority doesn't match current program upgrade authority
    #[error("Provided upgrade authority doesn't match current program upgrade authority")]
    InvalidUpgradeAuthority,

    /// Current program upgrade authority must sign transaction
    #[error("Current program upgrade authority must sign transaction")]
    UpgradeAuthorityMustSign,

    /// Given program is not upgradable
    #[error("Given program is not upgradable")]
    ProgramNotUpgradable,
}

impl PrintProgramError for BpfLoaderToolsError {
    fn print<E>(&self) {
        msg!("BPF-LOADER-TOOLS-ERROR: {}", &self.to_string());
    }
}

impl From<BpfLoaderToolsError> for ProgramError {
    fn from(e: BpfLoaderToolsError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for BpfLoaderToolsError {
    fn type_of() -> &'static str {
        "BPF Loader Tools Error"
    }
}

/// Returns ProgramData account address for the given Program
pub fn get_program_data_address(program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[program.as_ref()], &bpf_loader_upgradeable::id()).0
}

/// Upgradeable loader account states.
/// Note: The struct is taken as is from solana-sdk which doesn't support bpf and can't be referenced from a program
#[derive(Debug, Serialize, Deserialize, PartialEq, Clone, Copy)]
pub enum UpgradeableLoaderState {
    /// Account is not initialized.
    Uninitialized,
    /// A Buffer account.
    Buffer {
        /// Authority address
        authority_address: Option<Pubkey>,
        // The raw program data follows this serialized structure in the
        // account's data.
    },
    /// An Program account.
    Program {
        /// Address of the ProgramData account.
        programdata_address: Pubkey,
    },
    /// A ProgramData account.
    ProgramData {
        /// Slot that the program was last modified.
        slot: u64,
        /// Address of the Program's upgrade authority.
        upgrade_authority_address: Option<Pubkey>,
        // The raw program data follows this serialized structure in the
        // account's data.
    },
}

/// Checks whether the expected program upgrade authority is the current upgrade authority of the program
/// If it's not then it asserts the current program upgrade authority  is a signer of the transaction
pub fn assert_program_upgrade_authority(
    expected_upgrade_authority: &Pubkey,
    program_address: &Pubkey,
    program_data_info: &AccountInfo,
    program_upgrade_authority_info: &AccountInfo,
) -> Result<(), ProgramError> {
    if program_data_info.owner != &bpf_loader_upgradeable::id() {
        return Err(ProgramError::IncorrectProgramId);
    }

    let program_data_address = get_program_data_address(program_address);

    if program_data_address != *program_data_info.key {
        return Err(BpfLoaderToolsError::InvalidProgramDataAccountAddress.into());
    }

    let upgrade_authority = match deserialize(&program_data_info.data.borrow())
        .map_err(|_| BpfLoaderToolsError::InvalidProgramDataAccountData)?
    {
        UpgradeableLoaderState::ProgramData {
            slot: _,
            upgrade_authority_address,
        } => upgrade_authority_address,
        _ => None,
    };

    match upgrade_authority {
        Some(upgrade_authority) => {
            if upgrade_authority != *expected_upgrade_authority {
                if upgrade_authority != *program_upgrade_authority_info.key {
                    return Err(BpfLoaderToolsError::InvalidUpgradeAuthority.into());
                }
                if !program_upgrade_authority_info.is_signer {
                    return Err(BpfLoaderToolsError::UpgradeAuthorityMustSign.into());
                }
            }
            Ok(())
        }
        None => Err(BpfLoaderToolsError::ProgramNotUpgradable.into()),
    }
}
