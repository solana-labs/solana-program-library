//! Error types

use {
    num_derive::FromPrimitive,
    solana_program::{
        decode_error::DecodeError,
        msg,
        program_error::{PrintProgramError, ProgramError},
    },
    thiserror::Error,
};

/// Errors that may be returned by the Governance program
// Start Governance custom errors from 500 to avoid conflicts with programs
// invoked via CPI
#[derive(Clone, Debug, Eq, Error, FromPrimitive, PartialEq)]
pub enum GovernanceError {
    /// Invalid instruction passed to program
    #[error("Invalid instruction passed to program")]
    InvalidInstruction = 500,

    /// Realm with the given name and governing mints already exists
    #[error("Realm with the given name and governing mints already exists")]
    RealmAlreadyExists,

    /// Invalid Realm
    #[error("Invalid realm")]
    InvalidRealm,

    /// Invalid Governing Token Mint
    #[error("Invalid Governing Token Mint")]
    InvalidGoverningTokenMint, // 503

    /// Governing Token Owner must sign transaction
    #[error("Governing Token Owner must sign transaction")]
    GoverningTokenOwnerMustSign,

    /// Governing Token Owner or Delegate  must sign transaction
    #[error("Governing Token Owner or Delegate  must sign transaction")]
    GoverningTokenOwnerOrDelegateMustSign, // 505

    /// All votes must be relinquished to withdraw governing tokens
    #[error("All votes must be relinquished to withdraw governing tokens")]
    AllVotesMustBeRelinquishedToWithdrawGoverningTokens,

    /// Invalid Token Owner Record account address
    #[error("Invalid Token Owner Record account address")]
    InvalidTokenOwnerRecordAccountAddress,

    /// Invalid GoverningMint for TokenOwnerRecord
    #[error("Invalid GoverningMint for TokenOwnerRecord")]
    InvalidGoverningMintForTokenOwnerRecord, // 508

    /// Invalid Realm for TokenOwnerRecord
    #[error("Invalid Realm for TokenOwnerRecord")]
    InvalidRealmForTokenOwnerRecord, // 509

    /// Invalid Proposal for ProposalTransaction,
    #[error("Invalid Proposal for ProposalTransaction,")]
    InvalidProposalForProposalTransaction, // 510

    /// Invalid Signatory account address
    #[error("Invalid Signatory account address")]
    InvalidSignatoryAddress, // 511

    /// Signatory already signed off
    #[error("Signatory already signed off")]
    SignatoryAlreadySignedOff, // 512

    /// Signatory must sign
    #[error("Signatory must sign")]
    SignatoryMustSign, // 513

    /// Invalid Proposal Owner
    #[error("Invalid Proposal Owner")]
    InvalidProposalOwnerAccount, // 514

    /// Invalid Proposal for VoterRecord
    #[error("Invalid Proposal for VoterRecord")]
    InvalidProposalForVoterRecord, // 515

    /// Invalid GoverningTokenOwner  for VoteRecord
    #[error("Invalid GoverningTokenOwner for VoteRecord")]
    InvalidGoverningTokenOwnerForVoteRecord, // 516

    /// Invalid Governance config: Vote threshold percentage out of range"
    #[error("Invalid Governance config: Vote threshold percentage out of range")]
    InvalidVoteThresholdPercentage, // 517

    /// Proposal for the given Governance, Governing Token Mint and index
    /// already exists
    #[error("Proposal for the given Governance, Governing Token Mint and index already exists")]
    ProposalAlreadyExists, // 518

    /// Token Owner already voted on the Proposal
    #[error("Token Owner already voted on the Proposal")]
    VoteAlreadyExists, // 519

    /// Owner doesn't have enough governing tokens to create Proposal
    #[error("Owner doesn't have enough governing tokens to create Proposal")]
    NotEnoughTokensToCreateProposal, // 520

    /// Invalid State: Can't edit Signatories
    #[error("Invalid State: Can't edit Signatories")]
    InvalidStateCannotEditSignatories, // 521

