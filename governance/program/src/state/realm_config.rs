//! RealmConfig account

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

/// RealmConfig account
/// The account is an optional extension to RealmConfig stored on Realm account
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct RealmConfigAccount {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// The realm the config belong to
    pub realm: Pubkey,

    /// Addin providing voter weights for community token
    pub community_voter_weight_addin: Option<Pubkey>,

    /// Reserved for community max vote weight addin
    pub reserved_1: Option<Pubkey>,

    /// Reserved for council voter weight addin
    pub reserved_2: Option<Pubkey>,

    /// Reserved for council max vote weight addin
    pub reserved_3: Option<Pubkey>,

    /// Reserved
    pub reserved: [u8; 128],
}

impl AccountMaxSize for RealmConfigAccount {
    fn get_max_size(&self) -> Option<usize> {
        Some(1 + 32 + 33 * 4 + 128)
    }
}

impl IsInitialized for RealmConfigAccount {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::RealmConfig
    }
}

/// Deserializes RealmConfig account and checks owner program
pub fn get_realm_config_data(
    program_id: &Pubkey,
    realm_config_info: &AccountInfo,
) -> Result<RealmConfigAccount, ProgramError> {
    get_account_data::<RealmConfigAccount>(realm_config_info, program_id)
}

/// Deserializes RealmConfig account and checks the owner program and the Realm it belongs to
pub fn get_realm_config_data_for_realm(
    program_id: &Pubkey,
    realm_config_info: &AccountInfo,
    realm: &Pubkey,
) -> Result<RealmConfigAccount, ProgramError> {
    let realm_config_data = get_realm_config_data(program_id, realm_config_info)?;

    if realm_config_data.realm != *realm {
        return Err(GovernanceError::InvalidRealmConfigForRealm.into());
    }

    Ok(realm_config_data)
}

/// Returns RealmConfig PDA seeds
pub fn get_realm_config_address_seeds(realm: &Pubkey) -> [&[u8]; 2] {
    [b"realm-config", realm.as_ref()]
}

/// Returns RealmConfig PDA address
pub fn get_realm_config_address(program_id: &Pubkey, realm: &Pubkey) -> Pubkey {
    Pubkey::find_program_address(&get_realm_config_address_seeds(realm), program_id).0
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::state::{enums::GovernanceAccountType, realm_config::RealmConfigAccount};

    #[test]
    fn test_max_size() {
        let realm_config = RealmConfigAccount {
            account_type: GovernanceAccountType::Realm,
            realm: Pubkey::new_unique(),
            community_voter_weight_addin: Some(Pubkey::new_unique()),
            reserved_1: Some(Pubkey::new_unique()),
            reserved_2: Some(Pubkey::new_unique()),
            reserved_3: Some(Pubkey::new_unique()),
            reserved: [0; 128],
        };

        let size = realm_config.try_to_vec().unwrap().len();

        assert_eq!(realm_config.get_max_size(), Some(size));
    }
}
