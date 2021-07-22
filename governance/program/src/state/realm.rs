//! Realm Account

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::IsInitialized,
    pubkey::Pubkey,
};

use crate::{
    error::GovernanceError,
    tools::account::{assert_is_valid_account, get_account_data, AccountMaxSize},
    PROGRAM_AUTHORITY_SEED,
};

use crate::state::enums::GovernanceAccountType;

/// Governance Realm Account
/// Account PDA seeds" ['governance', name]
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct Realm {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Community mint
    pub community_mint: Pubkey,

    /// Reserved space for future versions
    pub reserved: [u8; 8],

    /// Council mint
    pub council_mint: Option<Pubkey>,

    /// Realm authority. The authority must sign transactions which update the realm (ex. adding governance, setting council)
    /// The authority can be transferer to Realm Governance and hence make the Realm self governed through proposals
    /// Note: This field is not used yet. It's reserved for future versions
    pub authority: Option<Pubkey>,

    /// Governance Realm name
    pub name: String,
}

impl AccountMaxSize for Realm {
    fn get_max_size(&self) -> Option<usize> {
        Some(self.name.len() + 111)
    }
}

impl IsInitialized for Realm {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::Realm
    }
}

impl Realm {
    /// Asserts the given mint is either Community or Council mint of the Realm
    pub fn assert_is_valid_governing_token_mint(
        &self,
        governing_token_mint: &Pubkey,
    ) -> Result<(), ProgramError> {
        if self.community_mint == *governing_token_mint {
            return Ok(());
        }

        if self.council_mint == Some(*governing_token_mint) {
            return Ok(());
        }

        Err(GovernanceError::InvalidGoverningTokenMint.into())
    }
}

/// Checks whether realm account exists, is initialized and  owned by Governance program
pub fn assert_is_valid_realm(
    program_id: &Pubkey,
    realm_info: &AccountInfo,
) -> Result<(), ProgramError> {
    assert_is_valid_account(realm_info, GovernanceAccountType::Realm, program_id)
}

/// Deserializes account and checks owner program
pub fn get_realm_data(
    program_id: &Pubkey,
    realm_info: &AccountInfo,
) -> Result<Realm, ProgramError> {
    get_account_data::<Realm>(realm_info, program_id)
}

/// Deserializes Ream account and asserts the given governing_token_mint is either Community or Council mint of the Realm
pub fn get_realm_data_for_governing_token_mint(
    program_id: &Pubkey,
    realm_info: &AccountInfo,
    governing_token_mint: &Pubkey,
) -> Result<Realm, ProgramError> {
    let realm_data = get_realm_data(program_id, realm_info)?;

    realm_data.assert_is_valid_governing_token_mint(governing_token_mint)?;

    Ok(realm_data)
}

/// Returns Realm PDA seeds
pub fn get_realm_address_seeds(name: &str) -> [&[u8]; 2] {
    [PROGRAM_AUTHORITY_SEED, name.as_bytes()]
}

/// Returns Realm PDA address
pub fn get_realm_address(program_id: &Pubkey, name: &str) -> Pubkey {
    Pubkey::find_program_address(&get_realm_address_seeds(name), program_id).0
}

/// Returns Realm Token Holding PDA seeds
pub fn get_governing_token_holding_address_seeds<'a>(
    realm: &'a Pubkey,
    governing_token_mint: &'a Pubkey,
) -> [&'a [u8]; 3] {
    [
        PROGRAM_AUTHORITY_SEED,
        realm.as_ref(),
        governing_token_mint.as_ref(),
    ]
}

/// Returns Realm Token Holding PDA address
pub fn get_governing_token_holding_address(
    program_id: &Pubkey,
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_governing_token_holding_address_seeds(realm, governing_token_mint),
        program_id,
    )
    .0
}

#[cfg(test)]
mod test {

    use super::*;

    #[test]
    fn test_max_size() {
        let realm = Realm {
            account_type: GovernanceAccountType::Realm,
            community_mint: Pubkey::new_unique(),
            reserved: [0; 8],
            council_mint: Some(Pubkey::new_unique()),
            authority: Some(Pubkey::new_unique()),
            name: "test-realm".to_string(),
        };

        let size = realm.try_to_vec().unwrap().len();

        assert_eq!(realm.get_max_size(), Some(size));
    }
}