    /// Invalid Proposal state
    #[error("Invalid Proposal state")]
    InvalidProposalState, // 522

    /// Invalid State: Can't edit transactions
    #[error("Invalid State: Can't edit transactions")]
    InvalidStateCannotEditTransactions, // 523

    /// Invalid State: Can't execute transaction
    #[error("Invalid State: Can't execute transaction")]
    InvalidStateCannotExecuteTransaction, // 524

    /// Can't execute transaction within its hold up time
    #[error("Can't execute transaction within its hold up time")]
    CannotExecuteTransactionWithinHoldUpTime, // 525

    /// Transaction already executed
    #[error("Transaction already executed")]
    TransactionAlreadyExecuted, // 526

    /// Invalid Transaction index
    #[error("Invalid Transaction index")]
    InvalidTransactionIndex, // 527

    /// Legacy TransactionHoldUpTimeBelowRequiredMin
    #[error("Legacy3")]
    Legacy3, // 528

    /// Transaction at the given index for the Proposal already exists
    #[error("Transaction at the given index for the Proposal already exists")]
    TransactionAlreadyExists, // 529

    /// Invalid State: Can't sign off
    #[error("Invalid State: Can't sign off")]
    InvalidStateCannotSignOff, // 530

    /// Invalid State: Can't vote
    #[error("Invalid State: Can't vote")]
    InvalidStateCannotVote, // 531

    /// Invalid State: Can't finalize vote
    #[error("Invalid State: Can't finalize vote")]
    InvalidStateCannotFinalize, // 532

    /// Invalid State: Can't cancel Proposal
    #[error("Invalid State: Can't cancel Proposal")]
    InvalidStateCannotCancelProposal, // 533

    /// Vote already relinquished
    #[error("Vote already relinquished")]
    VoteAlreadyRelinquished, // 534

    /// Can't finalize vote. Voting still in progress
    #[error("Can't finalize vote. Voting still in progress")]
    CannotFinalizeVotingInProgress, // 535

    /// Proposal voting time expired
    #[error("Proposal voting time expired")]
    ProposalVotingTimeExpired, // 536

    /// Invalid Signatory Mint
    #[error("Invalid Signatory Mint")]
    InvalidSignatoryMint, // 537

    /// Proposal does not belong to the given Governance
    #[error("Proposal does not belong to the given Governance")]
    InvalidGovernanceForProposal, // 538

    /// Proposal does not belong to given Governing Mint"
    #[error("Proposal does not belong to given Governing Mint")]
    InvalidGoverningMintForProposal, // 539

    /// Current mint authority must sign transaction
    #[error("Current mint authority must sign transaction")]
    MintAuthorityMustSign, // 540

    /// Invalid mint authority
    #[error("Invalid mint authority")]
    InvalidMintAuthority, // 542

    /// Mint has no authority
    #[error("Mint has no authority")]
    MintHasNoAuthority, // 542

    /// ---- SPL Token Tools Errors ----

    /// Invalid Token account owner
    #[error("Invalid Token account owner")]
    SplTokenAccountWithInvalidOwner, // 543

    /// Invalid Mint account owner
    #[error("Invalid Mint account owner")]
    SplTokenMintWithInvalidOwner, // 544

    /// Token Account is not initialized
    #[error("Token Account is not initialized")]
    SplTokenAccountNotInitialized, // 545

    /// Token Account doesn't exist
    #[error("Token Account doesn't exist")]
    SplTokenAccountDoesNotExist, // 546

    /// Token account data is invalid
    #[error("Token account data is invalid")]
    SplTokenInvalidTokenAccountData, // 547

    /// Token mint account data is invalid
    #[error("Token mint account data is invalid")]
    SplTokenInvalidMintAccountData, // 548

    /// Token Mint is not initialized
    #[error("Token Mint account is not initialized")]
    SplTokenMintNotInitialized, // 549

    /// Token Mint account doesn't exist
    #[error("Token Mint account doesn't exist")]
    SplTokenMintDoesNotExist, // 550

    /// ---- Bpf Upgradable Loader Tools Errors ----

