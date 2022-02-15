//! VoterWeight Addin interface

use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{clock::Slot, program_pack::IsInitialized, pubkey::Pubkey};
use spl_governance_tools::account::AccountMaxSize;

/// The governance action VoterWeight is evaluated for
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub enum VoterWeightAction {
    /// Cast vote for a proposal. Target: Proposal
    CastVote,

    /// Comment a proposal. Target: Proposal
    CommentProposal,

    /// Create Governance within a realm. Target: Realm
    CreateGovernance,

    /// Create a proposal for a governance. Target: Governance
    CreateProposal,

    /// Signs off a proposal for a governance. Target: Proposal
    /// Note: SignOffProposal is not supported in the current version
    SignOffProposal,
}

/// VoterWeightRecord account
/// The account is used as an api interface to provide voting power to the governance program from external addin contracts
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
pub struct VoterWeightRecord {
    /// VoterWeightRecord discriminator sha256("account:VoterWeightRecord")[..8]
    /// Note: The discriminator size must match the addin implementing program discriminator size
    /// to ensure it's stored in the private space of the account data and it's unique
    pub account_discriminator: [u8; 8],

    /// The Realm the VoterWeightRecord belongs to
    pub realm: Pubkey,

    /// Governing Token Mint the VoterWeightRecord is associated with
    /// Note: The addin can take deposits of any tokens and is not restricted to the community or council tokens only
    // The mint here is to link the record to either community or council mint of the realm
    pub governing_token_mint: Pubkey,

    /// The owner of the governing token and voter
    /// This is the actual owner (voter) and corresponds to TokenOwnerRecord.governing_token_owner
    pub governing_token_owner: Pubkey,

    /// Voter's weight
    /// The weight of the voter provided by the addin for the given realm, governing_token_mint and governing_token_owner (voter)
    pub voter_weight: u64,

    /// The slot when the voting weight expires
    /// It should be set to None if the weight never expires
    /// If the voter weight decays with time, for example for time locked based weights, then the expiry must be set
    /// As a common pattern Revise instruction to update the weight should be invoked before governance instruction within the same transaction
    /// and the expiry set to the current slot to provide up to date weight
    pub voter_weight_expiry: Option<Slot>,

    /// The governance action the voter's weight pertains to
    /// It allows to provided voter's weight specific to the particular action the weight is evaluated for
    /// When the action is provided then the governance program asserts the executing action is the same as specified by the addin
    pub weight_action: Option<VoterWeightAction>,

    /// The target the voter's weight  action pertains to
    /// It allows to provided voter's weight specific to the target the weight is evaluated for
    /// For example when addin supplies weight to vote on a particular proposal then it must specify the proposal as the action target
    /// When the target is provided then the governance program asserts the target is the same as specified by the addin
    pub weight_action_target: Option<Pubkey>,

    /// Reserved space for future versions
    pub reserved: [u8; 8],
}

impl VoterWeightRecord {
    /// sha256("account:VoterWeightRecord")[..8]
    pub const ACCOUNT_DISCRIMINATOR: [u8; 8] = *b"2ef99b4b";
}

impl AccountMaxSize for VoterWeightRecord {}

impl IsInitialized for VoterWeightRecord {
    fn is_initialized(&self) -> bool {
        self.account_discriminator == VoterWeightRecord::ACCOUNT_DISCRIMINATOR
    }
}
