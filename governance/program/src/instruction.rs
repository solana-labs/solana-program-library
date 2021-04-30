//! Program instructions

use crate::state::{
    Vote, MAX_GOVERNANCE_NAME_LENGTH, MAX_PROPOSAL_DESCRIPTION_LINK_LENGTH,
    MAX_PROPOSAL_NAME_LENGTH,
};

/// Instructions supported by the Governance program
#[derive(Clone)]
#[allow(clippy::large_enum_variant)]
pub enum GovernanceInstruction {
    /// Creates Governance account
    ///
    ///   0. `[writable]` Governance account. The account pubkey needs to be set to program-derived address (PDA) with the following seeds:
    ///           1) 'governance' const prefix,
    ///           2) Governed Program account key
    ///   1. `[]` Account of the Program governed by this Governance account
    ///   2. `[writable]` Program Data account of the Program governed by this Governance account
    ///   3. `[signer]` Current Upgrade Authority account of the Program governed by this Governance account
    ///   4. `[]` Governance mint that this Governance uses
    ///   5. `[signer]` Payer
    ///   6. `[]` System account
    ///   7. `[]` Bpf_upgrade_loader account
    ///   8. `[]` Council mint that this Governance uses [Optional]
    CreateGovernance {
        /// Voting threshold in % required to tip the vote
        /// It's the percentage of tokens out of the entire pool of governance tokens eligible to vote
        vote_threshold: u8,

        /// Minimum waiting time in slots for a transaction to be executed after proposal is voted on
        min_transaction_hold_up_time: u64,

        /// Time limit in slots for proposal to be open for voting
        max_voting_time: u64,

        /// Minimum % of tokens for a governance token owner to be able to create proposal
        /// It's the percentage of tokens out of the entire pool of governance tokens eligible to vote
        token_threshold_to_create_proposal: u8,

        /// Governance name
        name: [u8; MAX_GOVERNANCE_NAME_LENGTH],
    },

    /// Initializes a new empty Proposal for Instructions that will be executed at various slots in the future
    /// The instruction also grants Admin and Signatory token to the provided account
    ///
    ///   0. `[writable]` Uninitialized Proposal State account
    ///   1. `[writable]` Uninitialized Proposal account
    ///   2. `[writable]` Initialized Governance account
    ///   3. `[writable]` Initialized Signatory Mint account
    ///   4. `[writable]` Initialized Admin Mint account
    ///   5. `[writable]` Initialized Admin account for the issued admin token
    ///   6. `[writable]` Initialized Signatory account for the issued signatory token
    ///   7. `[writable]` Initialized Source Token Holding account
    ///   8. `[]` Source mint account
    ///   9. `[]` Proposal Authority account. PDA with seeds: ['governance',proposal_key]
    ///   10. '[]` Token program account
    ///   11. `[]` Rent sysvar
    InitializeProposal {
        /// Link to gist explaining proposal
        description_link: [u8; MAX_PROPOSAL_DESCRIPTION_LINK_LENGTH],
        /// Name of the proposal
        name: [u8; MAX_PROPOSAL_NAME_LENGTH],
    },

    /// [Requires Admin token]
    /// Adds a signatory to the Proposal which means that this Proposal can't leave Draft state until yet another signatory burns
    /// their signatory token indicating they are satisfied with the Proposal instructions.
    /// They'll receive an signatory token as a result of this call that they can burn later
    ///
    ///   0. `[writable]` Initialized Signatory account
    ///   1. `[writable]` Initialized Signatory Mint account
    ///   2. `[writable]` Admin account
    ///   3. `[writable]` Proposal State account
    ///   4. `[]` Proposal account
    ///   5. `[]` Transfer authority
    ///   6. `[]` Proposal Authority account. PDA with seeds: ['governance',proposal_key]
    ///   7. '[]` Token program account
    AddSignatory,

    /// [Requires Admin token]
    /// Removes a signer from the Proposal
    ///
    ///   0. `[writable]` Signatory account to remove token from
    ///   1. `[writable]` Signatory Mint account
    ///   2. `[writable]` Admin account
    ///   3. `[writable]` Proposal State account
    ///   4. `[]` Proposal account
    ///   5. `[]` Transfer authority
    ///   6. `[]` Proposal Authority account. PDA with seeds: ['governance',proposal_key]
    ///   7. '[]` Token program account
    RemoveSignatory,

    /// [Requires Signatory token]
    /// Adds a Transaction to the Proposal Max of 5 of any Transaction type. More than 5 will throw error.
    /// Creates a PDA using your authority to be used to later execute the instruction.
    /// This transaction needs to contain authority to execute the program.
    ///
    ///   0. `[writable]` Uninitialized Proposal Transaction account
    ///   1. `[writable]` Proposal state account
    ///   2. `[writable]` Signatory account
    ///   3. `[]` Proposal account.
    ///   4. `[]` Governance account
    ///   5. `[]` Transfer authority
    ///   6. `[]` Proposal Authority account. PDA with seeds: ['governance',proposal_key]
    ///   7. `[]` Governance program account
    AddCustomSingleSignerTransaction {
        /// Slot waiting time between vote period ending and this being eligible for execution
        hold_up_time: u64,
        /// Instruction
        instruction: Vec<u8>,
        /// Position in transaction array
        position: u8,
    },