    /// Invalid ProgramData account Address
    #[error("Invalid ProgramData account address")]
    InvalidProgramDataAccountAddress, // 551

    /// Invalid ProgramData account data
    #[error("Invalid ProgramData account Data")]
    InvalidProgramDataAccountData, // 552

    /// Provided upgrade authority doesn't match current program upgrade
    /// authority
    #[error("Provided upgrade authority doesn't match current program upgrade authority")]
    InvalidUpgradeAuthority, // 553

    /// Current program upgrade authority must sign transaction
    #[error("Current program upgrade authority must sign transaction")]
    UpgradeAuthorityMustSign, // 554

    /// Given program is not upgradable
    #[error("Given program is not upgradable")]
    ProgramNotUpgradable, // 555

    /// Invalid token owner
    #[error("Invalid token owner")]
    InvalidTokenOwner, // 556

    /// Current token owner must sign transaction
    #[error("Current token owner must sign transaction")]
    TokenOwnerMustSign, // 557

    /// Given VoteThresholdType is not supported
    #[error("Given VoteThresholdType is not supported")]
    VoteThresholdTypeNotSupported, // 558

    /// Given VoteWeightSource is not supported
    #[error("Given VoteWeightSource is not supported")]
    VoteWeightSourceNotSupported, // 559

    /// Legacy1
    #[error("Legacy1")]
    Legacy1, // 560

    /// Governance PDA must sign
    #[error("Governance PDA must sign")]
    GovernancePdaMustSign, // 561

    /// Previously TransactionAlreadyFlaggedWithError
    #[error("Legacy2")]
    Legacy2, // 562

    /// Invalid Realm for Governance
    #[error("Invalid Realm for Governance")]
    InvalidRealmForGovernance, // 563

    /// Invalid Authority for Realm
    #[error("Invalid Authority for Realm")]
    InvalidAuthorityForRealm, // 564

    /// Realm has no authority
    #[error("Realm has no authority")]
    RealmHasNoAuthority, // 565

    /// Realm authority must sign
    #[error("Realm authority must sign")]
    RealmAuthorityMustSign, // 566

    /// Invalid governing token holding account
    #[error("Invalid governing token holding account")]
    InvalidGoverningTokenHoldingAccount, // 567

    /// Realm council mint change is not supported
    #[error("Realm council mint change is not supported")]
    RealmCouncilMintChangeIsNotSupported, // 568

    /// Invalid max voter weight absolute value
    #[error("Invalid max voter weight absolute value")]
    InvalidMaxVoterWeightAbsoluteValue, // 569

    /// Invalid max voter weight supply fraction
    #[error("Invalid max voter weight supply fraction")]
    InvalidMaxVoterWeightSupplyFraction, // 570

    /// Owner doesn't have enough governing tokens to create Governance
    #[error("Owner doesn't have enough governing tokens to create Governance")]
    NotEnoughTokensToCreateGovernance, // 571

    /// Too many outstanding proposals
    #[error("Too many outstanding proposals")]
    TooManyOutstandingProposals, // 572

    /// All proposals must be finalized to withdraw governing tokens
    #[error("All proposals must be finalized to withdraw governing tokens")]
    AllProposalsMustBeFinalisedToWithdrawGoverningTokens, // 573

    /// Invalid VoterWeightRecord for Realm
    #[error("Invalid VoterWeightRecord for Realm")]
    InvalidVoterWeightRecordForRealm, // 574

    /// Invalid VoterWeightRecord for GoverningTokenMint
    #[error("Invalid VoterWeightRecord for GoverningTokenMint")]
    InvalidVoterWeightRecordForGoverningTokenMint, // 575

    /// Invalid VoterWeightRecord for TokenOwner
    #[error("Invalid VoterWeightRecord for TokenOwner")]
    InvalidVoterWeightRecordForTokenOwner, // 576

    /// VoterWeightRecord expired
    #[error("VoterWeightRecord expired")]
    VoterWeightRecordExpired, // 577

