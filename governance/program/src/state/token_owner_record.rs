//! Token Owner Record Account

use borsh::maybestd::io::Write;
use std::slice::Iter;

use crate::{
    addins::voter_weight::{
        assert_is_valid_voter_weight, get_voter_weight_record_data_for_token_owner_record,
    },
    error::GovernanceError,
    state::{
        enums::GovernanceAccountType, governance::GovernanceConfig, legacy::TokenOwnerRecordV1,
        realm::RealmV2,
    },
    PROGRAM_AUTHORITY_SEED,
};

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
};
use spl_governance_addin_api::voter_weight::VoterWeightAction;
use spl_governance_tools::account::{get_account_data, get_account_type, AccountMaxSize};

use crate::state::realm_config::RealmConfigAccount;

/// Governance Token Owner Record
/// Account PDA seeds: ['governance', realm, token_mint, token_owner ]
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct TokenOwnerRecordV2 {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// The Realm the TokenOwnerRecord belongs to
    pub realm: Pubkey,

    /// Governing Token Mint the TokenOwnerRecord holds deposit for
    pub governing_token_mint: Pubkey,

    /// The owner (either single or multisig) of the deposited governing SPL Tokens
    /// This is who can authorize a withdrawal of the tokens
    pub governing_token_owner: Pubkey,

    /// The amount of governing tokens deposited into the Realm
    /// This amount is the voter weight used when voting on proposals
    pub governing_token_deposit_amount: u64,

    /// The number of votes cast by TokenOwner but not relinquished yet
    /// Every time a vote is cast this number is increased and it's always decreased when relinquishing a vote regardless of the vote state
    pub unrelinquished_votes_count: u64,

    /// The number of outstanding proposals the TokenOwner currently owns
    /// The count is increased when TokenOwner creates a proposal
    /// and decreased  once it's either voted on (Succeeded or Defeated) or Cancelled
    /// By default it's restricted to 1 outstanding Proposal per token owner
    pub outstanding_proposal_count: u8,

    /// Version of the account layout
    /// Note: In future versions (>program V3) we should introduce GovernanceAccountType::TokenOwnerRecord(version:u8) as a way to version this account (and all other accounts too)
    /// It can't be done in program V3  because it would require to fetch another GovernanceAccountType by the UI and the RPC is already overloaded with all the existing types
    /// The new account type and versioning scheme can be introduced once we migrate UI to use indexer to fetch all the accounts
    /// Once the new versioning scheme is introduced this field can be migrated and removed
    ///
    /// The other issues which need to be addressed before we can cleanup the account versioning code:
    /// 1) Remove the specific governance accounts (ProgramGovernance, TokenGovernance, MintGovernance)
    ///    The only reason they exist is the UI which can't handle the generic use case for those assets
    /// 2) For account layout breaking changes all plugins would have to be upgraded
    /// 3) For account layout changes the Holaplex indexer would have to be upgraded
    /// 4) We should migrate the UI to use the indexer for fetching data and stop using getProgramAccounts
    /// 5) The UI would have to be upgraded to support account migration to the latest version
    /// 6) The client sdk is already messy because of the different program/account versions and it should be cleaned up before we add even more versions.
    pub version: u8,

    /// Reserved space for future versions
    pub reserved: [u8; 6],

    /// A single account that is allowed to operate governance with the deposited governing tokens
    /// It can be delegated to by the governing_token_owner or current governance_delegate
    pub governance_delegate: Option<Pubkey>,

    /// Reserved space for versions v2 and onwards
    /// Note: V1 accounts must be resized before using this space
    pub reserved_v2: [u8; 128],
}

/// The current version of TokenOwnerRecord account layout
/// Note: It's the version of the account layout and not the version of the program or the account type
///
/// program V1,V2 -> account layout version 0
/// program V3 -> account layout version 1
pub const TOKEN_OWNER_RECORD_LAYOUT_VERSION: u8 = 1;

impl AccountMaxSize for TokenOwnerRecordV2 {
    fn get_max_size(&self) -> Option<usize> {
        Some(282)
    }
}

