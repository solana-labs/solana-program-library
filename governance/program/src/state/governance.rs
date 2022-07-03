//! Governance Account
use borsh::maybestd::io::Write;

use crate::{
    error::GovernanceError,
    state::{
        enums::{GovernanceAccountType, VoteThreshold, VoteTipping},
        legacy::{is_governance_v1_account_type, GovernanceV1},
        realm::{assert_is_valid_realm, RealmV2},
        vote_record::VoteKind,
    },
};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, borsh::try_from_slice_unchecked, program_error::ProgramError,
    program_pack::IsInitialized, pubkey::Pubkey,
};
use spl_governance_tools::{
    account::{assert_is_valid_account_of_types, get_account_data, AccountMaxSize},
    error::GovernanceToolsError,
};

/// Governance config
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct GovernanceConfig {
    /// The type of the vote threshold used for community vote
    /// Note: In the current version only YesVotePercentage and Disabled thresholds are supported
    pub community_vote_threshold: VoteThreshold,

    /// Minimum community weight a governance token owner must possess to be able to create a proposal
    pub min_community_weight_to_create_proposal: u64,

    /// Minimum waiting time in seconds for a transaction to be executed after proposal is voted on
    pub min_transaction_hold_up_time: u32,

    /// Time limit in seconds for proposal to be open for voting
    pub max_voting_time: u32,

    /// Conditions under which a vote will complete early
    pub vote_tipping: VoteTipping,

    /// The type of the vote threshold used for council vote
    /// Note: In the current version only YesVotePercentage and Disabled thresholds are supported
    pub council_vote_threshold: VoteThreshold,

    /// The threshold for Council Veto votes
    pub council_veto_vote_threshold: VoteThreshold,

    /// Minimum council weight a governance token owner must possess to be able to create a proposal
    pub min_council_weight_to_create_proposal: u64,
    //
    // The threshold for Community Veto votes
    // Note: Community Veto vote is not supported in the current version
    // In order to use this threshold the space from GovernanceV2.reserved must be taken to expand GovernanceConfig size
    // pub community_veto_vote_threshold: VoteThreshold,
}

/// Governance Account
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct GovernanceV2 {
    /// Account type. It can be Uninitialized, Governance, ProgramGovernance, TokenGovernance or MintGovernance
    pub account_type: GovernanceAccountType,

    /// Governance Realm
    pub realm: Pubkey,

    /// Account governed by this Governance and/or PDA identity seed
    /// It can be Program account, Mint account, Token account or any other account
    ///
    /// Note: The account doesn't have to exist. In that case the field is only a PDA seed
    ///
    /// Note: Setting governed_account doesn't give any authority over the governed account
    /// The relevant authorities for specific account types must still be transferred to the Governance PDA
    /// Ex: mint_authority/freeze_authority for a Mint account
    /// or upgrade_authority for a Program account should be transferred to the Governance PDA
    pub governed_account: Pubkey,

    /// Running count of proposals
    pub proposals_count: u32,

    /// Governance config
    pub config: GovernanceConfig,

    /// Reserved space for future versions
    pub reserved: [u8; 6],

    /// The number of proposals in voting state in the Governance
    pub voting_proposal_count: u16,

    /// Reserved space for versions v2 and onwards
    /// Note: This space won't be available to v1 accounts until runtime supports resizing
    pub reserved_v2: [u8; 128],
}

impl AccountMaxSize for GovernanceV2 {}

