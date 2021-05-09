//! Program instructions

use solana_program::{epoch_schedule::Slot, instruction::Instruction};

use crate::state::Vote;

/// Instructions supported by the Governance program
#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum GovernanceInstruction {
    /// Creates Governance Realm account which aggregates governances for given Governance Mint and optional Council Mint.
    ///
    /// 1. `[writable]` Governance Realm account.
    /// 2. `[]` Governance Token Mint.
    /// 3. `[writable, signer]` Governances Token Holding account.
    /// 4. `[signer]` Payer.
    /// 5. `[]` System.
    /// 6. `[]` SPL Token.
    /// 7. `[]` Sysvar Rent.
    /// 8. `[]` Council Token mint - optional.
    /// 9. `[writable, signer]` Council Token Holding account - optional.
    CreateGovernanceRealm {
        /// UTF-8 encoded Governance Realm name.
        name: String,
    },

    /// Deposits governing tokens (Governance or Council) to Governance Realm and establishes your voter weight to be used for voting within the Realm.
    /// Note: If subsequent (top up) deposit is made and there are active votes for the Voter then the weights won't be updated automatically.
    /// It can be done by relinquishing votes on active Proposals and voting again with the new weight.
    ///
    ///  0. `[]` Governance Realm account.
    ///  1. `[]` Governing Token mint.
    ///  2. `[writable]` Governing Token Holding account.
    ///  3. `[writable]` Governing Token Source account.
    ///  4. `[writable]` Voter Record account.
    ///  4. `[signer]` Voter (payer).
    ///  5. `[]` System.
    ///  6. `[]` SPL Token.
    ///  7. `[]` Sysvar Rent.
    DepositGoverningTokens {
        /// Amount of Governing tokens to deposit. If None then all tokens from Source Account would be deposited.
        amount: Option<u64>,
    },

    /// Withdraws governing tokens (Governance or Council) from Governance Realm and updates your voter weight within the Realm.
    /// Note: It's only possible to withdraw tokens if the Voter doesn't have any outstanding active votes.
    /// If there are any outstanding votes then they must be relinquished before tokens could be withdrawn.
    ///
    ///  0. `[]` Governance Realm account.
    ///  1. `[]` Governing Token mint.
    ///  2. `[writable]` Governing Token Holding account.
    ///  3. `[writable]` Governing Token Source account.
    ///  4. `[writable]` Voter Record account.
    ///  6. `[]` SPL Token.
    ///  7. `[]` Sysvar Rent.   
    WithdrawGoverningTokens {
        /// Amount of Governing tokens to withdraw. If None then all Voter tokens from Holding Account would be withdrawn.
        amount: Option<u64>,
    },

    /// Creates Program Governance account which governs an upgradable program.
    ///
    ///   0. `[writable]` Governance account. The account pubkey needs to be set to program-derived address (PDA) with the following seeds:
    ///           1) 'governance' const prefix
    ///           2) Governed Program address.
    ///   1. `[]` Account of the Program governed by this Governance account.
    ///   2. `[writable]` Program Data account of the Program governed by this Governance account.
    ///   3. `[signer]` Current Upgrade Authority account of the Program governed by this Governance account.
    ///   4. `[]` Governance Realm the Program Governance belongs to.
    ///   5. `[signer]` Payer.
    ///   6. `[]` System account.
    ///   7. `[]` Bpf_upgrade_loader account.
    CreateProgramGovernance {
        /// Voting threshold in % required to tip the vote.
        /// It's the percentage of tokens out of the entire pool of governance tokens eligible to vote.
        vote_threshold: u8,

        /// Minimum waiting time in slots for an instruction to be executed after proposal is voted on.
        min_instruction_hold_up_time: Slot,

        /// Time limit in slots for proposal to be open for voting.
        max_voting_time: Slot,

        /// Minimum % of tokens for a governance token owner to be able to create proposal.
        /// It's the percentage of tokens out of the entire pool of governance tokens eligible to vote.
        token_threshold_to_create_proposal: u8,
    },

    /// Create Proposal account for Instructions that will be executed at various slots in the future.
    /// The instruction also grants Admin and Signatory token to the provided account.
    ///
    ///   0. `[writable]` Uninitialized Proposal State account.
    ///   1. `[writable]` Uninitialized Proposal account.
    ///   2. `[writable]` Initialized Governance account.
    ///   3. `[writable]` Initialized Signatory Mint account.
    ///   4. `[writable]` Initialized Admin Mint account.
    ///   5. `[writable]` Initialized Admin account for the issued admin token.
    ///   6. `[writable]` Initialized Signatory account for the issued signatory token.
    ///   7. `[writable]` Initialized Source Token Holding account.
    ///   8. `[]` Source mint account.
    ///   9. '[]` Token program account.
    ///   10. `[]` Rent sysvar.
    CreateProposal {
        /// Link to gist explaining proposal.
        description_link: String,
        /// UTF-8 encoded name of the proposal.
        name: String,
    },

    /// [Requires Admin token]
    /// Adds a signatory to the Proposal which means this Proposal can't leave Draft state until yet another Signatory signs.
    /// As a result of this call the new Signatory will receive a Signatory Token which then can be used to Sign proposal.
    ///
    ///   0. `[writable]` Initialized Signatory account.
    ///   1. `[writable]` Initialized Signatory Mint account.
    ///   2. `[signer]` Admin account.
    ///   3. `[writable]` Proposal State account.
    ///   4. `[]` Proposal account.
    ///   5. '[]` Token program account.
    AddSignatory,

    /// [Requires Admin token]
    /// Removes a Signatory from the Proposal.
    ///
    ///   0. `[writable]` Signatory account to remove token from.
    ///   1. `[writable]` Signatory Mint account.
    ///   2. `[signer]` Admin account.
    ///   3. `[writable]` Proposal State account.
    ///   4. `[]` Proposal account.
    ///   5. `[signer]` Transfer authority.
    ///   6. '[]` Token program account.
    RemoveSignatory,

    /// [Requires Admin token]
    /// Adds an instruction to the Proposal. Max of 5 of any  type. More than 5 will throw error.
    ///
    ///   0. `[writable]` Uninitialized Proposal Instruction account.
    ///   1. `[writable]` Proposal state account.
    ///   2. `[signer]` Admin account.
    ///   3. `[]` Proposal account.
    ///   4. `[]` Governance account.
    ///   5. `[]` Governance program account.
    AddCustomSingleSignerInstruction {
        /// Slot waiting time between vote period ending and this being eligible for execution.
        hold_up_time: Slot,
        /// Instruction.
        instruction: Instruction,
        /// Position in instruction array.
        position: u8,
    },

    /// [Requires Admin token]
    /// Remove instruction from the Proposal.
    ///
    ///   0. `[writable]` Proposal State account.
    ///   1. `[writable]` Proposal instruction account.
    ///   2. `[signer]` Admin account.
    ///   3. `[]` Proposal account.
    RemoveInstruction,

    /// [Requires Admin token]
    /// Update instruction hold up time in the Proposal.
    ///
    ///   0. `[writable]` Proposal instruction account.
    ///   1. `[signer]` Admin account.
    ///   2. `[]` Proposal State account.
    ///   3. `[]` Proposal account.
    UpdateInstructionHoldUpTime {
        /// Minimum waiting time in slots for an instruction to be executed after proposal is voted on.
        hold_up_time: Slot,
    },

    /// [Requires Admin token]
    /// Cancels Proposal and moves it into Canceled.
    ///
    ///   0. `[writable]` Proposal state account.
    ///   1. `[writable]` Admin account.
    ///   2. `[]` Proposal account.
    CancelProposal,

    /// [Requires Signatory token]
    /// Burns signatory token, indicating you approve of moving this Proposal from Draft state to Voting state.
    /// The last Signatory token to be burned moves the state to Voting.
    ///
    ///   0. `[writable]` Proposal State account.
    ///   1. `[writable]` Signatory account.
    ///   2. `[writable]` Signatory Mint account.
    ///   3. `[]` Proposal account.
    ///   4. `[signer]` Transfer authority.
    ///   5. `[]` Token program account.
    ///   6. `[]` Clock sysvar.
    SignProposal,

    /// [Requires Voting tokens]
    ///  Uses your voter weight (deposited Governance or Council tokens) to cast a vote on a Proposal,
    ///  By doing so you indicate you approve or disapprove of running the Proposal set of instructions.
    ///  If you tip the consensus then the instructions can begin to be run after their hold up time.
    ///
    ///   0. `[writable]` Governance Vote Record account. Needs to be set with pubkey set to PDA with seeds of the.
    ///                   1) 'governance' const prefix,
    ///                   2)  Voter account address
    ///                   3)  Proposal account address.    
    ///   1. `[]` Proposal account.
    ///   2. `[writable]` Proposal State account.
    ///   3. `[]` Governance account.
    ///   4. '[]' Voter Record account.
    ///   5. `[]` Token program account.
    ///   6. `[]` System account.
    ///   7. `[]` Clock sysvar.
    Vote {
        /// Yes/No  with amount of votes.
        vote: Vote,
    },

    ///  Relinquish Vote removes voter weight from a Proposal and removes it from voter's active votes.
    ///  If the Proposal is still being voted on then the voter's weight won't count towards the vote outcome.
    ///  If the Proposal is already in decided state then the instruction has no impact on the Proposal
    ///  and only allows voters to prune their outstanding votes in case they wanted to withdraw Governing tokens from the Realm
    ///
    ///   0. `[writable]` Governance Vote Record account.
    ///   1. `[writable]` Voter Record account.
    ///   2. `[]` Proposal State account
    ///   3. `[]` Proposal account
    RelinquishVote,

    /// Executes an instruction in the Proposal
    ///
    ///   0. `[writable]` Instruction account you wish to execute
    ///   1. `[writable]` Proposal State account
    ///   2. `[]` Program being invoked account
    ///   3. `[]` Proposal account
    ///   4. `[]` Governance account
    ///   5. `[]` Clock sysvar
    ///   6+ Any extra accounts that are part of the instruction, in order
    Execute,
}