impl IsInitialized for TokenOwnerRecordV2 {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::TokenOwnerRecordV2
    }
}

impl TokenOwnerRecordV2 {
    /// Checks whether the provided Governance Authority signed transaction
    pub fn assert_token_owner_or_delegate_is_signer(
        &self,
        governance_authority_info: &AccountInfo,
    ) -> Result<(), ProgramError> {
        if governance_authority_info.is_signer {
            if &self.governing_token_owner == governance_authority_info.key {
                return Ok(());
            }

            if let Some(governance_delegate) = self.governance_delegate {
                if &governance_delegate == governance_authority_info.key {
                    return Ok(());
                }
            };
        }

        Err(GovernanceError::GoverningTokenOwnerOrDelegateMustSign.into())
    }

    /// Asserts TokenOwner has enough tokens to be allowed to create proposal and doesn't have any outstanding proposals
    pub fn assert_can_create_proposal(
        &self,
        realm_data: &RealmV2,
        config: &GovernanceConfig,
        voter_weight: u64,
    ) -> Result<(), ProgramError> {
        let min_weight_to_create_proposal =
            if self.governing_token_mint == realm_data.community_mint {
                config.min_community_weight_to_create_proposal
            } else if Some(self.governing_token_mint) == realm_data.config.council_mint {
                config.min_council_weight_to_create_proposal
            } else {
                return Err(GovernanceError::InvalidGoverningTokenMint.into());
            };

        // If the weight threshold is set to u64::MAX then it indicates explicitly Disabled value
        // which should prevent any possibility of using it
        if min_weight_to_create_proposal == u64::MAX {
            return Err(GovernanceError::VoterWeightThresholdDisabled.into());
        }

        if voter_weight < min_weight_to_create_proposal {
            return Err(GovernanceError::NotEnoughTokensToCreateProposal.into());
        }

        // The number of outstanding proposals is currently restricted to 10
        // If there is a need to change it in the future then it should be added to realm or governance config
        if self.outstanding_proposal_count >= 10 {
            return Err(GovernanceError::TooManyOutstandingProposals.into());
        }

        Ok(())
    }

    /// Asserts TokenOwner has enough tokens to be allowed to create governance
    pub fn assert_can_create_governance(
        &self,
        realm_data: &RealmV2,
        voter_weight: u64,
    ) -> Result<(), ProgramError> {
        let min_weight_to_create_governance =
            if self.governing_token_mint == realm_data.community_mint {
                realm_data.config.min_community_weight_to_create_governance
            } else if Some(self.governing_token_mint) == realm_data.config.council_mint {
                // For council tokens it's enough to be in possession of any number of tokens
                1
            } else {
                return Err(GovernanceError::InvalidGoverningTokenMint.into());
            };

        // If the weight threshold is set to u64::MAX then it indicates explicitly Disabled value
        // which should prevent any possibility of using it
        if min_weight_to_create_governance == u64::MAX {
            return Err(GovernanceError::VoterWeightThresholdDisabled.into());
        }

        if voter_weight < min_weight_to_create_governance {
            return Err(GovernanceError::NotEnoughTokensToCreateGovernance.into());
        }

        Ok(())
    }

    /// Asserts TokenOwner can withdraw tokens from Realm
    pub fn assert_can_withdraw_governing_tokens(&self) -> Result<(), ProgramError> {
        if self.unrelinquished_votes_count > 0 {
            return Err(
                GovernanceError::AllVotesMustBeRelinquishedToWithdrawGoverningTokens.into(),
            );
        }

        if self.outstanding_proposal_count > 0 {
            return Err(
                GovernanceError::AllProposalsMustBeFinalisedToWithdrawGoverningTokens.into(),
            );
        }

        Ok(())
    }

    /// Decreases outstanding_proposal_count
    pub fn decrease_outstanding_proposal_count(&mut self) {
        // Previous versions didn't use the count and it can be already 0
        // TODO: Remove this check once all outstanding proposals on mainnet are resolved
        if self.outstanding_proposal_count != 0 {
            self.outstanding_proposal_count =
                self.outstanding_proposal_count.checked_sub(1).unwrap();
        }
    }

