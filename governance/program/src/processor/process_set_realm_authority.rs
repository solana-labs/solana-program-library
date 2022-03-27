//! Program state processor

use solana_program::{
    account_info::{next_account_info, AccountInfo},
    entrypoint::ProgramResult,
    pubkey::Pubkey,
};

use crate::{
    error::GovernanceError,
    state::{
        governance::assert_governance_for_realm,
        realm::{get_realm_data_for_authority, SetRealmAuthorityAction},
    },
};

/// Processes SetRealmAuthority instruction
pub fn process_set_realm_authority(
    program_id: &Pubkey,
    accounts: &[AccountInfo],
    action: SetRealmAuthorityAction,
) -> ProgramResult {
    let account_info_iter = &mut accounts.iter();

    let realm_info = next_account_info(account_info_iter)?; // 0
    let realm_authority_info = next_account_info(account_info_iter)?; // 1

    let mut realm_data =
        get_realm_data_for_authority(program_id, realm_info, realm_authority_info.key)?;

    if !realm_authority_info.is_signer {
        return Err(GovernanceError::RealmAuthorityMustSign.into());
    }

    let new_realm_authority = match action {
        SetRealmAuthorityAction::SetUnchecked | SetRealmAuthorityAction::SetChecked => {
            let new_realm_authority_info = next_account_info(account_info_iter)?; // 2

            if action == SetRealmAuthorityAction::SetChecked {
                // Ensure the new realm authority is one of the governances from the realm
                assert_governance_for_realm(program_id, new_realm_authority_info, realm_info.key)?;
            }

            Some(*new_realm_authority_info.key)
        }
        SetRealmAuthorityAction::Remove => None,
    };

    realm_data.authority = new_realm_authority;

    realm_data.serialize(&mut *realm_info.data.borrow_mut())?;

    Ok(())
}
