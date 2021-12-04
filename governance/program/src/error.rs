//! Error types

use num_derive::FromPrimitive;
use solana_program::{
    decode_error::DecodeError,
    msg,
    program_error::{PrintProgramError, ProgramError},
};
use thiserror::Error;

/// Errors that may be returned by the Governance program
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum GovernanceError {
    /// Invalid instruction passed to program
    #[error("Invalid instruction passed to program")]
    InvalidInstruction = 500, // Start Governance custom errors from 500 to avoid conflicts with programs invoked via CPI

    /// Realm with the given name and governing mints already exists
    #[error("Realm with the given name and governing mints already exists")]
    RealmAlreadyExists,

    /// Invalid Realm
    #[error("Invalid realm")]
    InvalidRealm,

    /// Invalid Governing Token Mint
    #[error("Invalid Governing Token Mint")]
    InvalidGoverningTokenMint,

    /// Governing Token Owner must sign transaction
    #[error("Governing Token Owner must sign transaction")]
    GoverningTokenOwnerMustSign,

    /// Governing Token Owner or Delegate  must sign transaction
    #[error("Governing Token Owner or Delegate  must sign transaction")]
    GoverningTokenOwnerOrDelegateMustSign,

    /// All votes must be relinquished to withdraw governing tokens
    #[error("All votes must be relinquished to withdraw governing tokens")]
    AllVotesMustBeRelinquishedToWithdrawGoverningTokens,

    /// Invalid Token Owner Record account address
    #[error("Invalid Token Owner Record account address")]
    InvalidTokenOwnerRecordAccountAddress,

    /// Invalid GoverningMint for TokenOwnerRecord
    #[error("Invalid GoverningMint for TokenOwnerRecord")]
    InvalidGoverningMintForTokenOwnerRecord,

    /// Invalid Realm for TokenOwnerRecord
    #[error("Invalid Realm for TokenOwnerRecord")]
    InvalidRealmForTokenOwnerRecord,

    /// Invalid Proposal for ProposalInstruction
    #[error("Invalid Proposal for ProposalInstruction")]
    InvalidProposalForProposalInstruction,

    /// Invalid Signatory account address
    #[error("Invalid Signatory account address")]
    InvalidSignatoryAddress,

    /// Signatory already signed off
    #[error("Signatory already signed off")]
    SignatoryAlreadySignedOff,

    /// Signatory must sign
    #[error("Signatory must sign")]
    SignatoryMustSign,

    /// Invalid Proposal Owner
    #[error("Invalid Proposal Owner")]
    InvalidProposalOwnerAccount,

    /// Invalid Proposal for VoterRecord
    #[error("Invalid Proposal for VoterRecord")]
    InvalidProposalForVoterRecord,

    /// Invalid GoverningTokenOwner  for VoteRecord
    #[error("Invalid GoverningTokenOwner for VoteRecord")]
    InvalidGoverningTokenOwnerForVoteRecord,

    /// Invalid Governance config: Vote threshold percentage out of range"
    #[error("Invalid Governance config: Vote threshold percentage out of range")]
    InvalidVoteThresholdPercentage,

    /// Proposal for the given Governance, Governing Token Mint and index already exists
    #[error("Proposal for the given Governance, Governing Token Mint and index already exists")]
    ProposalAlreadyExists,

    /// Token Owner already voted on the Proposal
    #[error("Token Owner already voted on the Proposal")]
    VoteAlreadyExists,

    /// Owner doesn't have enough governing tokens to create Proposal
    #[error("Owner doesn't have enough governing tokens to create Proposal")]
    NotEnoughTokensToCreateProposal,

    /// Invalid State: Can't edit Signatories
    #[error("Invalid State: Can't edit Signatories")]
    InvalidStateCannotEditSignatories,

    /// Invalid Proposal state
    #[error("Invalid Proposal state")]
    InvalidProposalState,
    /// Invalid State: Can't edit instructions
    #[error("Invalid State: Can't edit instructions")]
    InvalidStateCannotEditInstructions,

    /// Invalid State: Can't execute instruction
    #[error("Invalid State: Can't execute instruction")]
    InvalidStateCannotExecuteInstruction,

    /// Can't execute instruction within its hold up time
    #[error("Can't execute instruction within its hold up time")]
    CannotExecuteInstructionWithinHoldUpTime,

    /// Instruction already executed
    #[error("Instruction already executed")]
    InstructionAlreadyExecuted,

    /// Invalid Instruction index
    #[error("Invalid Instruction index")]
    InvalidInstructionIndex,

    /// Instruction hold up time is below the min specified by Governance
    #[error("Instruction hold up time is below the min specified by Governance")]
    InstructionHoldUpTimeBelowRequiredMin,

    /// Instruction at the given index for the Proposal already exists
    #[error("Instruction at the given index for the Proposal already exists")]
    InstructionAlreadyExists,

    /// Invalid State: Can't sign off
    #[error("Invalid State: Can't sign off")]
    InvalidStateCannotSignOff,

    /// Invalid State: Can't vote
    #[error("Invalid State: Can't vote")]
    InvalidStateCannotVote,

    /// Invalid State: Can't finalize vote
    #[error("Invalid State: Can't finalize vote")]
    InvalidStateCannotFinalize,

    /// Invalid State: Can't cancel Proposal
    #[error("Invalid State: Can't cancel Proposal")]
    InvalidStateCannotCancelProposal,

    /// Vote already relinquished
    #[error("Vote already relinquished")]
    VoteAlreadyRelinquished,

    /// Can't finalize vote. Voting still in progress
    #[error("Can't finalize vote. Voting still in progress")]
    CannotFinalizeVotingInProgress,

    /// Proposal voting time expired
    #[error("Proposal voting time expired")]
    ProposalVotingTimeExpired,

    /// Invalid Signatory Mint
    #[error("Invalid Signatory Mint")]
    InvalidSignatoryMint,

    /// Proposal does not belong to the given Governance
    #[error("Proposal does not belong to the given Governance")]
    InvalidGovernanceForProposal,

    /// Proposal does not belong to given Governing Mint"
    #[error("Proposal does not belong to given Governing Mint")]
    InvalidGoverningMintForProposal,

    /// Current mint authority must sign transaction
    #[error("Current mint authority must sign transaction")]
    MintAuthorityMustSign,

    /// Invalid mint authority
    #[error("Invalid mint authority")]
    InvalidMintAuthority,

    /// Mint has no authority
    #[error("Mint has no authority")]
    MintHasNoAuthority,

    /// ---- SPL Token Tools Errors ----

    /// Invalid Token account owner
    #[error("Invalid Token account owner")]
    SplTokenAccountWithInvalidOwner,

    /// Invalid Mint account owner
    #[error("Invalid Mint account owner")]
    SplTokenMintWithInvalidOwner,

    /// Token Account is not initialized
    #[error("Token Account is not initialized")]
    SplTokenAccountNotInitialized,

    /// Token Account doesn't exist
    #[error("Token Account doesn't exist")]
    SplTokenAccountDoesNotExist,

    /// Token account data is invalid
    #[error("Token account data is invalid")]
    SplTokenInvalidTokenAccountData,

    /// Token mint account data is invalid
    #[error("Token mint account data is invalid")]
    SplTokenInvalidMintAccountData,

    /// Token Mint is not initialized
    #[error("Token Mint account is not initialized")]
    SplTokenMintNotInitialized,

    /// Token Mint account doesn't exist
    #[error("Token Mint account doesn't exist")]
    SplTokenMintDoesNotExist,

    /// ---- Bpf Upgradable Loader Tools Errors ----

    /// Invalid ProgramData account Address
    #[error("Invalid ProgramData account address")]
    InvalidProgramDataAccountAddress,

    /// Invalid ProgramData account data
    #[error("Invalid ProgramData account Data")]
    InvalidProgramDataAccountData,

    /// Provided upgrade authority doesn't match current program upgrade authority
    #[error("Provided upgrade authority doesn't match current program upgrade authority")]
    InvalidUpgradeAuthority,

    /// Current program upgrade authority must sign transaction
    #[error("Current program upgrade authority must sign transaction")]
    UpgradeAuthorityMustSign,

    /// Given program is not upgradable
    #[error("Given program is not upgradable")]
    ProgramNotUpgradable,

    /// Invalid token owner
    #[error("Invalid token owner")]
    InvalidTokenOwner,

    /// Current token owner must sign transaction
    #[error("Current token owner must sign transaction")]
    TokenOwnerMustSign,

    /// Given VoteThresholdPercentageType is not supported
    #[error("Given VoteThresholdPercentageType is not supported")]
    VoteThresholdPercentageTypeNotSupported,

    /// Given VoteWeightSource is not supported
    #[error("Given VoteWeightSource is not supported")]
    VoteWeightSourceNotSupported,

    /// Proposal cool off time is not supported
    #[error("Proposal cool off time is not supported")]
    ProposalCoolOffTimeNotSupported,

    /// Governance PDA must sign
    #[error("Governance PDA must sign")]
    GovernancePdaMustSign,

    /// Instruction already flagged with error
    #[error("Instruction already flagged with error")]
    InstructionAlreadyFlaggedWithError,

    /// Invalid Realm for Governance
    #[error("Invalid Realm for Governance")]
    InvalidRealmForGovernance,

    /// Invalid Authority for Realm
    #[error("Invalid Authority for Realm")]
    InvalidAuthorityForRealm,

    /// Realm has no authority
    #[error("Realm has no authority")]
    RealmHasNoAuthority,

    /// Realm authority must sign
    #[error("Realm authority must sign")]
    RealmAuthorityMustSign,

    /// Invalid governing token holding account
    #[error("Invalid governing token holding account")]
    InvalidGoverningTokenHoldingAccount,

    /// Realm council mint change is not supported
    #[error("Realm council mint change is not supported")]
    RealmCouncilMintChangeIsNotSupported,

    /// Not supported mint max vote weight source
    #[error("Not supported mint max vote weight source")]
    MintMaxVoteWeightSourceNotSupported,

    /// Invalid max vote weight supply fraction
    #[error("Invalid max vote weight supply fraction")]
    InvalidMaxVoteWeightSupplyFraction,

    /// Owner doesn't have enough governing tokens to create Governance
    #[error("Owner doesn't have enough governing tokens to create Governance")]
    NotEnoughTokensToCreateGovernance,

    /// Too many outstanding proposals
    #[error("Too many outstanding proposals")]
    TooManyOutstandingProposals,

    /// All proposals must be finalized to withdraw governing tokens
    #[error("All proposals must be finalized to withdraw governing tokens")]
    AllProposalsMustBeFinalisedToWithdrawGoverningTokens,

    /// Invalid VoterWeightRecord for Realm
    #[error("Invalid VoterWeightRecord for Realm")]
    InvalidVoterWeightRecordForRealm,

    /// Invalid VoterWeightRecord for GoverningTokenMint
    #[error("Invalid VoterWeightRecord for GoverningTokenMint")]
    InvalidVoterWeightRecordForGoverningTokenMint,

    /// Invalid VoterWeightRecord for TokenOwner
    #[error("Invalid VoterWeightRecord for TokenOwner")]
    InvalidVoterWeightRecordForTokenOwner,

    /// VoterWeightRecord expired
    #[error("VoterWeightRecord expired")]
    VoterWeightRecordExpired,

    /// Invalid RealmConfig for Realm
    #[error("Invalid RealmConfig for Realm")]
    InvalidRealmConfigForRealm,

    /// TokenOwnerRecord already exists
    #[error("TokenOwnerRecord already exists")]
    TokenOwnerRecordAlreadyExists,

    /// Governing token deposits not allowed
    #[error("Governing token deposits not allowed")]
    GoverningTokenDepositsNotAllowed,

    /// Invalid vote choice weight percentage
    #[error("Invalid vote choice weight percentage")]
    InvalidVoteChoiceWeightPercentage,

    /// Vote type not supported
    #[error("Vote type not supported")]
    VoteTypeNotSupported,

    /// InvalidProposalOptions
    #[error("Invalid proposal options")]
    InvalidProposalOptions,

    /// Proposal is not not executable
    #[error("Proposal is not not executable")]
    ProposalIsNotExecutable,

    /// Invalid vote
    #[error("Invalid vote")]
    InvalidVote,

    /// Cannot execute defeated option
    #[error("Cannot execute defeated option")]
    CannotExecuteDefeatedOption,
}

impl PrintProgramError for GovernanceError {
    fn print<E>(&self) {
        msg!("GOVERNANCE-ERROR: {}", &self.to_string());
    }
}

impl From<GovernanceError> for ProgramError {
    fn from(e: GovernanceError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

impl<T> DecodeError<T> for GovernanceError {
    fn type_of() -> &'static str {
        "Governance Error"
    }
}