    /// Resolves voter's weight using either the amount deposited into the realm or weight provided by voter weight addin (if configured)
    #[allow(clippy::too_many_arguments)]
    pub fn resolve_voter_weight(
        &self,
        account_info_iter: &mut Iter<AccountInfo>,
        realm_data: &RealmV2,
        realm_config_data: &RealmConfigAccount,
        weight_action: VoterWeightAction,
        weight_action_target: &Pubkey,
    ) -> Result<u64, ProgramError> {
        // if the Realm is configured to use voter weight plugin for our governing_token_mint then use the externally provided voter_weight
        // instead of governing_token_deposit_amount
        if let Some(voter_weight_addin) = realm_config_data
            .get_token_config(realm_data, &self.governing_token_mint)?
            .voter_weight_addin
        {
            let voter_weight_record_info = next_account_info(account_info_iter)?;

            let voter_weight_record_data = get_voter_weight_record_data_for_token_owner_record(
                &voter_weight_addin,
                voter_weight_record_info,
                self,
            )?;

            assert_is_valid_voter_weight(
                &voter_weight_record_data,
                weight_action,
                weight_action_target,
            )?;

            Ok(voter_weight_record_data.voter_weight)
        } else {
            Ok(self.governing_token_deposit_amount)
        }
    }

    /// Serializes account into the target buffer
    pub fn serialize<W: Write>(self, writer: &mut W) -> Result<(), ProgramError> {
        if self.account_type == GovernanceAccountType::TokenOwnerRecordV2 {
            BorshSerialize::serialize(&self, writer)?
        } else if self.account_type == GovernanceAccountType::TokenOwnerRecordV1 {
            // V1 account can't be resized and we have to translate it back to the original format

            // If reserved_v2 is used it must be individually asses for v1 backward compatibility impact
            if self.reserved_v2 != [0; 128] {
                panic!("Extended data not supported by TokenOwnerRecordV1")
            }

            let token_owner_record_data_v1 = TokenOwnerRecordV1 {
                account_type: self.account_type,
                realm: self.realm,
                governing_token_mint: self.governing_token_mint,
                governing_token_owner: self.governing_token_owner,
                governing_token_deposit_amount: self.governing_token_deposit_amount,
                unrelinquished_votes_count: self.unrelinquished_votes_count,
                outstanding_proposal_count: self.outstanding_proposal_count,
                version: self.version,
                reserved: self.reserved,
                governance_delegate: self.governance_delegate,
            };

            BorshSerialize::serialize(&token_owner_record_data_v1, writer)?;
        }

        Ok(())
    }
}

/// Returns TokenOwnerRecord PDA address
pub fn get_token_owner_record_address(
    program_id: &Pubkey,
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
    governing_token_owner: &Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_token_owner_record_address_seeds(realm, governing_token_mint, governing_token_owner),
        program_id,
    )
    .0
}

/// Returns TokenOwnerRecord PDA seeds
pub fn get_token_owner_record_address_seeds<'a>(
    realm: &'a Pubkey,
    governing_token_mint: &'a Pubkey,
    governing_token_owner: &'a Pubkey,
) -> [&'a [u8]; 4] {
    [
        PROGRAM_AUTHORITY_SEED,
        realm.as_ref(),
        governing_token_mint.as_ref(),
        governing_token_owner.as_ref(),
    ]
}