    /// Invalid RealmConfig for Realm
    #[error("Invalid RealmConfig for Realm")]
    InvalidRealmConfigForRealm, // 578

    /// TokenOwnerRecord already exists
    #[error("TokenOwnerRecord already exists")]
    TokenOwnerRecordAlreadyExists, // 579

    /// Governing token deposits not allowed
    #[error("Governing token deposits not allowed")]
    GoverningTokenDepositsNotAllowed, // 580

    /// Invalid vote choice weight percentage
    #[error("Invalid vote choice weight percentage")]
    InvalidVoteChoiceWeightPercentage, // 581

    /// Vote type not supported
    #[error("Vote type not supported")]
    VoteTypeNotSupported, // 582

    /// InvalidProposalOptions
    #[error("Invalid proposal options")]
    InvalidProposalOptions, // 583

    /// Proposal is not executable
    #[error("Proposal is not executable")]
    ProposalIsNotExecutable, // 584

    /// Deny vote is not allowed
    #[error("Deny vote is not allowed")]
    DenyVoteIsNotAllowed, // 585

    /// Cannot execute defeated option
    #[error("Cannot execute defeated option")]
    CannotExecuteDefeatedOption, // 586

    /// VoterWeightRecord invalid action
    #[error("VoterWeightRecord invalid action")]
    VoterWeightRecordInvalidAction, // 587

    /// VoterWeightRecord invalid action target
    #[error("VoterWeightRecord invalid action target")]
    VoterWeightRecordInvalidActionTarget, // 588

    /// Invalid MaxVoterWeightRecord for Realm
    #[error("Invalid MaxVoterWeightRecord for Realm")]
    InvalidMaxVoterWeightRecordForRealm, // 589

    /// Invalid MaxVoterWeightRecord for GoverningTokenMint
    #[error("Invalid MaxVoterWeightRecord for GoverningTokenMint")]
    InvalidMaxVoterWeightRecordForGoverningTokenMint, // 590

    /// MaxVoterWeightRecord expired
    #[error("MaxVoterWeightRecord expired")]
    MaxVoterWeightRecordExpired, // 591

    /// Not supported VoteType
    #[error("Not supported VoteType")]
    NotSupportedVoteType, // 592

    /// RealmConfig change not allowed
    #[error("RealmConfig change not allowed")]
    RealmConfigChangeNotAllowed, // 593

    /// GovernanceConfig change not allowed
    #[error("GovernanceConfig change not allowed")]
    GovernanceConfigChangeNotAllowed, // 594

    /// At least one VoteThreshold is required
    #[error("At least one VoteThreshold is required")]
    AtLeastOneVoteThresholdRequired, // 595

    /// Reserved buffer must be empty
    #[error("Reserved buffer must be empty")]
    ReservedBufferMustBeEmpty, // 596

    /// Cannot Relinquish in Finalizing state
    #[error("Cannot Relinquish in Finalizing state")]
    CannotRelinquishInFinalizingState, // 597

    /// Invalid RealmConfig account address
    #[error("Invalid RealmConfig account address")]
    InvalidRealmConfigAddress, // 598

    /// Cannot deposit dormant tokens
    #[error("Cannot deposit dormant tokens")]
    CannotDepositDormantTokens, // 599

    /// Cannot withdraw membership tokens
    #[error("Cannot withdraw membership tokens")]
    CannotWithdrawMembershipTokens, // 600

    /// Cannot revoke GoverningTokens
    #[error("Cannot revoke GoverningTokens")]
    CannotRevokeGoverningTokens, // 601

    /// Invalid Revoke amount
    #[error("Invalid Revoke amount")]
    InvalidRevokeAmount, // 602

    /// Invalid GoverningToken source
    #[error("Invalid GoverningToken source")]
    InvalidGoverningTokenSource, // 603

    /// Cannot change community TokenType to Membership
    #[error("Cannot change community TokenType to Membership")]
    CannotChangeCommunityTokenTypeToMembership, // 604

    /// Voter weight threshold disabled
    #[error("Voter weight threshold disabled")]
    VoterWeightThresholdDisabled, // 605

