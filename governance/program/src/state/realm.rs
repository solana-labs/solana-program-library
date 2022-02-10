//! Realm Account

use borsh::maybestd::io::Write;
use std::slice::Iter;

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::{next_account_info, AccountInfo},
    borsh::try_from_slice_unchecked,
    program_error::ProgramError,
    program_pack::IsInitialized,
    pubkey::Pubkey,
};
use spl_governance_addin_api::voter_weight::VoterWeightAction;
use spl_governance_tools::account::{
    assert_is_valid_account_of_types, get_account_data, AccountMaxSize,
};

use crate::{
    error::GovernanceError,
    state::{
        enums::{GovernanceAccountType, MintMaxVoteWeightSource},
        legacy::RealmV1,
        token_owner_record::get_token_owner_record_data_for_realm,
    },
    PROGRAM_AUTHORITY_SEED,
};

/// Realm Config instruction args
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct RealmConfigArgs {
    /// Indicates whether council_mint should be used
    /// If yes then council_mint account must also be passed to the instruction
    pub use_council_mint: bool,

    /// Min number of community tokens required to create a governance
    pub min_community_weight_to_create_governance: u64,

    /// The source used for community mint max vote weight source
    pub community_mint_max_vote_weight_source: MintMaxVoteWeightSource,

    /// Indicates whether an external addin program should be used to provide community voters weights
    /// If yes then the voters weight program account must be passed to the instruction
    pub use_community_voter_weight_addin: bool,

    /// Indicates whether an external addin program should be used to provide max voters weight for the community mint
    /// If yes then the max voter weight program account must be passed to the instruction
    pub use_max_community_voter_weight_addin: bool,
}

/// SetRealmAuthority instruction action
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum SetRealmAuthorityAction {
    /// Sets realm authority without any checks
    /// Uncheck option allows to set the realm authority to non governance accounts
    SetUnchecked,

    /// Sets realm authority and checks the new new authority is one of the realm's governances
    // Note: This is not a security feature because governance creation is only gated with min_community_weight_to_create_governance
    //       The check is done to prevent scenarios where the authority could be accidentally set to a wrong or none existing account
    SetChecked,

    /// Removes realm authority
    Remove,
}

/// Realm Config defining Realm parameters.
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct RealmConfig {
    /// Indicates whether an external addin program should be used to provide voters weights for the community mint
    pub use_community_voter_weight_addin: bool,

    /// Indicates whether an external addin program should be used to provide max voter weight for the community mint
    pub use_max_community_voter_weight_addin: bool,

    /// Reserved space for future versions
    pub reserved: [u8; 6],

    /// Min number of voter's community weight required to create a governance
    pub min_community_weight_to_create_governance: u64,

    /// The source used for community mint max vote weight source
    pub community_mint_max_vote_weight_source: MintMaxVoteWeightSource,

    /// Optional council mint
    pub council_mint: Option<Pubkey>,
}

/// Governance Realm Account
/// Account PDA seeds" ['governance', name]
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct RealmV2 {
    /// Governance account type
    pub account_type: GovernanceAccountType,

    /// Community mint
    pub community_mint: Pubkey,

    /// Configuration of the Realm
    pub config: RealmConfig,

    /// Reserved space for future versions
    pub reserved: [u8; 6],

    /// The number of proposals in voting state in the Realm
    pub voting_proposal_count: u16,

    /// Realm authority. The authority must sign transactions which update the realm config
    /// The authority should be transferred to Realm Governance to make the Realm self governed through proposals
    pub authority: Option<Pubkey>,

    /// Governance Realm name
    pub name: String,

    /// Reserved space for versions v2 and onwards
    /// Note: This space won't be available to v1 accounts until runtime supports resizing
    pub reserved_v2: [u8; 128],
}

impl AccountMaxSize for RealmV2 {
    fn get_max_size(&self) -> Option<usize> {
        Some(self.name.len() + 264)
    }
}