/// Deserializes TokenOwnerRecord account and checks owner program
pub fn get_token_owner_record_data(
    program_id: &Pubkey,
    token_owner_record_info: &AccountInfo,
) -> Result<TokenOwnerRecordV2, ProgramError> {
    let account_type: GovernanceAccountType =
        get_account_type(program_id, token_owner_record_info)?;

    // If the account is V1 version then translate to V2
    let mut token_owner_record_data = if account_type == GovernanceAccountType::TokenOwnerRecordV1 {
        let token_owner_record_data_v1 =
            get_account_data::<TokenOwnerRecordV1>(program_id, token_owner_record_info)?;

        TokenOwnerRecordV2 {
            account_type,
            realm: token_owner_record_data_v1.realm,
            governing_token_mint: token_owner_record_data_v1.governing_token_mint,
            governing_token_owner: token_owner_record_data_v1.governing_token_owner,
            governing_token_deposit_amount: token_owner_record_data_v1
                .governing_token_deposit_amount,
            unrelinquished_votes_count: token_owner_record_data_v1.unrelinquished_votes_count,
            outstanding_proposal_count: token_owner_record_data_v1.outstanding_proposal_count,
            version: token_owner_record_data_v1.version,
            reserved: token_owner_record_data_v1.reserved,
            governance_delegate: token_owner_record_data_v1.governance_delegate,

            // Add the extra reserved_v2 padding
            reserved_v2: [0; 128],
        }
    } else {
        get_account_data::<TokenOwnerRecordV2>(program_id, token_owner_record_info)?
    };

    // If the deserialized account uses the old account layout indicated by the version value then migrate the data to version 1
    if token_owner_record_data.version < 1 {
        token_owner_record_data.version = 1;

        // In previous versions unrelinquished_votes_count was u32 followed by total_votes_count:u32
        // In program V3 unrelinquished_votes_count was changed to u64 by extending it into the space previously used by total_votes_count:u32
        // Since total_votes_count could have some value we have to zero the upper 4 bytes of unrelinquished_votes_count
        token_owner_record_data.unrelinquished_votes_count &= u32::MAX as u64;
    }

    Ok(token_owner_record_data)
}

/// Deserializes TokenOwnerRecord account and checks its PDA against the provided seeds
pub fn get_token_owner_record_data_for_seeds(
    program_id: &Pubkey,
    token_owner_record_info: &AccountInfo,
    token_owner_record_seeds: &[&[u8]],
) -> Result<TokenOwnerRecordV2, ProgramError> {
    let (token_owner_record_address, _) =
        Pubkey::find_program_address(token_owner_record_seeds, program_id);

    if token_owner_record_address != *token_owner_record_info.key {
        return Err(GovernanceError::InvalidTokenOwnerRecordAccountAddress.into());
    }

    get_token_owner_record_data(program_id, token_owner_record_info)
}

/// Deserializes TokenOwnerRecord account and asserts it belongs to the given realm
pub fn get_token_owner_record_data_for_realm(
    program_id: &Pubkey,
    token_owner_record_info: &AccountInfo,
    realm: &Pubkey,
) -> Result<TokenOwnerRecordV2, ProgramError> {
    let token_owner_record_data = get_token_owner_record_data(program_id, token_owner_record_info)?;

    if token_owner_record_data.realm != *realm {
        return Err(GovernanceError::InvalidRealmForTokenOwnerRecord.into());
    }

    Ok(token_owner_record_data)
}

/// Deserializes TokenOwnerRecord account and  asserts it belongs to the given realm and is for the given governing mint
pub fn get_token_owner_record_data_for_realm_and_governing_mint(
    program_id: &Pubkey,
    token_owner_record_info: &AccountInfo,
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
) -> Result<TokenOwnerRecordV2, ProgramError> {
    let token_owner_record_data =
        get_token_owner_record_data_for_realm(program_id, token_owner_record_info, realm)?;

    if token_owner_record_data.governing_token_mint != *governing_token_mint {
        return Err(GovernanceError::InvalidGoverningMintForTokenOwnerRecord.into());
    }

    Ok(token_owner_record_data)
}

///  Deserializes TokenOwnerRecord account and checks its address is the give proposal_owner
pub fn get_token_owner_record_data_for_proposal_owner(
    program_id: &Pubkey,
    token_owner_record_info: &AccountInfo,
    proposal_owner: &Pubkey,
) -> Result<TokenOwnerRecordV2, ProgramError> {
    if token_owner_record_info.key != proposal_owner {
        return Err(GovernanceError::InvalidProposalOwnerAccount.into());
    }

    get_token_owner_record_data(program_id, token_owner_record_info)
}

#[cfg(test)]
mod test {
    use solana_program::{borsh::get_packed_len, stake_history::Epoch};

    use super::*;