/// Checks if the given account type is one of the Governance V2 account types
pub fn is_governance_v2_account_type(account_type: &GovernanceAccountType) -> bool {
    match account_type {
        GovernanceAccountType::GovernanceV2
        | GovernanceAccountType::ProgramGovernanceV2
        | GovernanceAccountType::MintGovernanceV2
        | GovernanceAccountType::TokenGovernanceV2 => true,
        GovernanceAccountType::Uninitialized
        | GovernanceAccountType::RealmV1
        | GovernanceAccountType::RealmV2
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

/// Checks if the given account type is on of the Governance account types of any version
pub fn is_governance_account_type(account_type: &GovernanceAccountType) -> bool {
    is_governance_v1_account_type(account_type) || is_governance_v2_account_type(account_type)
}

impl IsInitialized for GovernanceV2 {
    fn is_initialized(&self) -> bool {
        is_governance_v2_account_type(&self.account_type)
    }
}

impl GovernanceV2 {
    /// Returns Governance PDA seeds
    pub fn get_governance_address_seeds(&self) -> Result<[&[u8]; 3], ProgramError> {
        let seeds = match self.account_type {
            GovernanceAccountType::GovernanceV1 | GovernanceAccountType::GovernanceV2 => {
                get_governance_address_seeds(&self.realm, &self.governed_account)
            }
            GovernanceAccountType::ProgramGovernanceV1
            | GovernanceAccountType::ProgramGovernanceV2 => {
                get_program_governance_address_seeds(&self.realm, &self.governed_account)
            }
            GovernanceAccountType::MintGovernanceV1 | GovernanceAccountType::MintGovernanceV2 => {
                get_mint_governance_address_seeds(&self.realm, &self.governed_account)
            }
            GovernanceAccountType::TokenGovernanceV1 | GovernanceAccountType::TokenGovernanceV2 => {
                get_token_governance_address_seeds(&self.realm, &self.governed_account)
            }
            GovernanceAccountType::Uninitialized
            | GovernanceAccountType::RealmV1
            | GovernanceAccountType::TokenOwnerRecordV1
            | GovernanceAccountType::ProposalV1
            | GovernanceAccountType::SignatoryRecordV1
            | GovernanceAccountType::VoteRecordV1
            | GovernanceAccountType::ProposalInstructionV1
            | GovernanceAccountType::RealmConfig
            | GovernanceAccountType::VoteRecordV2
            | GovernanceAccountType::ProposalTransactionV2
            | GovernanceAccountType::ProposalV2
            | GovernanceAccountType::ProgramMetadata
            | GovernanceAccountType::RealmV2
            | GovernanceAccountType::TokenOwnerRecordV2
            | GovernanceAccountType::SignatoryRecordV2 => {
                return Err(GovernanceToolsError::InvalidAccountType.into())
            }
        };

        Ok(seeds)
    }

    /// Serializes account into the target buffer
    pub fn serialize<W: Write>(self, writer: &mut W) -> Result<(), ProgramError> {
        if is_governance_v2_account_type(&self.account_type) {
            BorshSerialize::serialize(&self, writer)?
        } else if is_governance_v1_account_type(&self.account_type) {
            // V1 account can't be resized and we have to translate it back to the original format

            // If reserved_v2 is used it must be individually asses for v1 backward compatibility impact
            if self.reserved_v2 != [0; 128] {
                panic!("Extended data not supported by GovernanceV1")
            }

            let governance_data_v1 = GovernanceV1 {
                account_type: self.account_type,
                realm: self.realm,
                governed_account: self.governed_account,
                proposals_count: self.proposals_count,
                config: self.config,
                reserved: self.reserved,
                voting_proposal_count: self.voting_proposal_count,
            };

            BorshSerialize::serialize(&governance_data_v1, writer)?;
        }

        Ok(())
    }

    /// Asserts the provided voting population represented by the given governing_token_mint
    /// can cast the given vote type on proposals for the Governance
    pub fn assert_governing_token_mint_can_vote(
        &self,
        realm_data: &RealmV2,
        vote_governing_token_mint: &Pubkey,
        vote_kind: &VoteKind,
    ) -> Result<(), ProgramError> {
        // resolve_vote_threshold() asserts the vote threshold exists for the given governing_token_mint and is not disabled
        let _ = self.resolve_vote_threshold(realm_data, vote_governing_token_mint, vote_kind)?;

        Ok(())
    }

    /// Resolves VoteThreshold for the given realm, governing token and Vote kind
    pub fn resolve_vote_threshold(
        &self,
        realm_data: &RealmV2,
        vote_governing_token_mint: &Pubkey,
        vote_kind: &VoteKind,
    ) -> Result<VoteThreshold, ProgramError> {
        let vote_threshold = if realm_data.community_mint == *vote_governing_token_mint {
            match vote_kind {
                VoteKind::Electorate => &self.config.community_vote_threshold,
                VoteKind::Veto => {
                    // Community Veto vote is not supported in current version
                    return Err(GovernanceError::GoverningTokenMintNotAllowedToVote.into());
                }
            }
        } else if realm_data.config.council_mint == Some(*vote_governing_token_mint) {
            match vote_kind {
                VoteKind::Electorate => &self.config.council_vote_threshold,
                VoteKind::Veto => &self.config.council_veto_vote_threshold,
            }
        } else {
            return Err(GovernanceError::InvalidGoverningTokenMint.into());
        };

        if *vote_threshold == VoteThreshold::Disabled {
            return Err(GovernanceError::GoverningTokenMintNotAllowedToVote.into());
        }

        Ok(vote_threshold.clone())
    }
}

/// Deserializes Governance account and checks owner program
pub fn get_governance_data(
    program_id: &Pubkey,
    governance_info: &AccountInfo,
) -> Result<GovernanceV2, ProgramError> {
    if governance_info.data_is_empty() {
        return Err(GovernanceToolsError::AccountDoesNotExist.into());
    }

    let account_type: GovernanceAccountType =
        try_from_slice_unchecked(&governance_info.data.borrow())?;

    // If the account is V1 version then translate to V2
    let mut governance_data = if is_governance_v1_account_type(&account_type) {
        let governance_data_v1 = get_account_data::<GovernanceV1>(program_id, governance_info)?;

        GovernanceV2 {
            account_type,
            realm: governance_data_v1.realm,
            governed_account: governance_data_v1.governed_account,
            proposals_count: governance_data_v1.proposals_count,
            config: governance_data_v1.config,
            reserved: governance_data_v1.reserved,
            voting_proposal_count: governance_data_v1.voting_proposal_count,

            // Add the extra reserved_v2 padding
            reserved_v2: [0; 128],
        }
    } else {
        get_account_data::<GovernanceV2>(program_id, governance_info)?
    };

    // In previous versions of spl-gov (< 3) we had config.proposal_cool_off_time:u32 which was unused and always 0
    // In version 3.0.0 proposal_cool_off_time was replaced with council_vote_threshold:VoteThreshold and council_veto_vote_threshold:VoteThreshold
    //
    // If we read a legacy account then council_vote_threshold == VoteThreshold::YesVotePercentage(0)
    // and we coerce it to be equal to community_vote_threshold which was used for both council and community thresholds before
    //
    // Note: assert_is_valid_governance_config() prevents setting council_vote_threshold to VoteThreshold::YesVotePercentage(0)
    // which gives as guarantee that it is a legacy account layout set with proposal_cool_off_time = 0
    if governance_data.config.council_vote_threshold == VoteThreshold::YesVotePercentage(0) {
        governance_data.config.council_vote_threshold =
            governance_data.config.community_vote_threshold.clone();

        // The assumption here is that council should have Veto vote enabled by default and equal to council_vote_threshold
        governance_data.config.council_veto_vote_threshold =
            governance_data.config.council_vote_threshold.clone();
    }

    Ok(governance_data)
}

/// Deserializes Governance account, checks owner program and asserts governance belongs to the given ream
pub fn get_governance_data_for_realm(
    program_id: &Pubkey,
    governance_info: &AccountInfo,
    realm: &Pubkey,
) -> Result<GovernanceV2, ProgramError> {
    let governance_data = get_governance_data(program_id, governance_info)?;

    if governance_data.realm != *realm {
        return Err(GovernanceError::InvalidRealmForGovernance.into());
    }

    Ok(governance_data)
}

/// Checks the given account is a governance account and belongs to the given realm
pub fn assert_governance_for_realm(
    program_id: &Pubkey,
    governance_info: &AccountInfo,
    realm: &Pubkey,
) -> Result<(), ProgramError> {
    get_governance_data_for_realm(program_id, governance_info, realm)?;
    Ok(())
}

/// Returns ProgramGovernance PDA seeds
pub fn get_program_governance_address_seeds<'a>(
    realm: &'a Pubkey,
    governed_program: &'a Pubkey,
) -> [&'a [u8]; 3] {
    // 'program-governance' prefix ensures uniqueness of the PDA
    // Note: Only the current program upgrade authority can create an account with this PDA using CreateProgramGovernance instruction
    [
        b"program-governance",
        realm.as_ref(),
        governed_program.as_ref(),
    ]
}

/// Returns ProgramGovernance PDA address
pub fn get_program_governance_address<'a>(
    program_id: &Pubkey,
    realm: &'a Pubkey,
    governed_program: &'a Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_program_governance_address_seeds(realm, governed_program),
        program_id,
    )
    .0
}