impl IsInitialized for RealmV2 {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::RealmV2
    }
}

/// Checks if the given account type is on of the Realm account types of any version
pub fn is_realm_account_type(account_type: &GovernanceAccountType) -> bool {
    match account_type {
        GovernanceAccountType::RealmV1 | GovernanceAccountType::RealmV2 => true,
        GovernanceAccountType::GovernanceV2
        | GovernanceAccountType::ProgramGovernanceV2
        | GovernanceAccountType::MintGovernanceV2
        | GovernanceAccountType::TokenGovernanceV2
        | GovernanceAccountType::Uninitialized
        | GovernanceAccountType::RealmConfig
        | GovernanceAccountType::TokenOwnerRecordV1
        | GovernanceAccountType::TokenOwnerRecordV2
        | GovernanceAccountType::GovernanceV1
        | GovernanceAccountType::ProgramGovernanceV1
        | GovernanceAccountType::MintGovernanceV1
        | GovernanceAccountType::TokenGovernanceV1
        | GovernanceAccountType::ProposalV1
        | GovernanceAccountType::ProposalV2
        | GovernanceAccountType::SignatoryRecordV1
        | GovernanceAccountType::SignatoryRecordV2
        | GovernanceAccountType::ProposalInstructionV1
        | GovernanceAccountType::ProposalTransactionV2
        | GovernanceAccountType::VoteRecordV1
        | GovernanceAccountType::VoteRecordV2
        | GovernanceAccountType::ProgramMetadata => false,
    }
}

impl RealmV2 {
    /// Asserts the given mint is either Community or Council mint of the Realm
    pub fn assert_is_valid_governing_token_mint(
        &self,
        governing_token_mint: &Pubkey,
    ) -> Result<(), ProgramError> {
        if self.community_mint == *governing_token_mint {
            return Ok(());
        }

        if self.config.council_mint == Some(*governing_token_mint) {
            return Ok(());
        }

        Err(GovernanceError::InvalidGoverningTokenMint.into())
    }

    /// Asserts the given governing token mint and holding accounts are valid for the realm
    pub fn assert_is_valid_governing_token_mint_and_holding(
        &self,
        program_id: &Pubkey,
        realm: &Pubkey,
        governing_token_mint: &Pubkey,
        governing_token_holding: &Pubkey,
    ) -> Result<(), ProgramError> {
        self.assert_is_valid_governing_token_mint(governing_token_mint)?;

        let governing_token_holding_address =
            get_governing_token_holding_address(program_id, realm, governing_token_mint);

        if governing_token_holding_address != *governing_token_holding {
            return Err(GovernanceError::InvalidGoverningTokenHoldingAccount.into());
        }

        Ok(())
    }

    /// Asserts the given governing token can be deposited into the realm
    pub fn asset_governing_tokens_deposits_allowed(
        &self,
        governing_token_mint: &Pubkey,
    ) -> Result<(), ProgramError> {
        // If the deposit is for the community token and the realm uses community voter weight addin then panic
        if self.config.use_community_voter_weight_addin
            && self.community_mint == *governing_token_mint
        {
            return Err(GovernanceError::GoverningTokenDepositsNotAllowed.into());
        }

        Ok(())
    }

    /// Assert the given create authority can create governance
    pub fn assert_create_authority_can_create_governance(
        &self,
        program_id: &Pubkey,
        realm: &Pubkey,
        token_owner_record_info: &AccountInfo,
        create_authority_info: &AccountInfo,
        account_info_iter: &mut Iter<AccountInfo>,
    ) -> Result<(), ProgramError> {
        // Check if create_authority_info is realm_authority and if yes then it must signed the transaction
        if self.authority == Some(*create_authority_info.key) {
            return if !create_authority_info.is_signer {
                Err(GovernanceError::RealmAuthorityMustSign.into())
            } else {
                Ok(())
            };
        }

        // If realm_authority hasn't signed then check if TokenOwner or Delegate signed and can crate governance
        let token_owner_record_data =
            get_token_owner_record_data_for_realm(program_id, token_owner_record_info, realm)?;

        token_owner_record_data.assert_token_owner_or_delegate_is_signer(create_authority_info)?;

        let realm_config_info = next_account_info(account_info_iter)?;

        let voter_weight = token_owner_record_data.resolve_voter_weight(
            program_id,
            realm_config_info,
            account_info_iter,
            realm,
            self,
            VoterWeightAction::CreateGovernance,
            realm,
        )?;

        token_owner_record_data.assert_can_create_governance(self, voter_weight)?;

        Ok(())
    }

