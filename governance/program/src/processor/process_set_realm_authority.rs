//! Program state processor

use borsh::BorshSerialize;
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

use crate::{error::GovernanceError, state::realm::get_realm_data_for_authority};

/// Processes SetRealmAuthority instruction
pub fn process_set_realm_authority(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    new_realm_authority: Option<Pubkey>,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let realm_authority_info = next_account_info(account_info_iter)?; // 1

    let mut realm_data =
        get_realm_data_for_authority(program_id, realm_info, realm_authority_info.key)?;

    if !realm_authority_info.is_signer {
        return Err(GovernanceError::RealmAuthorityMustSign.into());
    }

    realm_data.authority = new_realm_authority;

    realm_data.serialize(&mut *realm_info.data.borrow_mut())?;

    Ok(())
}