/// Returns MintGovernance PDA seeds
pub fn get_mint_governance_address_seeds<'a>(
    realm: &'a Pubkey,
    governed_mint: &'a Pubkey,
) -> [&'a [u8]; 3] {
    // 'mint-governance' prefix ensures uniqueness of the PDA
    // Note: Only the current mint authority can create an account with this PDA using CreateMintGovernance instruction
    [b"mint-governance", realm.as_ref(), governed_mint.as_ref()]
}

/// Returns MintGovernance PDA address
pub fn get_mint_governance_address<'a>(
    program_id: &Pubkey,
    realm: &'a Pubkey,
    governed_mint: &'a Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_mint_governance_address_seeds(realm, governed_mint),
        program_id,
    )
    .0
}

/// Returns TokenGovernance PDA seeds
pub fn get_token_governance_address_seeds<'a>(
    realm: &'a Pubkey,
    governed_token: &'a Pubkey,
) -> [&'a [u8]; 3] {
    // 'token-governance' prefix ensures uniqueness of the PDA
    // Note: Only the current token account owner can create an account with this PDA using CreateTokenGovernance instruction
    [b"token-governance", realm.as_ref(), governed_token.as_ref()]
}

/// Returns TokenGovernance PDA address
pub fn get_token_governance_address<'a>(
    program_id: &Pubkey,
    realm: &'a Pubkey,
    governed_token: &'a Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_token_governance_address_seeds(realm, governed_token),
        program_id,
    )
    .0
}