    /// Serializes account into the target buffer
    pub fn serialize<W: Write>(self, writer: &mut W) -> Result<(), ProgramError> {
        if self.account_type == GovernanceAccountType::RealmV2 {
            BorshSerialize::serialize(&self, writer)?
        } else if self.account_type == GovernanceAccountType::RealmV1 {
            // V1 account can't be resized and we have to translate it back to the original format

            // If reserved_v2 is used it must be individually asses for v1 backward compatibility impact
            if self.reserved_v2 != [0; 128] {
                panic!("Extended data not supported by RealmV1")
            }

            let realm_data_v1 = RealmV1 {
                account_type: self.account_type,
                community_mint: self.community_mint,
                config: self.config,
                reserved: self.reserved,
                voting_proposal_count: self.voting_proposal_count,
                authority: self.authority,
                name: self.name,
            };

            BorshSerialize::serialize(&realm_data_v1, writer)?;
        }

        Ok(())
    }
}

/// Checks whether the Realm account exists, is initialized and  owned by Governance program
pub fn assert_is_valid_realm(
    program_id: &Pubkey,
    realm_info: &AccountInfo,
) -> Result<(), ProgramError> {
    assert_is_valid_account_of_types(program_id, realm_info, is_realm_account_type)
}

/// Deserializes account and checks owner program
pub fn get_realm_data(
    program_id: &Pubkey,
    realm_info: &AccountInfo,
) -> Result<RealmV2, ProgramError> {
    let account_type: GovernanceAccountType = try_from_slice_unchecked(&realm_info.data.borrow())?;

    // If the account is V1 version then translate to V2
    if account_type == GovernanceAccountType::RealmV1 {
        let realm_data_v1 = get_account_data::<RealmV1>(program_id, realm_info)?;

        return Ok(RealmV2 {
            account_type,
            community_mint: realm_data_v1.community_mint,
            config: realm_data_v1.config,
            reserved: realm_data_v1.reserved,
            voting_proposal_count: realm_data_v1.voting_proposal_count,
            authority: realm_data_v1.authority,
            name: realm_data_v1.name,
            // Add the extra reserved_v2 padding
            reserved_v2: [0; 128],
        });
    }

    get_account_data::<RealmV2>(program_id, realm_info)
}

/// Deserializes account and checks the given authority is Realm's authority
pub fn get_realm_data_for_authority(
    program_id: &Pubkey,
    realm_info: &AccountInfo,
    realm_authority: &Pubkey,
) -> Result<RealmV2, ProgramError> {
    let realm_data = get_realm_data(program_id, realm_info)?;

    if realm_data.authority.is_none() {
        return Err(GovernanceError::RealmHasNoAuthority.into());
    }

    if realm_data.authority.unwrap() != *realm_authority {
        return Err(GovernanceError::InvalidAuthorityForRealm.into());
    }

    Ok(realm_data)
}

