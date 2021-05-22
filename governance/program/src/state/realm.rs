//! Realm Account

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::IsInitialized,
    pubkey::Pubkey,
};

use crate::{
    id,
    tools::account::{assert_is_valid_account, deserialize_account, AccountMaxSize},
    PROGRAM_AUTHORITY_SEED,
};

use super::enums::GovernanceAccountType;

/// Governance Realm Account
/// Account PDA seeds" ['governance', name]
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct Realm {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Community mint
    pub community_mint: Pubkey,

    /// Council mint
    pub council_mint: Option<Pubkey>,

    /// Governance Realm name
    pub name: String,
}

impl AccountMaxSize for Realm {}

impl IsInitialized for Realm {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::Realm
    }
}

/// Checks whether realm account exists, is initialized and  owned by Governance program
pub fn assert_is_valid_realm(realm_info: &AccountInfo) -> Result<(), ProgramError> {
    assert_is_valid_account(realm_info, GovernanceAccountType::Realm, &id())
}

/// Deserializes account and checks owner program
pub fn deserialize_realm(realm_info: &AccountInfo) -> Result<Realm, ProgramError> {
    deserialize_account::<Realm>(realm_info, &id())
}

/// Returns Realm PDA seeds
pub fn get_realm_address_seeds(name: &str) -> [&[u8]; 2] {
    [PROGRAM_AUTHORITY_SEED, &name.as_bytes()]
}

/// Returns Realm PDA address
pub fn get_realm_address(name: &str) -> Pubkey {
    Pubkey::find_program_address(&get_realm_address_seeds(&name), &id()).0
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
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_governing_token_holding_address_seeds(realm, governing_token_mint),
        &id(),
    )
    .0
}