/// Returns Governance PDA seeds
pub fn get_governance_address_seeds<'a>(
    realm: &'a Pubkey,
    governed_account: &'a Pubkey,
) -> [&'a [u8]; 3] {
    [
        b"account-governance",
        realm.as_ref(),
        governed_account.as_ref(),
    ]
}

/// Returns Governance PDA address
pub fn get_governance_address<'a>(
    program_id: &Pubkey,
    realm: &'a Pubkey,
    governed_account: &'a Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_governance_address_seeds(realm, governed_account),
        program_id,
    )
    .0
}

/// Checks whether the Governance account exists, is initialized and owned by the Governance program
pub fn assert_is_valid_governance(
    program_id: &Pubkey,
    governance_info: &AccountInfo,
) -> Result<(), ProgramError> {
    assert_is_valid_account_of_types(program_id, governance_info, is_governance_account_type)
}

/// Validates args supplied to create governance account
pub fn assert_valid_create_governance_args(
    program_id: &Pubkey,
    governance_config: &GovernanceConfig,
    realm_info: &AccountInfo,
) -> Result<(), ProgramError> {
    assert_is_valid_realm(program_id, realm_info)?;

    assert_is_valid_governance_config(governance_config)?;

    Ok(())
}

/// Validates governance config parameters
pub fn assert_is_valid_governance_config(
    governance_config: &GovernanceConfig,
) -> Result<(), ProgramError> {
    assert_is_valid_vote_threshold(&governance_config.community_vote_threshold)?;
    assert_is_valid_vote_threshold(&governance_config.council_vote_threshold)?;
    assert_is_valid_vote_threshold(&governance_config.council_veto_vote_threshold)?;

    // Setting both thresholds to Disabled is not allowed, however we might reconsider it as
    // a way to disable Governance permanently
    if governance_config.community_vote_threshold == VoteThreshold::Disabled
        && governance_config.council_vote_threshold == VoteThreshold::Disabled
    {
        return Err(GovernanceError::AtLeastOneVoteThresholdRequired.into());
    }

    Ok(())
}

/// Asserts the provided vote_threshold is valid
pub fn assert_is_valid_vote_threshold(vote_threshold: &VoteThreshold) -> Result<(), ProgramError> {
    match *vote_threshold {
        VoteThreshold::YesVotePercentage(yes_vote_threshold_percentage) => {
            if !(1..=100).contains(&yes_vote_threshold_percentage) {
                return Err(GovernanceError::InvalidVoteThresholdPercentage.into());
            }
        }
        VoteThreshold::QuorumPercentage(_) => {
            return Err(GovernanceError::VoteThresholdTypeNotSupported.into());
        }
        VoteThreshold::Disabled => {}
    }

    Ok(())
}

#[cfg(test)]
mod test {
    use solana_program::clock::Epoch;

    use super::*;