/// Deserializes Ream account and asserts the given governing_token_mint is either Community or Council mint of the Realm
pub fn get_realm_data_for_governing_token_mint(
    program_id: &Pubkey,
    realm_info: &AccountInfo,
    governing_token_mint: &Pubkey,
) -> Result<RealmV2, ProgramError> {
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

/// Asserts given realm config args are correct
pub fn assert_valid_realm_config_args(config_args: &RealmConfigArgs) -> Result<(), ProgramError> {
    match config_args.community_mint_max_vote_weight_source {
        MintMaxVoteWeightSource::SupplyFraction(fraction) => {
            if !(1..=MintMaxVoteWeightSource::SUPPLY_FRACTION_BASE).contains(&fraction) {
                return Err(GovernanceError::InvalidMaxVoteWeightSupplyFraction.into());
            }
        }
        MintMaxVoteWeightSource::Absolute(_) => {
            return Err(GovernanceError::MintMaxVoteWeightSourceNotSupported.into())
        }
    }

    Ok(())
}

#[cfg(test)]
mod test {

    use crate::instruction::GovernanceInstruction;
    use solana_program::borsh::try_from_slice_unchecked;

    use super::*;

    #[test]
    fn test_max_size() {
        let realm = RealmV2 {
            account_type: GovernanceAccountType::RealmV2,
            community_mint: Pubkey::new_unique(),
            reserved: [0; 6],

            authority: Some(Pubkey::new_unique()),
            name: "test-realm".to_string(),
            config: RealmConfig {
                council_mint: Some(Pubkey::new_unique()),
                use_community_voter_weight_addin: false,
                use_max_community_voter_weight_addin: false,
                reserved: [0; 6],
                community_mint_max_vote_weight_source: MintMaxVoteWeightSource::Absolute(100),
                min_community_weight_to_create_governance: 10,
            },

            voting_proposal_count: 0,
            reserved_v2: [0; 128],
        };

        let size = realm.try_to_vec().unwrap().len();

        assert_eq!(realm.get_max_size(), Some(size));
    }

    /// Realm Config instruction args
    #[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
    pub struct RealmConfigArgsV1 {
        /// Indicates whether council_mint should be used
        /// If yes then council_mint account must also be passed to the instruction
        pub use_council_mint: bool,

        /// Min number of community tokens required to create a governance
        pub min_community_weight_to_create_governance: u64,

        /// The source used for community mint max vote weight source
        pub community_mint_max_vote_weight_source: MintMaxVoteWeightSource,
    }

    /// Instructions supported by the Governance program
    #[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
    pub enum GovernanceInstructionV1 {
        /// Creates Governance Realm account which aggregates governances for given Community Mint and optional Council Mint
        CreateRealm {
            #[allow(dead_code)]
            /// UTF-8 encoded Governance Realm name
            name: String,

            #[allow(dead_code)]
            /// Realm config args     
            config_args: RealmConfigArgsV1,
        },

        /// Deposits governing tokens (Community or Council) to Governance Realm and establishes your voter weight to be used for voting within the Realm
        DepositGoverningTokens {
            /// The amount to deposit into the realm
            #[allow(dead_code)]
            amount: u64,
        },
    }

    #[test]
    fn test_deserialize_v1_create_realm_instruction_from_v2() {
        // Arrange
        let create_realm_ix_v2 = GovernanceInstruction::CreateRealm {
            name: "test-realm".to_string(),
            config_args: RealmConfigArgs {
                use_council_mint: true,
                min_community_weight_to_create_governance: 100,
                community_mint_max_vote_weight_source:
                    MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
                use_community_voter_weight_addin: false,
                use_max_community_voter_weight_addin: false,
            },
        };

        let mut create_realm_ix_data = vec![];
        create_realm_ix_v2
            .serialize(&mut create_realm_ix_data)
            .unwrap();

        // Act
        let create_realm_ix_v1: GovernanceInstructionV1 =
            try_from_slice_unchecked(&create_realm_ix_data).unwrap();

        // Assert
        if let GovernanceInstructionV1::CreateRealm { name, config_args } = create_realm_ix_v1 {
            assert_eq!("test-realm", name);
            assert_eq!(
                MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
                config_args.community_mint_max_vote_weight_source
            );
        } else {
            panic!("Can't deserialize v1 CreateRealm instruction from v2");
        }
    }
}
