//! General purpose bpf_loader_upgradeable utility functions

use solana_program::{
    account_info::AccountInfo,
    bpf_loader_upgradeable::{self, UpgradeableLoaderState},
    program::invoke,
    program_error::ProgramError,
    pubkey::Pubkey,
};

use bincode::deserialize;

use crate::error::GovernanceError;

/// Returns ProgramData account address for the given Program
pub fn get_program_data_address(program: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&[program.as_ref()], &bpf_loader_upgradeable::id()).0
}

/// Returns upgrade_authority from the given Upgradable Loader Account
pub fn get_program_upgrade_authority(
    upgradable_loader_state: &UpgradeableLoaderState,
) -> Result<Option<Pubkey>, ProgramError> {
    let upgrade_authority = match upgradable_loader_state {
        UpgradeableLoaderState::ProgramData {
            slot: _,
            upgrade_authority_address,
        } => *upgrade_authority_address,
        _ => return Err(ProgramError::InvalidAccountData),
    };

    Ok(upgrade_authority)
}

/// Sets new upgrade authority for the given upgradable program
pub fn set_program_upgrade_authority<'a>(
    program_address: &Pubkey,
    program_data_info: &AccountInfo<'a>,
    program_upgrade_authority_info: &AccountInfo<'a>,
    new_authority_info: &AccountInfo<'a>,
    bpf_upgrade_loader_info: &AccountInfo<'a>,
) -> Result<(), ProgramError> {
    let set_upgrade_authority_instruction = bpf_loader_upgradeable::set_upgrade_authority(
        program_address,
        &program_upgrade_authority_info.key,
        Some(&new_authority_info.key),
    );

    invoke(
        &set_upgrade_authority_instruction,
        &[
            program_data_info.clone(),
            program_upgrade_authority_info.clone(),
            bpf_upgrade_loader_info.clone(),
            new_authority_info.clone(),
        ],
    )
}

/// Asserts the program  is upgradable and its upgrade authority is a signer of the transaction
pub fn assert_program_upgrade_authority_is_signer(
    program_address: &Pubkey,
    program_data_info: &AccountInfo,
    program_upgrade_authority_info: &AccountInfo,
) -> Result<(), ProgramError> {
    if program_data_info.owner != &bpf_loader_upgradeable::id() {
        return Err(ProgramError::IncorrectProgramId);
    }
    let program_data_address = get_program_data_address(program_address);

    if program_data_address != *program_data_info.key {
        return Err(GovernanceError::InvalidProgramDataAccountAddress.into());
    }

    let upgrade_authority = match deserialize(&program_data_info.data.borrow())
        .map_err(|_| GovernanceError::InvalidProgramDataAccountData)?
    {
        UpgradeableLoaderState::ProgramData {
            slot: _,
            upgrade_authority_address,
        } => upgrade_authority_address,
        _ => None,
    };

    match upgrade_authority {
        Some(upgrade_authority) => {
            if upgrade_authority != *program_upgrade_authority_info.key {
                return Err(GovernanceError::InvalidUpgradeAuthority.into());
            }
            if !program_upgrade_authority_info.is_signer {
                return Err(GovernanceError::UpgradeAuthorityMustSign.into());
            }
        }
        None => return Err(GovernanceError::ProgramNotUpgradable.into()),
    }

    Ok(())
}
