//! RealmConfig account
use std::slice::Iter;

use solana_program::account_info::next_account_info;

use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::IsInitialized,
    pubkey::Pubkey,
};

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use spl_governance_tools::account::{get_account_data, AccountMaxSize};

use crate::tools::structs::Reserved110;
use crate::{error::GovernanceError, state::enums::GovernanceAccountType};

use crate::state::realm::GoverningTokenConfigArgs;

use crate::state::realm::{RealmConfigArgs, RealmV2};

/// The type of the governing token defines:
/// 1) Who retains the authority over deposited tokens
/// 2) Which token instructions Deposit, Withdraw and Revoke (burn) are allowed
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum GoverningTokenType {
    /// Liquid token is a token which is fully liquid and the token owner retains full authority over it
    /// Deposit - Yes
    /// Withdraw - Yes  
    /// Revoke - No, Realm authority cannot revoke liquid tokens
    Liquid,

    /// Membership token is a token controlled by Realm authority
    /// Deposit - Yes, membership tokens can be deposited to gain governance power
    ///           The membership tokens are conventionally minted into the holding account to keep them out of members possession  
    /// Withdraw - No, after membership tokens are deposited they are no longer transferable and can't be withdrawn
    /// Revoke - Yes, Realm authority can Revoke (burn) membership tokens
    Membership,

    /// Dormant token is a token which is only a placeholder and its deposits are not accepted and not used for governance power within the Realm
    ///
    /// The Dormant token type is used when only a single voting population is operational. For example a Multisig starter DAO uses Council only
    /// and sets Community as Dormant to indicate its not utilized for any governance power.
    /// Once the starter DAO decides to decentralise then it can change the Community token to Liquid
    ///
    /// Note: When an external voter weight plugin which takes deposits of the token is used then the type should be set to Dormant
    /// to make the intention explicit
    ///
    /// Deposit - No, dormant tokens can't be deposited into the Realm
    /// Withdraw - Yes, tokens can still be withdrawn from Realm to support scenario where the config is changed while some tokens are still deposited
    /// Revoke - No, Realm authority cannot revoke dormant tokens
    Dormant,
}

#[allow(clippy::derivable_impls)]
impl Default for GoverningTokenType {
    fn default() -> Self {
        GoverningTokenType::Liquid
    }
}

/// GoverningTokenConfig specifies configuration for Realm governing token (Community or Council)
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, BorshSchema, Default)]
pub struct GoverningTokenConfig {
    /// Plugin providing voter weights for the governing token
    pub voter_weight_addin: Option<Pubkey>,

    /// Plugin providing max voter weight for the governing token
    pub max_voter_weight_addin: Option<Pubkey>,

    /// Governing token type
    pub token_type: GoverningTokenType,

    /// Reserved space for future versions
    pub reserved: [u8; 8],
}

/// RealmConfig account
/// The account is an optional extension to RealmConfig stored on Realm account
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct RealmConfigAccount {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// The realm the config belong to
    pub realm: Pubkey,

    /// Community token config
    pub community_token_config: GoverningTokenConfig,

    /// Council token config
    pub council_token_config: GoverningTokenConfig,

    /// Reserved
    pub reserved: Reserved110,
}

impl AccountMaxSize for RealmConfigAccount {
    fn get_max_size(&self) -> Option<usize> {
        Some(1 + 32 + 75 * 2 + 110)
    }
}

impl IsInitialized for RealmConfigAccount {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::RealmConfig
    }
}

impl RealmConfigAccount {
    /// Returns GoverningTokenConfig for the given governing_token_mint
    pub fn get_token_config(
        &self,
        realm_data: &RealmV2,
        governing_token_mint: &Pubkey,
    ) -> Result<&GoverningTokenConfig, ProgramError> {
        let token_config = if *governing_token_mint == realm_data.community_mint {
            &self.community_token_config
        } else if Some(*governing_token_mint) == realm_data.config.council_mint {
            &self.council_token_config
        } else {
            return Err(GovernanceError::InvalidGoverningTokenMint.into());
        };

        Ok(token_config)
    }

    /// Asserts the given governing token can be revoked
    pub fn assert_can_revoke_governing_token(
        &self,
        realm_data: &RealmV2,
        governing_token_mint: &Pubkey,
    ) -> Result<(), ProgramError> {
        let governing_token_type = &self
            .get_token_config(realm_data, governing_token_mint)?
            .token_type;

        match governing_token_type {
            GoverningTokenType::Membership => Ok(()),
            GoverningTokenType::Liquid | GoverningTokenType::Dormant => {
                Err(GovernanceError::CannotRevokeGoverningTokens.into())
            }
        }
    }

    /// Asserts the given governing token can be deposited
    pub fn assert_can_deposit_governing_token(
        &self,
        realm_data: &RealmV2,
        governing_token_mint: &Pubkey,
    ) -> Result<(), ProgramError> {
        let governing_token_type = &self
            .get_token_config(realm_data, governing_token_mint)?
            .token_type;

        match governing_token_type {
            GoverningTokenType::Membership | GoverningTokenType::Liquid => Ok(()),
            // Note: Preventing deposits of the Dormant type tokens is not a direct security concern
            // It only makes the intention of not using deposited tokens as governance power stronger
            GoverningTokenType::Dormant => Err(GovernanceError::CannotDepositDormantTokens.into()),
        }
    }

