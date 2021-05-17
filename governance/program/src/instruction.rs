//! Program instructions

use solana_program::{instruction::Instruction, pubkey::Pubkey};

use crate::state::enums::GoverningTokenType;

/// Yes/No Vote
#[repr(C)]
#[derive(Clone, Debug, PartialEq)]
pub enum Vote {
    /// Yes vote
    Yes,
    /// No vote
    No,
}

/// Instructions supported by the Governance program
#[derive(Clone)]
#[repr(C)]
#[allow(clippy::large_enum_variant)]
pub enum GovernanceInstruction {
    /// Creates Governance Realm account which aggregates governances for given Community Mint and optional Council Mint
    ///
    /// 0. `[writable]` Governance Realm account. PDA seeds:['governance',name]
    /// 1. `[]` Community Token Mint
    /// 2. `[writable]` Community Token Holding account. PDA seeds: ['governance',realm,community_mint]
    ///     The account will be created with the Realm PDA as its owner
    /// 3. `[signer]` Payer
    /// 4. `[]` System
    /// 5. `[]` SPL Token
    /// 6. `[]` Sysvar Rent
    /// 7. `[]` Council Token Mint - optional
    /// 8. `[writable]` Council Token Holding account - optional. . PDA seeds: ['governance',realm,council_mint]
    ///     The account will be created with the Realm PDA as its owner
    CreateRealm {
        /// UTF-8 encoded Governance Realm name
        name: String,
    },

    /// Deposits governing tokens (Community or Council) to Governance Realm and establishes your voter weight to be used for voting within the Realm
    /// Note: If subsequent (top up) deposit is made and there are active votes for the Voter then the vote weights won't be updated automatically
    /// It can be done by relinquishing votes on active Proposals and voting again with the new weight
    ///
    ///  0. `[]` Governance Realm account
    ///  1. `[writable]` Governing Token Holding account. PDA seeds: ['governance',realm, governing_token_mint]
    ///  2. `[writable]` Governing Token Source account. All tokens from the account will be transferred to the Holding account
    ///  3. `[signer]` Governing Token Owner account
    ///  4. `[writable]` Voter Record account. PDA seeds: ['governance',realm, governing_token_mint, governing_token_owner]
    ///  5. `[signer]` Payer
    ///  6. `[]` System
    ///  7. `[]` SPL Token
    DepositGoverningTokens {},

    /// Withdraws governing tokens (Community or Council) from Governance Realm and downgrades your voter weight within the Realm
    /// Note: It's only possible to withdraw tokens if the Voter doesn't have any outstanding active votes
    /// If there are any outstanding votes then they must be relinquished before tokens could be withdrawn
    ///
    ///  0. `[]` Governance Realm account
    ///  1. `[writable]` Governing Token Holding account. PDA seeds: ['governance',realm, governing_token_mint]
    ///  2. `[writable]` Governing Token Destination account. All tokens will be transferred to this account
    ///  3. `[signer]` Governing Token Owner account
    ///  4. `[writable]` Voter Record account. PDA seeds: ['governance',realm, governing_token_mint, governing_token_owner]
    ///  5. `[]` SPL Token   
    WithdrawGoverningTokens {},

    /// Sets vote authority for the given Realm and Governing Token Mint (Community or Council)
    /// The vote authority would have voting rights and could vote on behalf of the Governing Token Owner
    ///
    /// 0. `[signer]` Governing Token Owner
    /// 1. `[writable]` Voter Record
    SetVoteAuthority {
        #[allow(dead_code)]
        /// Governance Realm the new vote authority is set for
        realm: Pubkey,

        #[allow(dead_code)]
        /// Governing Token Mint the vote authority is granted over
        governing_token_mint: Pubkey,

        #[allow(dead_code)]
        /// New vote authority
        vote_authority: Pubkey,
    },

    /// Creates Program Governance account which governs an upgradable program
    ///
    ///   0. `[writable]` Governance account. PDA seeds: ['governance', governed_program]
    ///   1. `[]` Account of the Program governed by this Governance account
    ///   2. `[writable]` Program Data account of the Program governed by this Governance account
    ///   3. `[signer]` Current Upgrade Authority account of the Program governed by this Governance account
    ///   4. `[]` Governance Realm the Program Governance belongs to
    ///   5. `[signer]` Payer
    ///   6. `[]` System account
    ///   7. `[]` Bpf_upgrade_loader account
    CreateProgramGovernance {
        /// Voting threshold in % required to tip the vote
        /// It's the percentage of tokens out of the entire pool of governance tokens eligible to vote
        vote_threshold: u8,

        /// Minimum waiting time in slots for an instruction to be executed after proposal is voted on
        min_instruction_hold_up_time: u64,

        /// Time limit in slots for proposal to be open for voting
        max_voting_time: u64,

        /// Minimum % of tokens for a governance token owner to be able to create proposal
        /// It's the percentage of tokens out of the entire pool of governance tokens eligible to vote
        token_threshold_to_create_proposal: u8,
    },