    fn create_test_token_owner_record() -> TokenOwnerRecordV2 {
        TokenOwnerRecordV2 {
            account_type: GovernanceAccountType::TokenOwnerRecordV2,
            realm: Pubkey::new_unique(),
            governing_token_mint: Pubkey::new_unique(),
            governing_token_owner: Pubkey::new_unique(),
            governing_token_deposit_amount: 10,
            governance_delegate: Some(Pubkey::new_unique()),
            unrelinquished_votes_count: 1,
            outstanding_proposal_count: 1,
            version: 1,
            reserved: [0; 6],
            reserved_v2: [0; 128],
        }
    }

    fn create_test_program_v1_token_owner_record() -> TokenOwnerRecordV1 {
        TokenOwnerRecordV1 {
            account_type: GovernanceAccountType::TokenOwnerRecordV1,
            realm: Pubkey::new_unique(),
            governing_token_mint: Pubkey::new_unique(),
            governing_token_owner: Pubkey::new_unique(),
            governing_token_deposit_amount: 10,
            governance_delegate: Some(Pubkey::new_unique()),
            unrelinquished_votes_count: 1,
            outstanding_proposal_count: 1,
            version: 0,
            reserved: [0; 6],
        }
    }

    #[test]
    fn test_max_size() {
        // Arrange
        let token_owner_record = create_test_token_owner_record();

        // Act
        let size = get_packed_len::<TokenOwnerRecordV2>();

        // Assert
        assert_eq!(token_owner_record.get_max_size(), Some(size));
    }

    #[test]
    fn test_program_v1_token_owner_record_size() {
        // Arrange
        let governance = create_test_program_v1_token_owner_record();

        // Act
        let size = governance.try_to_vec().unwrap().len();

        // Assert
        assert_eq!(154, size);
    }

    /// Legacy TokenOwnerRecord for program V1 and V2 accounts with outstanding_proposal_count and without version
    #[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, BorshSchema)]
    pub struct LegacyTokenOwnerRecord {
        pub account_type: GovernanceAccountType,

        pub realm: Pubkey,

        pub governing_token_mint: Pubkey,

        pub governing_token_owner: Pubkey,

        pub governing_token_deposit_amount: u64,

        /// Legacy u32 field. Changed to u64 in program V3
        pub unrelinquished_votes_count: u32,

        /// Legacy field. Removed in program V3
        pub total_votes_count: u32,

        pub outstanding_proposal_count: u8,

        pub reserved: [u8; 7],

        pub governance_delegate: Option<Pubkey>,

        pub reserved_v2: [u8; 128],
    }

    #[test]
    fn test_migrate_token_owner_record_from_legacy_data_to_program_v3() {
        // Arrange
        let legacy_token_owner_record = LegacyTokenOwnerRecord {
            account_type: GovernanceAccountType::TokenOwnerRecordV2,
            realm: Pubkey::new_unique(),
            governing_token_mint: Pubkey::new_unique(),
            governing_token_owner: Pubkey::new_unique(),
            governing_token_deposit_amount: 10,
            unrelinquished_votes_count: 10,
            // Set total_votes_count which should be trimmed in program V3 version
            total_votes_count: 100,
            outstanding_proposal_count: 1,
            reserved: [0; 7],
            governance_delegate: Some(Pubkey::new_unique()),
            reserved_v2: [0; 128],
        };

        let mut legacy_data = vec![];
        legacy_token_owner_record
            .serialize(&mut legacy_data)
            .unwrap();

        let program_id = Pubkey::new_unique();

        let info_key = Pubkey::new_unique();
        let mut lamports = 10u64;

        let legacy_account_info = AccountInfo::new(
            &info_key,
            false,
            false,
            &mut lamports,
            &mut legacy_data[..],
            &program_id,
            false,
            Epoch::default(),
        );

        // Act
        let token_owner_record_program_v3 =
            get_token_owner_record_data(&program_id, &legacy_account_info).unwrap();

        // Assert
        assert_eq!(token_owner_record_program_v3.unrelinquished_votes_count, 10);
        assert_eq!(
            token_owner_record_program_v3.version,
            TOKEN_OWNER_RECORD_LAYOUT_VERSION
        );
    }
}
