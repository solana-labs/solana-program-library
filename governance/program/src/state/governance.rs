//! Governance Account

use crate::{
    error::GovernanceError,
    state::{
        enums::{GovernanceAccountType, VoteThresholdPercentage, VoteWeightSource},
        realm::assert_is_valid_realm,
    },
    tools::account::{get_account_data, AccountMaxSize},
};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    account_info::AccountInfo, program_error::ProgramError, program_pack::IsInitialized,
    pubkey::Pubkey,
};

/// Governance config
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct GovernanceConfig {
    /// The type of the vote threshold used for voting
    /// Note: In the current version only YesVote threshold is supported
    pub vote_threshold_percentage: VoteThresholdPercentage,

    /// Minimum number of community tokens a governance token owner must possess to be able to create a proposal
    pub min_community_tokens_to_create_proposal: u64,

    /// Minimum waiting time in seconds for an instruction to be executed after proposal is voted on
    pub min_instruction_hold_up_time: u32,

    /// Time limit in seconds for proposal to be open for voting
    pub max_voting_time: u32,

    /// The source of vote weight for voters
    /// Note: In the current version only token deposits are accepted as vote weight
    pub vote_weight_source: VoteWeightSource,

    /// The time period in seconds within which a Proposal can be still cancelled after being voted on
    /// Once cool off time expires Proposal can't be cancelled any longer and becomes a law
    /// Note: This field is not implemented in the current version
    pub proposal_cool_off_time: u32,

    /// Minimum number of council tokens a governance token owner must possess to be able to create a proposal
    pub min_council_tokens_to_create_proposal: u64,
}

/// Governance Account
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct Governance {
    /// Account type. It can be Uninitialized, AccountGovernance or ProgramGovernance
    pub account_type: GovernanceAccountType,

    /// Governance Realm
    pub realm: Pubkey,

    /// Account governed by this Governance. It can be for example Program account, Mint account or Token Account
    pub governed_account: Pubkey,

    /// Running count of proposals
    pub proposals_count: u32,

    /// Governance config
    pub config: GovernanceConfig,

    /// Reserved space for future versions
    pub reserved: [u8; 8],
}

impl AccountMaxSize for Governance {}

impl IsInitialized for Governance {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::AccountGovernance
            || self.account_type == GovernanceAccountType::ProgramGovernance
            || self.account_type == GovernanceAccountType::MintGovernance
            || self.account_type == GovernanceAccountType::TokenGovernance
    }
}

impl Governance {
    /// Returns Governance PDA seeds
    pub fn get_governance_address_seeds(&self) -> Result<[&[u8]; 3], ProgramError> {
        let seeds = match self.account_type {
            GovernanceAccountType::AccountGovernance => {
                get_account_governance_address_seeds(&self.realm, &self.governed_account)
            }
            GovernanceAccountType::ProgramGovernance => {
                get_program_governance_address_seeds(&self.realm, &self.governed_account)
            }
            GovernanceAccountType::MintGovernance => {
                get_mint_governance_address_seeds(&self.realm, &self.governed_account)
            }
            GovernanceAccountType::TokenGovernance => {
                get_token_governance_address_seeds(&self.realm, &self.governed_account)
            }
            _ => return Err(GovernanceError::InvalidAccountType.into()),
        };

        Ok(seeds)
    }
}

/// Deserializes Governance account and checks owner program
pub fn get_governance_data(
    program_id: &Pubkey,
    governance_info: &AccountInfo,
) -> Result<Governance, ProgramError> {
    get_account_data::<Governance>(governance_info, program_id)
}

/// Deserializes Governance account, checks owner program and asserts governance belongs to the given ream
pub fn get_governance_data_for_realm(
    program_id: &Pubkey,
    governance_info: &AccountInfo,
    realm: &Pubkey,
) -> Result<Governance, ProgramError> {
    let governance_data = get_governance_data(program_id, governance_info)?;

    if governance_data.realm != *realm {
        return Err(GovernanceError::InvalidRealmForGovernance.into());
    }

    Ok(governance_data)
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

/// Returns AccountGovernance PDA seeds
pub fn get_account_governance_address_seeds<'a>(
    realm: &'a Pubkey,
    governed_account: &'a Pubkey,
) -> [&'a [u8]; 3] {
    [
        b"account-governance",
        realm.as_ref(),
        governed_account.as_ref(),
    ]
}

/// Returns AccountGovernance PDA address
pub fn get_account_governance_address<'a>(
    program_id: &Pubkey,
    realm: &'a Pubkey,
    governed_account: &'a Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_account_governance_address_seeds(realm, governed_account),
        program_id,
    )
    .0
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
    match governance_config.vote_threshold_percentage {
        VoteThresholdPercentage::YesVote(yes_vote_threshold_percentage) => {
            if !(1..=100).contains(&yes_vote_threshold_percentage) {
                return Err(GovernanceError::InvalidVoteThresholdPercentage.into());
            }
        }
        _ => {
            return Err(GovernanceError::VoteThresholdPercentageTypeNotSupported.into());
        }
    }

    if governance_config.vote_weight_source != VoteWeightSource::Deposit {
        return Err(GovernanceError::VoteWeightSourceNotSupported.into());
    }

    if governance_config.proposal_cool_off_time > 0 {
        return Err(GovernanceError::ProposalCoolOffTimeNotSupported.into());
    }

    Ok(())
}
