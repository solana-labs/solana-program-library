//! Program Governance Account

use crate::{id, state::enums::GovernanceAccountType, tools::account::AccountMaxSize};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{program_pack::IsInitialized, pubkey::Pubkey};

/// Governance config
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct GovernanceConfig {
    /// Governance Realm
    pub realm: Pubkey,

    /// Account governed by this Governance. It can be for example Program account, Mint account or Token Account
    pub governed_account: Pubkey,

    /// Voting threshold in % required to tip the vote
    /// It's the percentage of tokens out of the entire pool of governance tokens eligible to vote
    pub vote_threshold_percentage: u8,

    /// Minimum % of tokens for a governance token owner to be able to create a proposal
    /// It's the percentage of tokens out of the entire pool of governance tokens eligible to vote
    pub token_threshold_percentage_to_create_proposal: u8,

    /// Minimum waiting time in slots for an instruction to be executed after proposal is voted on
    pub min_instruction_hold_up_time: u64,

    /// Time limit in slots for proposal to be open for voting
    pub max_voting_time: u64,
}

/// Governance Account
#[repr(C)]
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct Governance {
    /// Account type. It can be Uninitialized, AccountGovernance or ProgramGovernance
    pub account_type: GovernanceAccountType,

    /// Governance config
    pub config: GovernanceConfig,

    /// Running count of proposals
    pub proposal_count: u32,
}

impl AccountMaxSize for Governance {}

impl IsInitialized for Governance {
    fn is_initialized(&self) -> bool {
        self.account_type == GovernanceAccountType::AccountGovernance
            || self.account_type == GovernanceAccountType::ProgramGovernance
    }
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
        &realm.as_ref(),
        &governed_program.as_ref(),
    ]
}

/// Returns ProgramGovernance PDA address
pub fn get_program_governance_address<'a>(
    realm: &'a Pubkey,
    governed_program: &'a Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_program_governance_address_seeds(realm, governed_program),
        &id(),
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
        &realm.as_ref(),
        &governed_account.as_ref(),
    ]
}

/// Returns AccountGovernance PDA address
pub fn get_account_governance_address<'a>(
    realm: &'a Pubkey,
    governed_account: &'a Pubkey,
) -> Pubkey {
    Pubkey::find_program_address(
        &get_account_governance_address_seeds(realm, governed_account),
        &id(),
    )
    .0
}