    /// Vote not allowed in cool off time
    #[error("Vote not allowed in cool off time")]
    VoteNotAllowedInCoolOffTime, // 606

    /// Cannot refund ProposalDeposit
    #[error("Cannot refund ProposalDeposit")]
    CannotRefundProposalDeposit, // 607

    ///Invalid Proposal for ProposalDeposit
    #[error("Invalid Proposal for ProposalDeposit")]
    InvalidProposalForProposalDeposit, // 608

    /// Invalid deposit_exempt_proposal_count
    #[error("Invalid deposit_exempt_proposal_count")]
    InvalidDepositExemptProposalCount, // 609

    /// GoverningTokenMint not allowed to vote
    #[error("GoverningTokenMint not allowed to vote")]
    GoverningTokenMintNotAllowedToVote, // 610

    ///Invalid deposit Payer for ProposalDeposit
    #[error("Invalid deposit Payer for ProposalDeposit")]
    InvalidDepositPayerForProposalDeposit, // 611

    /// Invalid State: Proposal is not in final state
    #[error("Invalid State: Proposal is not in final state")]
    InvalidStateNotFinal, // 612

    ///Invalid state for proposal state transition to Completed
    #[error("Invalid state for proposal state transition to Completed")]
    InvalidStateToCompleteProposal, // 613

    /// Invalid number of vote choices
    #[error("Invalid number of vote choices")]
    InvalidNumberOfVoteChoices, // 614

    /// Ranked vote is not supported
    #[error("Ranked vote is not supported")]
    RankedVoteIsNotSupported, // 615

    /// Choice weight must be 100%
    #[error("Choice weight must be 100%")]
    ChoiceWeightMustBe100Percent, // 616

    /// Single choice only is allowed
    #[error("Single choice only is allowed")]
    SingleChoiceOnlyIsAllowed, // 617

    /// At least single choice is required
    #[error("At least single choice is required")]
    AtLeastSingleChoiceIsRequired, // 618

    /// Total vote weight must be 100%
    #[error("Total vote weight must be 100%")]
    TotalVoteWeightMustBe100Percent, // 619

    /// Invalid multi choice proposal parameters
    #[error("Invalid multi choice proposal parameters")]
    InvalidMultiChoiceProposalParameters, // 620

    /// Invalid Governance for RequiredSignatory
    #[error("Invalid Governance for RequiredSignatory")]
    InvalidGovernanceForRequiredSignatory, // 621

    /// SignatoryRecord already exists
    #[error("Signatory Record has already been created")]
    SignatoryRecordAlreadyExists, // 622

    /// Instruction has been removed
    #[error("Instruction has been removed")]
    InstructionDeprecated, // 623

    /// Proposal is missing signatories required by its governance
    #[error("Proposal is missing required signatories")]
    MissingRequiredSignatories, // 624

    /// TokenOwnerRecordLock authority must sign
    #[error("TokenOwnerRecordLock authority must sign")]
    TokenOwnerRecordLockAuthorityMustSign, // 625

    /// TokenOwnerRecordLock is expired
    #[error("TokenOwnerRecordLock is expired ")]
    ExpiredTokenOwnerRecordLock, // 626

    /// TokenOwnerRecord locked
    #[error("TokenOwnerRecord locked")]
    TokenOwnerRecordLocked, // 627

    /// Invalid TokenOwnerRecordLockAuthority
    #[error("Invalid TokenOwnerRecordLockAuthority")]
    InvalidTokenOwnerRecordLockAuthority, // 628

    /// TokenOwnerRecordLock authority already exists
    #[error("TokenOwnerRecordLock authority already exists")]
    TokenOwnerRecordLockAuthorityAlreadyExists, // 629

    /// TokenOwnerRecordLock not found
    #[error("TokenOwnerRecordLock not found")]
    TokenOwnerRecordLockNotFound, // 630

    /// TokenOwnerRecordLockAuthority not found
    #[error("TokenOwnerRecordLockAuthority not found")]
    TokenOwnerRecordLockAuthorityNotFound, // 631
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
