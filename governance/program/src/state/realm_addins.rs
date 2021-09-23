//! Realm Addins account

use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::IsInitialized,
    pubkey::Pubkey,
};

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};

use crate::{
    error::GovernanceError,
    state::enums::GovernanceAccountType,
    tools::account::{get_account_data, AccountMaxSize},
};

/// Realm Addins account
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct RealmAddins {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// The realm the addins belong to
    pub realm: Pubkey,

    /// Addin providing voter weights for community token
    pub community_voter_weight: Option<Pubkey>,

    /// Reserved for future addins
    pub reserved_1: Option<Pubkey>,

    /// Reserved for future addins
    pub reserved_2: Option<Pubkey>,
}

impl AccountMaxSize for RealmAddins {
    fn get_max_size(&self) -> Option<usize> {
        Some(1 + 32 + 33 * 3)
    }
}

impl IsInitialized for RealmAddins {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::RealmAddins
    }
}

/// Deserializes RealmAddins account and checks owner program
pub fn get_realm_addins_data(
    program_id: &Pubkey,
    realm_addins_info: &AccountInfo,
) -> Result<RealmAddins, ProgramError> {
    get_account_data::<RealmAddins>(realm_addins_info, program_id)
}

/// Deserializes RealmAddins account and checks the owner program and the Realm it belongs to
pub fn get_realm_addins_data_for_realm(
    program_id: &Pubkey,
    realm_addins_info: &AccountInfo,
    realm: &Pubkey,
) -> Result<RealmAddins, ProgramError> {
    let realm_addins_data = get_realm_addins_data(program_id, realm_addins_info)?;

    if realm_addins_data.realm != *realm {
        return Err(GovernanceError::InvalidRealmAddinsForRealm.into());
    }

    Ok(realm_addins_data)
}

/// Returns RealmAddins PDA seeds
pub fn get_realm_addins_address_seeds(realm: &Pubkey) -> [&[u8]; 2] {
    [b"realm-addins", realm.as_ref()]
}

/// Returns RealmAddins PDA address
pub fn get_realm_addins_address(program_id: &Pubkey, realm: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&get_realm_addins_address_seeds(realm), program_id).0
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::state::{enums::GovernanceAccountType, realm_addins::RealmAddins};

    #[test]
    fn test_max_size() {
        let realm_addins = RealmAddins {
            account_type: GovernanceAccountType::Realm,
            realm: Pubkey::new_unique(),
            community_voter_weight: Some(Pubkey::new_unique()),
            reserved_1: Some(Pubkey::new_unique()),
            reserved_2: Some(Pubkey::new_unique()),
        };

        let size = realm_addins.try_to_vec().unwrap().len();

        assert_eq!(realm_addins.get_max_size(), Some(size));
    }
}