    /// Create Proposal account for Instructions that will be executed at various slots in the future
    /// The instruction also grants Admin and Signatory token to the provided account
    ///
    ///   0. `[writable]` Uninitialized Proposal account
    ///   1. `[writable]` Initialized Governance account
    ///   2. `[writable]` Initialized Signatory Mint account
    ///   3. `[writable]` Initialized Admin Mint account
    ///   4. `[writable]` Initialized Admin account for the issued admin token
    ///   5. `[writable]` Initialized Signatory account for the issued signatory token
    ///   6. '[]` Token program account
    ///   7. `[]` Rent sysvar
    CreateProposal {
        /// Link to gist explaining proposal
        description_link: String,

        /// UTF-8 encoded name of the proposal
        name: String,

        /// The Governing token (Community or Council) which will be used for voting on the Proposal
        governing_token_type: GoverningTokenType,
    },

    /// [Requires Admin token]
    /// Adds a signatory to the Proposal which means this Proposal can't leave Draft state until yet another Signatory signs
    /// As a result of this call the new Signatory will receive a Signatory Token which then can be used to Sign proposal
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[writable]` Initialized Signatory account
    ///   2. `[writable]` Initialized Signatory Mint account
    ///   3. `[signer]` Admin account
    ///   4. '[]` Token program account
    AddSignatory,

    /// [Requires Admin token]
    /// Removes a Signatory from the Proposal
    ///
    ///   0. `[writable]` Proposal account   
    ///   1. `[writable]` Signatory account to remove token from
    ///   2. `[writable]` Signatory Mint account
    ///   3. `[signer]` Admin account
    ///   4. '[]` Token program account
    RemoveSignatory,

    /// [Requires Admin token]
    /// Adds an instruction to the Proposal. Max of 5 of any  type. More than 5 will throw error
    ///
    ///   0. `[writable]` Proposal account   
    ///   1. `[writable]` Uninitialized Proposal SingleSignerInstruction account
    ///   2. `[signer]` Admin account
    AddSingleSignerInstruction {
        /// Slot waiting time between vote period ending and this being eligible for execution
        hold_up_time: u64,

        /// Instruction
        instruction: Instruction,

        /// Position in instruction array
        position: u8,
    },

    /// [Requires Admin token]
    /// Remove instruction from the Proposal
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[writable]` Proposal SingleSignerInstruction account
    ///   2. `[signer]` Admin account
    RemoveInstruction,

    /// [Requires Admin token]
    /// Update instruction hold up time in the Proposal
    ///
    ///   0. `[]` Proposal account   
    ///   1. `[writable]` Proposal SingleSignerInstruction account
    ///   2. `[signer]` Admin account
    UpdateInstructionHoldUpTime {
        /// Minimum waiting time in slots for an instruction to be executed after proposal is voted on
        hold_up_time: u64,
    },

    /// [Requires Admin token]
    /// Cancels Proposal and moves it into Canceled
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[writable]` Admin account
    CancelProposal,

    /// [Requires Signatory token]
    /// Burns signatory token, indicating you approve and sign off on moving this Proposal from Draft state to Voting state
    /// The last Signatory token to be burned moves the state to Voting
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[writable]` Signatory account
    ///   2. `[writable]` Signatory Mint account
    ///   3. `[]` Token program account
    ///   4. `[]` Clock sysvar
    SignOffProposal,

    ///  Uses your voter weight (deposited Community or Council tokens) to cast a vote on a Proposal
    ///  By doing so you indicate you approve or disapprove of running the Proposal set of instructions
    ///  If you tip the consensus then the instructions can begin to be run after their hold up time
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[writable]` Voter Record account. PDA seeds: ['governance',realm, governing_token_mint, governing_token_owner]
    ///   2. `[writable]` Proposal Vote Record account. PDA seeds: ['governance',proposal,governing_token_owner]  
    ///   3. `[signer]` Vote Authority account
    ///   4. `[]` Governance account
    Vote {
        /// Yes/No vote
        vote: Vote,
    },

    ///  Relinquish Vote removes voter weight from a Proposal and removes it from voter's active votes
    ///  If the Proposal is still being voted on then the voter's weight won't count towards the vote outcome
    ///  If the Proposal is already in decided state then the instruction has no impact on the Proposal
    ///  and only allows voters to prune their outstanding votes in case they wanted to withdraw Governing tokens from the Realm
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[writable]` Voter Record account. PDA seeds: ['governance',realm, governing_token_mint, governing_token_owner]
    ///   2. `[writable]` Proposal Vote Record account. PDA seeds: ['governance',proposal,governing_token_owner]
    ///   3. `[signer]` Vote Authority account
    RelinquishVote,

    /// Executes an instruction in the Proposal
    /// Anybody can execute transaction once Proposal has been voted Yes and transaction_hold_up time has passed
    /// The actual instruction being executed will be signed by Governance PDA
    /// For example to execute Program upgrade the ProgramGovernance PDA would be used as the singer
    ///
    ///   0. `[writable]` Proposal account   
    ///   1. `[writable]` Instruction account you wish to execute
    ///   2. `[]` Program being invoked account
    ///   3. `[]` Governance account (PDA)
    ///   4. `[]` Clock sysvar
    ///   5+ Any extra accounts that are part of the instruction, in order
    Execute,
}