    /// Asserts the given governing token can be withdrawn
    pub fn assert_can_withdraw_governing_token(
        &self,
        realm_data: &RealmV2,
        governing_token_mint: &Pubkey,
    ) -> Result<(), ProgramError> {
        let governing_token_type = &self
            .get_token_config(realm_data, governing_token_mint)?
            .token_type;

        match governing_token_type {
            GoverningTokenType::Dormant | GoverningTokenType::Liquid => Ok(()),
            GoverningTokenType::Membership => {
                Err(GovernanceError::CannotWithdrawMembershipTokens.into())
            }
        }
    }

    /// Asserts the given RealmConfigArgs represent a valid Realm configuration change
    pub fn assert_can_change_config(
        &self,
        realm_config_args: &RealmConfigArgs,
    ) -> Result<(), ProgramError> {
        // Existing community token type can't be changed to Membership because it would
        // give the Realm authority the right to burn members tokens which should not be the case because the tokens belong to the members
        // On the other had for the Council token it's acceptable and in fact desired change because council tokens denote membership
        // which should be controlled by the Realm
        if self.community_token_config.token_type != GoverningTokenType::Membership
            && realm_config_args.community_token_config_args.token_type
                == GoverningTokenType::Membership
        {
            return Err(GovernanceError::CannotChangeCommunityTokenTypeToMembership.into());
        }

        Ok(())
    }
}

/// Deserializes RealmConfig account and checks owner program
pub fn get_realm_config_data(
    program_id: &Pubkey,
    realm_config_info: &AccountInfo,
) -> Result<RealmConfigAccount, ProgramError> {
    get_account_data::<RealmConfigAccount>(program_id, realm_config_info)
}

/// If the account exists then deserializes it into RealmConfigAccount struct and checks the owner program and the Realm it belongs to
/// If the account doesn't exist then it checks its address is derived from the given owner program and Realm and returns default RealmConfigAccount
pub fn get_realm_config_data_for_realm(
    program_id: &Pubkey,
    realm_config_info: &AccountInfo,
    realm: &Pubkey,
) -> Result<RealmConfigAccount, ProgramError> {
    let realm_config_data = if realm_config_info.data_is_empty() {
        // If RealmConfigAccount doesn't exist yet then validate its PDA
        // PDA validation is required because RealmConfigAccount might not exist for legacy Realms
        // and then its absence is used as default RealmConfigAccount value with no plugins and Liquid governance tokens
        let realm_config_address = get_realm_config_address(program_id, realm);

        if realm_config_address != *realm_config_info.key {
            return Err(GovernanceError::InvalidRealmConfigAddress.into());
        }

        RealmConfigAccount {
            account_type: GovernanceAccountType::RealmConfig,
            realm: *realm,
            community_token_config: GoverningTokenConfig::default(),
            council_token_config: GoverningTokenConfig::default(),
            reserved: Reserved110::default(),
        }
    } else {
        let realm_config_data = get_realm_config_data(program_id, realm_config_info)?;

        if realm_config_data.realm != *realm {
            return Err(GovernanceError::InvalidRealmConfigForRealm.into());
        }

        realm_config_data
    };

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
/// Resolves GoverningTokenConfig from GoverningTokenConfigArgs and instruction accounts
pub fn resolve_governing_token_config(
    account_info_iter: &mut Iter<AccountInfo>,
    governing_token_config_args: &GoverningTokenConfigArgs,
) -> Result<GoverningTokenConfig, ProgramError> {
    let voter_weight_addin = if governing_token_config_args.use_voter_weight_addin {
        let voter_weight_addin_info = next_account_info(account_info_iter)?;
        Some(*voter_weight_addin_info.key)
    } else {
        None
    };

    let max_voter_weight_addin = if governing_token_config_args.use_max_voter_weight_addin {
        let max_voter_weight_addin_info = next_account_info(account_info_iter)?;
        Some(*max_voter_weight_addin_info.key)
    } else {
        None
    };

    Ok(GoverningTokenConfig {
        voter_weight_addin,
        max_voter_weight_addin,
        token_type: governing_token_config_args.token_type.clone(),
        reserved: [0; 8],
    })
}

#[cfg(test)]
mod test {
    use super::*;
    use crate::state::{enums::GovernanceAccountType, realm_config::RealmConfigAccount};

    #[test]
    fn test_max_size() {
        let realm_config = RealmConfigAccount {
            account_type: GovernanceAccountType::RealmV2,
            realm: Pubkey::new_unique(),
            community_token_config: GoverningTokenConfig {
                voter_weight_addin: Some(Pubkey::new_unique()),
                max_voter_weight_addin: Some(Pubkey::new_unique()),
                token_type: GoverningTokenType::Liquid,
                reserved: [0; 8],
            },
            council_token_config: GoverningTokenConfig {
                voter_weight_addin: Some(Pubkey::new_unique()),
                max_voter_weight_addin: Some(Pubkey::new_unique()),
                token_type: GoverningTokenType::Liquid,
                reserved: [0; 8],
            },
            reserved: Reserved110::default(),
        };

        let size = realm_config.try_to_vec().unwrap().len();

        assert_eq!(realm_config.get_max_size(), Some(size));
    }
}