    #[test]
    fn test_deserialize_legacy_governance_account_without_council_vote_thresholds() {
        // Arrange

        // Legacy GovernanceV2 with
        // 1) config.community_vote_threshold = YesVotePercentage(10)
        // 2) config.proposal_cool_off_time = 0
        let mut account_data = [
            18, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 10, 10, 0, 0, 0, 0, 0, 0, 0, 10, 0, 0, 0, 100, 0,
            0, 0, 1, //
            // proposal_cool_off_time:
            0, 0, 0, 0, // ---------
            1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
            0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0,
        ];

        let program_id = Pubkey::new_unique();

        let info_key = Pubkey::new_unique();
        let mut lamports = 10u64;

        let governance_info = AccountInfo::new(
            &info_key,
            false,
            false,
            &mut lamports,
            &mut account_data[..],
            &program_id,
            false,
            Epoch::default(),
        );

        // Act
        let governance = get_governance_data(&program_id, &governance_info).unwrap();

        // Assert
        assert_eq!(
            governance.config.community_vote_threshold,
            governance.config.council_vote_threshold
        );

        assert_eq!(
            governance.config.council_vote_threshold,
            governance.config.council_veto_vote_threshold
        );
    }

    #[test]
    fn test_assert_config_invalid_with_council_zero_yes_vote_threshold() {
        // Arrange
        let governance_config = GovernanceConfig {
            community_vote_threshold: VoteThreshold::YesVotePercentage(1),
            min_community_weight_to_create_proposal: 1,
            min_transaction_hold_up_time: 1,
            max_voting_time: 1,
            vote_tipping: VoteTipping::Strict,
            council_vote_threshold: VoteThreshold::YesVotePercentage(0),
            council_veto_vote_threshold: VoteThreshold::YesVotePercentage(1),
            min_council_weight_to_create_proposal: 1,
        };

        // Act
        let err = assert_is_valid_governance_config(&governance_config)
            .err()
            .unwrap();

        // Assert
        assert_eq!(err, GovernanceError::InvalidVoteThresholdPercentage.into());
    }

    #[test]
    fn test_assert_config_invalid_with_community_zero_yes_vote_threshold() {
        // Arrange
        let governance_config = GovernanceConfig {
            community_vote_threshold: VoteThreshold::YesVotePercentage(0),
            min_community_weight_to_create_proposal: 1,
            min_transaction_hold_up_time: 1,
            max_voting_time: 1,
            vote_tipping: VoteTipping::Strict,
            council_vote_threshold: VoteThreshold::YesVotePercentage(1),
            council_veto_vote_threshold: VoteThreshold::YesVotePercentage(1),
            min_council_weight_to_create_proposal: 1,
        };

        // Act
        let err = assert_is_valid_governance_config(&governance_config)
            .err()
            .unwrap();

        // Assert
        assert_eq!(err, GovernanceError::InvalidVoteThresholdPercentage.into());
    }

    #[test]
    fn test_assert_config_invalid_with_all_vote_thresholds_disabled() {
        // Arrange
        let governance_config = GovernanceConfig {
            community_vote_threshold: VoteThreshold::Disabled,
            min_community_weight_to_create_proposal: 1,
            min_transaction_hold_up_time: 1,
            max_voting_time: 1,
            vote_tipping: VoteTipping::Strict,
            council_vote_threshold: VoteThreshold::Disabled,
            council_veto_vote_threshold: VoteThreshold::YesVotePercentage(1),
            min_council_weight_to_create_proposal: 1,
        };

        // Act
        let err = assert_is_valid_governance_config(&governance_config)
            .err()
            .unwrap();

        // Assert
        assert_eq!(err, GovernanceError::AtLeastOneVoteThresholdRequired.into());
    }

    #[test]
    fn test_assert_config_invalid_with_council_zero_yes_veto_vote_threshold() {
        // Arrange
        let governance_config = GovernanceConfig {
            community_vote_threshold: VoteThreshold::YesVotePercentage(1),
            min_community_weight_to_create_proposal: 1,
            min_transaction_hold_up_time: 1,
            max_voting_time: 1,
            vote_tipping: VoteTipping::Strict,
            council_vote_threshold: VoteThreshold::YesVotePercentage(1),
            council_veto_vote_threshold: VoteThreshold::YesVotePercentage(0),
            min_council_weight_to_create_proposal: 1,
        };

        // Act
        let err = assert_is_valid_governance_config(&governance_config)
            .err()
            .unwrap();

        // Assert
        assert_eq!(err, GovernanceError::InvalidVoteThresholdPercentage.into());
    }
}