    /// [Requires Signatory token]
    /// Remove Transaction from the Proposal
    ///
    ///   0. `[writable]` Proposal State account
    ///   1. `[writable]` Proposal Transaction account
    ///   2. `[writable]` Signatory account
    ///   3. `[]` Proposal account
    ///   4. `[]` Transfer Authority
    ///   5. `[]` Proposal Authority account. PDA with seeds: ['governance',proposal_key]
    RemoveTransaction,

    /// [Requires Signatory token]
    /// Update Transaction cool off time in the Proposal.
    ///
    ///   0. `[writable]` Proposal Transaction account
    ///   1. `[writable]` Signatory account
    ///   2. `[]` Proposal State account
    ///   3. `[]` Proposal account
    ///   4. `[]` Transfer authority
    ///   5. `[]` Proposal Authority account. PDA with seeds: ['governance',proposal_key]
    UpdateTransactionHoldUpTime {
        /// Minimum waiting time in slots for a transaction to be executed after proposal is voted on
        hold_up_time: u64,
    },

    /// [Requires Admin token]
    /// Cancels Proposal and moves it into Canceled
    ///
    ///   0. `[writable]` Proposal state account
    ///   1. `[writable]` Admin account
    ///   2. `[]` Proposal account
    ///   3. `[]` Transfer authority
    ///   4. `[]` Proposal Authority account. PDA with seeds: ['governance',proposal_key]
    CancelProposal,

    /// [Requires Signatory token]
    /// Burns signatory token, indicating you approve of moving this Proposal from Draft state to Voting state.
    /// The last Signatory token to be burned moves the state to Voting.
    ///
    ///   0. `[writable]` Proposal State account
    ///   1. `[writable]` Signatory account
    ///   2. `[writable]` Signatory Mint account
    ///   3. `[]` Proposal account
    ///   4. `[]` Transfer authority
    ///   5. `[]` Proposal Authority account. PDA with seeds: ['governance',proposal_key]
    ///   6. `[]` Token program account.
    ///   7. `[]` Clock sysvar.
    SignProposal,

    /// [Requires Voting tokens]
    ///  Deposits source voting tokens (governance or council), indicating you approve and/or disapprove of running this set of transactions
    /// If you tip the consensus then the transactions can begin to be run after their hold up time
    ///
    ///   0. `[writable]` Governance Vote Record account. Needs to be set with pubkey set to PDA with seeds of the
    ///                   1) 'governance' const prefix,
    ///                   2)  Voter account
    ///                   3)  Proposal account     
    ///   3. `[]` Proposal account
    ///   1. `[writable]` Proposal State account
    ///   2. `[]` Source Token Mint account
    ///   1. `[writable]` User Source Token account to deposit tokens from
    ///   2. `[writable]` Source Token Holding account for Proposal that will accept the tokens in escrow
    ///   4. `[]` Governance account
    ///   5. `[]` Transfer authority
    ///   6. `[]` Proposal Authority account. PDA with seeds: ['governance',proposal_key]
    ///   7. `[]` Token program account
    ///   3. `[]` System account
    ///   8. `[]` Clock sysvar
    Vote {
        /// Yes/No  with amount of votes
        vote: Vote,
    },

    /// Executes a transaction in the Proposal
    ///
    ///   0. `[writable]` Transaction account you wish to execute
    ///   1. `[writable]` Proposal State account
    ///   2. `[]` Program being invoked account
    ///   3. `[]` Proposal account
    ///   4. `[]` Governance account
    ///   5. `[]` Governance program account pub key
    ///   6. `[]` Clock sysvar
    ///   7+ Any extra accounts that are part of the instruction, in order
    Execute,

    /// [Requires voting tokens]
    /// Withdraws source tokens from the Proposal.
    /// If the Proposal is still being voted on then the tokens won't count towards the vote outcome
    ///
    ///   0. `[writable]` Governance Vote Record account. See Vote docs for more detail
    ///   4. `[writable]` User Source Token account that you wish your actual tokens to be returned to
    ///   5. `[writable]` Source Token Holding account owned by the Governance that will has the actual tokens in escrow
    ///   9. `[]` Proposal State account
    ///   10. `[]` Proposal account
    ///   11. `[]` Transfer authority
    ///   12. `[]` Proposal Authority account. PDA with seeds: ['governance',proposal_key]
    ///   13. `[]` Token program account
    WithdrawSourceTokens,
}
