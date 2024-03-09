//! Program instructions

use {
    crate::state::{
        enums::MintMaxVoterWeightSource,
        governance::{get_governance_address, GovernanceConfig},
        native_treasury::get_native_treasury_address,
        program_metadata::get_program_metadata_address,
        proposal::{get_proposal_address, VoteType},
        proposal_deposit::get_proposal_deposit_address,
        proposal_transaction::{get_proposal_transaction_address, InstructionData},
        realm::{
            get_governing_token_holding_address, get_realm_address,
            GoverningTokenConfigAccountArgs, GoverningTokenConfigArgs, RealmConfigArgs,
            SetRealmAuthorityAction, SetRealmConfigItemArgs,
        },
        realm_config::get_realm_config_address,
        required_signatory::get_required_signatory_address,
        signatory_record::get_signatory_record_address,
        token_owner_record::get_token_owner_record_address,
        vote_record::{get_vote_record_address, Vote},
    },
    borsh::{BorshDeserialize, BorshSchema, BorshSerialize},
    solana_program::{
        clock::UnixTimestamp,
        instruction::{AccountMeta, Instruction},
        pubkey::Pubkey,
        system_program, sysvar,
    },
};

/// Instructions supported by the Governance program
#[derive(Clone, Debug, PartialEq, Eq, BorshDeserialize, BorshSerialize, BorshSchema)]
#[allow(clippy::large_enum_variant)]
pub enum GovernanceInstruction {
    /// Creates Governance Realm account which aggregates governances for given
    /// Community Mint and optional Council Mint
    ///
    /// 0. `[writable]` Governance Realm account.
    ///     * PDA seeds:['governance',name]
    /// 1. `[]` Realm authority
    /// 2. `[]` Community Token Mint
    /// 3. `[writable]` Community Token Holding account.
    ///     * PDA seeds: ['governance',realm,community_mint]
    ///     The account will be created with the Realm PDA as its owner
    /// 4. `[signer]` Payer
    /// 5. `[]` System
    /// 6. `[]` SPL Token
    /// 7. `[]` Sysvar Rent
    /// 8. `[]` Council Token Mint - optional
    /// 9. `[writable]` Council Token Holding account - optional unless council
    ///    is used.
    ///     * PDA seeds: ['governance',realm,council_mint]
    ///     The account will be created with the Realm PDA as its owner
    /// 10. `[writable]` RealmConfig account.
    ///     * PDA seeds: ['realm-config', realm]
    /// 11. `[]` Optional Community Voter Weight Addin Program Id
    /// 12. `[]` Optional Max Community Voter Weight Addin Program Id
    /// 13. `[]` Optional Council Voter Weight Addin Program Id
    /// 14. `[]` Optional Max Council Voter Weight Addin Program Id
    CreateRealm {
        #[allow(dead_code)]
        /// UTF-8 encoded Governance Realm name
        name: String,

        #[allow(dead_code)]
        /// Realm config args
        config_args: RealmConfigArgs,
    },

    /// Deposits governing tokens (Community or Council) to Governance Realm and
    /// establishes your voter weight to be used for voting within the Realm
    /// Note: If subsequent (top up) deposit is made and there are active votes
    /// for the Voter then the vote weights won't be updated automatically
    /// It can be done by relinquishing votes on active Proposals and voting
    /// again with the new weight
    ///
    ///  0. `[]` Realm account
    ///  1. `[writable]` Governing Token Holding account.
    ///     * PDA seeds: ['governance',realm, governing_token_mint]
    ///  2. `[writable]` Governing Token Source account. It can be either
    ///     spl-token TokenAccount or MintAccount Tokens will be transferred or
    ///     minted to the Holding account
    ///  3. `[signer]` Governing Token Owner account
    ///  4. `[signer]` Governing Token Source account authority It should be
    ///     owner for TokenAccount and mint_authority for MintAccount
    ///  5. `[writable]` TokenOwnerRecord account.
    ///     * PDA seeds: ['governance',realm, governing_token_mint,
    ///       governing_token_owner]
    ///  6. `[signer]` Payer
    ///  7. `[]` System
    ///  8. `[]` SPL Token program
    ///  9. `[]` RealmConfig account.
    ///     * PDA seeds: ['realm-config', realm]
    DepositGoverningTokens {
        /// The amount to deposit into the realm
        #[allow(dead_code)]
        amount: u64,
    },

    /// Withdraws governing tokens (Community or Council) from Governance Realm
    /// and downgrades your voter weight within the Realm.
    /// Note: It's only possible to withdraw tokens if the Voter doesn't have
    /// any outstanding active votes.
    /// If there are any outstanding votes then they must be relinquished
    /// before tokens could be withdrawn
    ///
    ///  0. `[]` Realm account
    ///  1. `[writable]` Governing Token Holding account.
    ///     * PDA seeds: ['governance',realm, governing_token_mint]
    ///  2. `[writable]` Governing Token Destination account. All tokens will be
    ///     transferred to this account
    ///  3. `[signer]` Governing Token Owner account
    ///  4. `[writable]` TokenOwnerRecord account.
    ///     * PDA seeds: ['governance',realm, governing_token_mint,
    ///       governing_token_owner]
    ///  5. `[]` SPL Token program
    ///  6. `[]` RealmConfig account.
    ///     * PDA seeds: ['realm-config', realm]
    WithdrawGoverningTokens {},

    /// Sets Governance Delegate for the given Realm and Governing Token Mint
    /// (Community or Council). The Delegate would have voting rights and
    /// could vote on behalf of the Governing Token Owner. The Delegate would
    /// also be able to create Proposals on behalf of the Governing Token
    /// Owner.
    /// Note: This doesn't take voting rights from the Token Owner who still can
    /// vote and change governance_delegate
    ///
    /// 0. `[signer]` Current Governance Delegate or Governing Token owner
    /// 1. `[writable]` Token Owner  Record
    SetGovernanceDelegate {
        #[allow(dead_code)]
        /// New Governance Delegate
        new_governance_delegate: Option<Pubkey>,
    },

    /// Creates Governance account which can be used to govern any arbitrary
    /// Solana account or asset
    ///
    ///   0. `[]` Realm account the created Governance belongs to
    ///   1. `[writable]` Governance account
    ///     * PDA seeds: ['account-governance', realm, governance_seed]
    ///   2. `[]` Governance account PDA seed
    ///   3. `[]` Governing TokenOwnerRecord account (Used only if not signed by
    ///      RealmAuthority)
    ///   4. `[signer]` Payer
    ///   5. `[]` System program
    ///   6. `[signer]` Governance authority
    ///   7. `[]` RealmConfig account.
    ///     * PDA seeds: ['realm-config', realm]
    ///   8. `[]` Optional Voter Weight Record
    CreateGovernance {
        /// Governance config
        #[allow(dead_code)]
        config: GovernanceConfig,
    },

    /// Legacy CreateProgramGovernance instruction
    /// Exists for backwards-compatibility
    Legacy4,

    /// Creates Proposal account for Transactions which will be executed at some
    /// point in the future
    ///
    ///   0. `[]` Realm account the created Proposal belongs to
    ///   1. `[writable]` Proposal account.
    ///     * PDA seeds ['governance',governance, governing_token_mint,
    ///       proposal_seed]
    ///   2. `[writable]` Governance account
    ///   3. `[writable]` TokenOwnerRecord account of the Proposal owner
    ///   4. `[]` Governing Token Mint the Proposal is created for
    ///   5. `[signer]` Governance Authority (Token Owner or Governance
    ///      Delegate)
    ///   6. `[signer]` Payer
    ///   7. `[]` System program
    ///   8. `[]` RealmConfig account.
    ///     * PDA seeds: ['realm-config', realm]
    ///   9. `[]` Optional Voter Weight Record
    ///   10.`[writable]` Optional ProposalDeposit account.
    ///     * PDA seeds: ['proposal-deposit', proposal, deposit payer]
    ///     Proposal deposit is required when there are more active proposals
    ///     than the configured deposit exempt amount.
    ///     The deposit is paid by the Payer of the transaction and can be
    ///     reclaimed using RefundProposalDeposit once the Proposal is no
    ///     longer active.
    CreateProposal {
        #[allow(dead_code)]
        /// UTF-8 encoded name of the proposal
        name: String,

        #[allow(dead_code)]
        /// Link to a gist explaining the proposal
        description_link: String,

        #[allow(dead_code)]
        /// Proposal vote type
        vote_type: VoteType,

        #[allow(dead_code)]
        /// Proposal options
        options: Vec<String>,

        #[allow(dead_code)]
        /// Indicates whether the proposal has the deny option
        /// A proposal without the rejecting option is a non binding survey
        /// Only proposals with the rejecting option can have executable
        /// transactions
        use_deny_option: bool,

        #[allow(dead_code)]
        /// Unique seed for the Proposal PDA
        proposal_seed: Pubkey,
    },

    /// Adds a signatory to the Proposal which means this Proposal can't leave
    /// Draft state until yet another Signatory signs
    ///
    ///   0. `[]` Governance account
    ///   1. `[writable]` Proposal account associated with the governance
    ///   2. `[writable]` Signatory Record Account
    ///   3. `[signer]` Payer
    ///   4. `[]` System program
    ///   Either:
    ///      - 5. `[]` TokenOwnerRecord account of the Proposal owner
    ///        6. `[signer]` Governance Authority (Token Owner or Governance
    ///           Delegate)
    ///
    ///      - 5. `[]` RequiredSignatory account associated with the governance.
    AddSignatory {
        #[allow(dead_code)]
        /// Signatory to add to the Proposal
        signatory: Pubkey,
    },

    /// Formerly RemoveSignatory. Exists for backwards-compatibility.
    Legacy1,

    /// Inserts Transaction with a set of instructions for the Proposal at the
    /// given index position New Transaction must be inserted at the end of
    /// the range indicated by Proposal transactions_next_index
    /// If a Transaction replaces an existing Transaction at a given index then
    /// the old one must be removed using RemoveTransaction first

    ///   0. `[]` Governance account
    ///   1. `[writable]` Proposal account
    ///   2. `[]` TokenOwnerRecord account of the Proposal owner
    ///   3. `[signer]` Governance Authority (Token Owner or Governance
    ///      Delegate)
    ///   4. `[writable]` ProposalTransaction, account.
    ///     * PDA seeds: ['governance', proposal, option_index, index]
    ///   5. `[signer]` Payer
    ///   6. `[]` System program
    ///   7. `[]` Rent sysvar
    InsertTransaction {
        #[allow(dead_code)]
        /// The index of the option the transaction is for
        option_index: u8,
        #[allow(dead_code)]
        /// Transaction index to be inserted at.
        index: u16,
        #[allow(dead_code)]
        /// Legacy hold_up_time
        legacy: u32,

        #[allow(dead_code)]
        /// Instructions Data
        instructions: Vec<InstructionData>,
    },

    /// Removes Transaction from the Proposal
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[]` TokenOwnerRecord account of the Proposal owner
    ///   2. `[signer]` Governance Authority (Token Owner or Governance
    ///      Delegate)
    ///   3. `[writable]` ProposalTransaction, account
    ///   4. `[writable]` Beneficiary Account which would receive lamports from
    ///      the disposed ProposalTransaction account
    RemoveTransaction,

    /// Cancels Proposal by changing its state to Canceled
    ///
    ///   0. `[]` Realm account
    ///   1. `[writable]` Governance account
    ///   2. `[writable]` Proposal account
    ///   3. `[writable]`  TokenOwnerRecord account of the  Proposal owner
    ///   4. `[signer]` Governance Authority (Token Owner or Governance
    ///      Delegate)
    CancelProposal,

    /// Signs off Proposal indicating the Signatory approves the Proposal
    /// When the last Signatory signs off the Proposal it enters Voting state
    /// Note: Adding signatories to a Proposal is a quality and not a security
    /// gate and it's entirely at the discretion of the Proposal owner
    /// If Proposal owner doesn't designate any signatories then can sign off
    /// the Proposal themself
    ///
    ///   0. `[]` Realm account
    ///   1. `[]` Governance account
    ///   2. `[writable]` Proposal account
    ///   3. `[signer]` Signatory account signing off the Proposal Or Proposal
    ///      owner if the owner hasn't appointed any signatories
    ///   4. `[]` TokenOwnerRecord for the Proposal owner, required when the
    ///      owner signs off the Proposal Or `[writable]` SignatoryRecord
    ///      account, required when non owner sings off the Proposal
    SignOffProposal,

    ///  Uses your voter weight (deposited Community or Council tokens) to cast
    /// a vote on a Proposal  By doing so you indicate you approve or
    /// disapprove of running the Proposal set of transactions  If you tip
    /// the consensus then the transactions can begin to be run after their hold
    /// up time
    ///
    ///   0. `[]` Realm account
    ///   1. `[writable]` Governance account
    ///   2. `[writable]` Proposal account
    ///   3. `[writable]` TokenOwnerRecord of the Proposal owner
    ///   4. `[writable]` TokenOwnerRecord of the voter.
    ///     * PDA seeds: ['governance',realm, vote_governing_token_mint,
    ///       governing_token_owner]
    ///   5. `[signer]` Governance Authority (Token Owner or Governance
    ///      Delegate)
    ///   6. `[writable]` Proposal VoteRecord account.
    ///     * PDA seeds: ['governance',proposal,token_owner_record]
    ///   7. `[]` The Governing Token Mint which is used to cast the vote
    ///      (vote_governing_token_mint).
    ///     The voting token mint is the governing_token_mint of the Proposal
    ///     for Approve, Deny and Abstain votes.
    ///     For Veto vote the voting token mint is the mint of the opposite
    ///     voting population Council mint to veto Community proposals and
    ///     Community mint to veto Council proposals.
    ///     Note: In the current version only Council veto is supported
    ///   8. `[signer]` Payer
    ///   9. `[]` System program
    ///   10. `[]` RealmConfig account.
    ///     * PDA seeds: ['realm-config', realm]
    ///   11. `[]` Optional Voter Weight Record
    ///   12. `[]` Optional Max Voter Weight Record
    CastVote {
        #[allow(dead_code)]
        /// User's vote
        vote: Vote,
    },

    /// Finalizes vote in case the Vote was not automatically tipped within
    /// max_voting_time period
    ///
    ///   0. `[]` Realm account
    ///   1. `[writable]` Governance account
    ///   2. `[writable]` Proposal account
    ///   3. `[writable]` TokenOwnerRecord of the Proposal owner
    ///   4. `[]` Governing Token Mint
    ///   5. `[]` RealmConfig account.
    ///     * PDA seeds: ['realm-config', realm]
    ///   6. `[]` Optional Max Voter Weight Record
    FinalizeVote {},

    ///  Relinquish Vote removes voter weight from a Proposal and removes it
    /// from voter's active votes. If the Proposal is still being voted on
    /// then the voter's weight won't count towards the vote outcome. If the
    /// Proposal is already in decided state then the instruction has no impact
    /// on the Proposal and only allows voters to prune their outstanding
    /// votes in case they wanted to withdraw Governing tokens from the Realm
    ///
    ///   0. `[]` Realm account
    ///   1. `[]` Governance account
    ///   2. `[writable]` Proposal account
    ///   3. `[writable]` TokenOwnerRecord account.
    ///     * PDA seeds: ['governance',realm, vote_governing_token_mint,
    ///       governing_token_owner]
    ///   4. `[writable]` Proposal VoteRecord account.
    ///     * PDA seeds: ['governance',proposal, token_owner_record]
    ///   5. `[]` The Governing Token Mint which was used to cast the vote
    ///      (vote_governing_token_mint)
    ///   6. `[signer]` Optional Governance Authority (Token Owner or Governance
    ///      Delegate) It's required only when Proposal is still being voted on
    ///   7. `[writable]` Optional Beneficiary account which would receive
    ///      lamports when VoteRecord Account is disposed It's required only
    ///      when Proposal is still being voted on
    RelinquishVote,

    /// Executes a Transaction in the Proposal
    /// Anybody can execute transaction once Proposal has been voted Yes and
    /// transaction_hold_up time has passed The actual transaction being
    /// executed will be signed by Governance PDA the Proposal belongs to
    /// For example to execute Program upgrade the ProgramGovernance PDA would
    /// be used as the signer
    ///
    ///   0. `[]` Governance account
    ///   1. `[writable]` Proposal account
    ///   2. `[writable]` ProposalTransaction account you wish to execute
    ///   3+ Any extra accounts that are part of the transaction, in order
    ExecuteTransaction,

    /// Legacy CreateMintGovernance instruction
    /// Exists for backwards-compatibility
    Legacy2,

    /// Legacy CreateTokenGovernance instruction
    /// Exists for backwards-compatibility
    Legacy3,

    /// Sets GovernanceConfig for a Governance
    ///
    ///   0. `[]` Realm account the Governance account belongs to
    ///   1. `[writable, signer]` The Governance account the config is for
    SetGovernanceConfig {
        #[allow(dead_code)]
        /// New governance config
        config: GovernanceConfig,
    },

    /// Legacy FlagTransactionError instruction
    /// Exists for backwards-compatibility
    Legacy5,

    /// Sets new Realm authority
    ///
    ///   0. `[writable]` Realm account
    ///   1. `[signer]` Current Realm authority
    ///   2. `[]` New realm authority. Must be one of the realm governances when
    ///      set
    SetRealmAuthority {
        #[allow(dead_code)]
        /// Set action ( SetUnchecked, SetChecked, Remove)
        action: SetRealmAuthorityAction,
    },

    /// Sets realm config
    ///   0. `[writable]` Realm account
    ///   1. `[signer]`  Realm authority
    ///   2. `[]` Council Token Mint - optional
    ///     Note: In the current version it's only possible to remove council
    ///     mint (set it to None).
    ///     After setting council to None it won't be possible to withdraw the
    ///     tokens from the Realm any longer.
    ///     If that's required then it must be done before executing this
    ///     instruction.
    ///   3. `[writable]` Council Token Holding account - optional unless
    ///     council is used.
    ///     * PDA seeds: ['governance',realm,council_mint] The account will be
    ///     created with the Realm PDA as its owner
    ///   4. `[]` System
    ///   5. `[writable]` RealmConfig account.
    ///     * PDA seeds: ['realm-config', realm]
    ///   6. `[]` Optional Community Voter Weight Addin Program Id
    ///   7. `[]` Optional Max Community Voter Weight Addin Program Id
    ///   8. `[]` Optional Council Voter Weight Addin Program Id
    ///   9. `[]` Optional Max Council Voter Weight Addin Program Id
    ///   10. `[signer]` Optional Payer. Required if RealmConfig doesn't exist
    ///       and needs to be created
    SetRealmConfig {
        #[allow(dead_code)]
        /// Realm config args
        config_args: RealmConfigArgs,
    },

    /// Creates TokenOwnerRecord with 0 deposit amount
    /// It's used to register TokenOwner when voter weight addin is used and the
    /// Governance program doesn't take deposits
    ///
    ///   0. `[]` Realm account
    ///   1. `[]` Governing Token Owner account
    ///   2. `[writable]` TokenOwnerRecord account.
    ///     * PDA seeds: ['governance',realm, governing_token_mint,
    ///       governing_token_owner]
    ///   3. `[]` Governing Token Mint
    ///   4. `[signer]` Payer
    ///   5. `[]` System
    CreateTokenOwnerRecord {},

    /// Updates ProgramMetadata account
    /// The instruction dumps information implied by the program's code into a
    /// persistent account
    ///
    ///  0. `[writable]` ProgramMetadata account.
    ///     * PDA seeds: ['metadata']
    ///  1. `[signer]` Payer
    ///  2. `[]` System
    UpdateProgramMetadata {},

    /// Creates native SOL treasury account for a Governance account
    /// The account has no data and can be used as a payer for instructions
    /// signed by Governance PDAs or as a native SOL treasury
    ///
    ///  0. `[]` Governance account the treasury account is for
    ///  1. `[writable]` NativeTreasury account.
    ///     * PDA seeds: ['native-treasury', governance]
    ///  2. `[signer]` Payer
    ///  3. `[]` System
    CreateNativeTreasury,

    /// Revokes (burns) membership governing tokens for the given
    /// TokenOwnerRecord and hence takes away governance power from the
    /// TokenOwner. Note: If there are active votes for the TokenOwner then
    /// the vote weights won't be updated automatically
    ///
    ///  0. `[]` Realm account
    ///  1. `[writable]` Governing Token Holding account.
    ///     * PDA seeds: ['governance',realm, governing_token_mint]
    ///  2. `[writable]` TokenOwnerRecord account.
    ///     * PDA seeds: ['governance',realm, governing_token_mint,
    ///       governing_token_owner]
    ///  3. `[writable]` GoverningTokenMint
    ///  4. `[signer]` Revoke authority which can be either of:
    ///                1) GoverningTokenMint mint_authority to forcefully revoke
    ///                   the membership tokens
    ///                2) GoverningTokenOwner who voluntarily revokes their own
    ///                   membership
    ///  5. `[]` RealmConfig account.
    ///     * PDA seeds: ['realm-config', realm]
    ///  6. `[]` SPL Token program
    RevokeGoverningTokens {
        /// The amount to revoke
        #[allow(dead_code)]
        amount: u64,
    },

    /// Refunds ProposalDeposit once the given proposal is no longer active
    /// (Draft, SigningOff, Voting) Once the condition is met the
    /// instruction is permissionless and returns the deposit amount to the
    /// deposit payer
    ///
    ///   0. `[]` Proposal account
    ///   1. `[writable]` ProposalDeposit account.
    ///     * PDA seeds: ['proposal-deposit', proposal, deposit payer]
    ///   2. `[writable]` Proposal deposit payer (beneficiary) account
    RefundProposalDeposit {},

    /// Transitions an off-chain or manually executable Proposal from Succeeded
    /// into Completed state
    ///
    /// Upon a successful vote on an off-chain or manually executable proposal
    /// it remains in Succeeded state Once the external actions are executed
    /// the Proposal owner can use the instruction to manually transition it to
    /// Completed state
    ///
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[]` TokenOwnerRecord account of the Proposal owner
    ///   2. `[signer]` CompleteProposal authority (Token Owner or Delegate)
    CompleteProposal {},

    /// Adds a required signatory to the Governance, which will be applied to
    /// all proposals created with it
    ///
    ///   0. `[writable, signer]` The Governance account the config is for
    ///   1. `[writable]` RequiredSignatory Account
    ///   2. `[signer]` Payer
    ///   3. `[]` System program
    AddRequiredSignatory {
        #[allow(dead_code)]
        /// Required signatory to add to the Governance
        signatory: Pubkey,
    },

    /// Removes a required signatory from the Governance
    ///
    ///  0. `[writable, signer]` The Governance account the config is for
    ///  1. `[writable]` RequiredSignatory Account
    ///  2. `[writable]` Beneficiary Account which would receive lamports from
    ///     the disposed RequiredSignatory Account
    RemoveRequiredSignatory,

    /// Sets TokenOwnerRecord lock for the given authority and lock id
    ///
    ///   0. `[]` Realm
    ///   1. `[]` RealmConfig
    ///   2. `[writable]` TokenOwnerRecord the lock is set for
    ///   3. `[signer]` Lock authority issuing the lock
    ///   4. `[signer]` Payer
    ///   5. `[]` System
    SetTokenOwnerRecordLock {
        /// Custom lock id which can be used by the authority to issue
        /// different locks
        #[allow(dead_code)]
        lock_id: u8,

        /// The timestamp when the lock expires or None if it never expires
        #[allow(dead_code)]
        expiry: Option<UnixTimestamp>,
    },

    /// Removes all expired TokenOwnerRecord locks and if specified
    /// the locks identified by the given lock ids and authority
    ///
    ///
    ///   0. `[]` Realm
    ///   1. `[]` RealmConfig
    ///   2. `[writable]` TokenOwnerRecord the locks are removed from
    ///   3. `[signer]` Optional lock authority which issued the locks specified
    ///      by lock_ids. If the authority is configured in RealmConfig then it
    ///      must sign the transaction. If the authority is no longer configured
    ///      then the locks are removed without the authority signature
    RelinquishTokenOwnerRecordLocks {
        /// Custom lock ids identifying the lock to remove
        /// If the lock_id is None then only expired locks are removed
        #[allow(dead_code)]
        lock_ids: Option<Vec<u8>>,
    },

    /// Sets Realm config item
    /// Note:
    /// This instruction is used to set a single RealmConfig item at a time
    /// In the current version it only supports TokenOwnerRecordLockAuthority
    /// however eventually all Realm configuration items should be set using
    /// this instruction and SetRealmConfig instruction should be deprecated
    ///
    ///   0. `[writable]` Realm account
    ///   1. `[writable]` RealmConfig account
    ///   2. `[signer]`  Realm authority
    ///   3. `[signer]` Payer
    ///   4. `[]` System
    SetRealmConfigItem {
        #[allow(dead_code)]
        /// Config args
        args: SetRealmConfigItemArgs,
    },
}

/// Creates CreateRealm instruction
#[allow(clippy::too_many_arguments)]
pub fn create_realm(
    program_id: &Pubkey,
    // Accounts
    realm_authority: &Pubkey,
    community_token_mint: &Pubkey,
    payer: &Pubkey,
    council_token_mint: Option<Pubkey>,
    // Accounts Args
    community_token_config_args: Option<GoverningTokenConfigAccountArgs>,
    council_token_config_args: Option<GoverningTokenConfigAccountArgs>,
    // Args
    name: String,
    min_community_weight_to_create_governance: u64,
    community_mint_max_voter_weight_source: MintMaxVoterWeightSource,
) -> Instruction {
    let realm_address = get_realm_address(program_id, &name);
    let community_token_holding_address =
        get_governing_token_holding_address(program_id, &realm_address, community_token_mint);

    let mut accounts = vec![
        AccountMeta::new(realm_address, false),
        AccountMeta::new_readonly(*realm_authority, false),
        AccountMeta::new_readonly(*community_token_mint, false),
        AccountMeta::new(community_token_holding_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    let use_council_mint = if let Some(council_token_mint) = council_token_mint {
        let council_token_holding_address =
            get_governing_token_holding_address(program_id, &realm_address, &council_token_mint);

        accounts.push(AccountMeta::new_readonly(council_token_mint, false));
        accounts.push(AccountMeta::new(council_token_holding_address, false));
        true
    } else {
        false
    };

    let realm_config_address = get_realm_config_address(program_id, &realm_address);
    accounts.push(AccountMeta::new(realm_config_address, false));

    let community_token_config_args =
        with_governing_token_config_args(&mut accounts, community_token_config_args);

    let council_token_config_args =
        with_governing_token_config_args(&mut accounts, council_token_config_args);

    let instruction = GovernanceInstruction::CreateRealm {
        config_args: RealmConfigArgs {
            use_council_mint,
            min_community_weight_to_create_governance,
            community_mint_max_voter_weight_source,
            community_token_config_args,
            council_token_config_args,
        },
        name,
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates DepositGoverningTokens instruction
#[allow(clippy::too_many_arguments)]
pub fn deposit_governing_tokens(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governing_token_source: &Pubkey,
    governing_token_owner: &Pubkey,
    governing_token_source_authority: &Pubkey,
    payer: &Pubkey,
    // Args
    amount: u64,
    governing_token_mint: &Pubkey,
) -> Instruction {
    let token_owner_record_address = get_token_owner_record_address(
        program_id,
        realm,
        governing_token_mint,
        governing_token_owner,
    );

    let governing_token_holding_address =
        get_governing_token_holding_address(program_id, realm, governing_token_mint);

    let realm_config_address = get_realm_config_address(program_id, realm);

    let accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(governing_token_holding_address, false),
        AccountMeta::new(*governing_token_source, false),
        AccountMeta::new_readonly(*governing_token_owner, true),
        AccountMeta::new_readonly(*governing_token_source_authority, true),
        AccountMeta::new(token_owner_record_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(realm_config_address, false),
    ];

    let instruction = GovernanceInstruction::DepositGoverningTokens { amount };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates WithdrawGoverningTokens instruction
pub fn withdraw_governing_tokens(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governing_token_destination: &Pubkey,
    governing_token_owner: &Pubkey,
    // Args
    governing_token_mint: &Pubkey,
) -> Instruction {
    let token_owner_record_address = get_token_owner_record_address(
        program_id,
        realm,
        governing_token_mint,
        governing_token_owner,
    );

    let governing_token_holding_address =
        get_governing_token_holding_address(program_id, realm, governing_token_mint);

    let realm_config_address = get_realm_config_address(program_id, realm);

    let accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(governing_token_holding_address, false),
        AccountMeta::new(*governing_token_destination, false),
        AccountMeta::new_readonly(*governing_token_owner, true),
        AccountMeta::new(token_owner_record_address, false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(realm_config_address, false),
    ];

    let instruction = GovernanceInstruction::WithdrawGoverningTokens {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates SetGovernanceDelegate instruction
pub fn set_governance_delegate(
    program_id: &Pubkey,
    // Accounts
    governance_authority: &Pubkey,
    // Args
    realm: &Pubkey,
    governing_token_mint: &Pubkey,
    governing_token_owner: &Pubkey,
    new_governance_delegate: &Option<Pubkey>,
) -> Instruction {
    let vote_record_address = get_token_owner_record_address(
        program_id,
        realm,
        governing_token_mint,
        governing_token_owner,
    );

    let accounts = vec![
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(vote_record_address, false),
    ];

    let instruction = GovernanceInstruction::SetGovernanceDelegate {
        new_governance_delegate: *new_governance_delegate,
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates CreateGovernance instruction using optional voter weight addin
#[allow(clippy::too_many_arguments)]
pub fn create_governance(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governance_seed: &Pubkey,
    token_owner_record: &Pubkey,
    payer: &Pubkey,
    create_authority: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    // Args
    config: GovernanceConfig,
) -> Instruction {
    let governance_address = get_governance_address(program_id, realm, governance_seed);

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(governance_address, false),
        AccountMeta::new_readonly(*governance_seed, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(*create_authority, true),
    ];

    with_realm_config_accounts(program_id, &mut accounts, realm, voter_weight_record, None);

    let instruction = GovernanceInstruction::CreateGovernance { config };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates CreateProposal instruction
#[allow(clippy::too_many_arguments)]
pub fn create_proposal(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    proposal_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    payer: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    // Args
    realm: &Pubkey,
    name: String,
    description_link: String,
    governing_token_mint: &Pubkey,
    vote_type: VoteType,
    options: Vec<String>,
    use_deny_option: bool,
    proposal_seed: &Pubkey,
) -> Instruction {
    let proposal_address =
        get_proposal_address(program_id, governance, governing_token_mint, proposal_seed);

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(proposal_address, false),
        AccountMeta::new(*governance, false),
        AccountMeta::new(*proposal_owner_record, false),
        AccountMeta::new_readonly(*governing_token_mint, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    with_realm_config_accounts(program_id, &mut accounts, realm, voter_weight_record, None);

    // Deposit is only required when there are more active proposal then the
    // configured exempt amount Note: We always pass the account because the
    // actual value is not known here without passing Governance account data
    let proposal_deposit_address =
        get_proposal_deposit_address(program_id, &proposal_address, payer);
    accounts.push(AccountMeta::new(proposal_deposit_address, false));

    let instruction = GovernanceInstruction::CreateProposal {
        name,
        description_link,
        vote_type,
        options,
        use_deny_option,
        proposal_seed: *proposal_seed,
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates AddSignatory instruction
pub fn add_signatory(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    proposal: &Pubkey,
    add_signatory_authority: &AddSignatoryAuthority,
    payer: &Pubkey,
    // Args
    signatory: &Pubkey,
) -> Instruction {
    let signatory_record_address = get_signatory_record_address(program_id, proposal, signatory);

    let mut accounts = vec![
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(signatory_record_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    match add_signatory_authority {
        AddSignatoryAuthority::ProposalOwner {
            governance_authority,
            token_owner_record,
        } => {
            accounts.push(AccountMeta::new_readonly(*token_owner_record, false));
            accounts.push(AccountMeta::new_readonly(*governance_authority, true));
        }
        AddSignatoryAuthority::None => {
            accounts.push(AccountMeta::new_readonly(
                get_required_signatory_address(program_id, governance, signatory),
                false,
            ));
        }
    };

    let instruction = GovernanceInstruction::AddSignatory {
        signatory: *signatory,
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

#[derive(Debug, Copy, Clone)]
/// Enum to specify the authority by which the instruction should add a
/// signatory
pub enum AddSignatoryAuthority {
    /// Proposal owners can add optional signatories to a proposal
    ProposalOwner {
        /// Token owner or its delegate
        governance_authority: Pubkey,
        /// Token owner record of the Proposal owner
        token_owner_record: Pubkey,
    },
    /// Anyone can add signatories that are required by the governance to a
    /// proposal
    None,
}

/// Creates SignOffProposal instruction
pub fn sign_off_proposal(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    signatory: &Pubkey,
    proposal_owner_record: Option<&Pubkey>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new_readonly(*signatory, true),
    ];

    if let Some(proposal_owner_record) = proposal_owner_record {
        accounts.push(AccountMeta::new_readonly(*proposal_owner_record, false))
    } else {
        let signatory_record_address =
            get_signatory_record_address(program_id, proposal, signatory);
        accounts.push(AccountMeta::new(signatory_record_address, false));
    }

    let instruction = GovernanceInstruction::SignOffProposal;

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates CastVote instruction
#[allow(clippy::too_many_arguments)]
pub fn cast_vote(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    proposal_owner_record: &Pubkey,
    voter_token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    vote_governing_token_mint: &Pubkey,
    payer: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    max_voter_weight_record: Option<Pubkey>,
    // Args
    vote: Vote,
) -> Instruction {
    let vote_record_address =
        get_vote_record_address(program_id, proposal, voter_token_owner_record);

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*proposal_owner_record, false),
        AccountMeta::new(*voter_token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(vote_record_address, false),
        AccountMeta::new_readonly(*vote_governing_token_mint, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    with_realm_config_accounts(
        program_id,
        &mut accounts,
        realm,
        voter_weight_record,
        max_voter_weight_record,
    );

    let instruction = GovernanceInstruction::CastVote { vote };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates FinalizeVote instruction
pub fn finalize_vote(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    proposal_owner_record: &Pubkey,
    governing_token_mint: &Pubkey,
    max_voter_weight_record: Option<Pubkey>,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*proposal_owner_record, false),
        AccountMeta::new_readonly(*governing_token_mint, false),
    ];

    with_realm_config_accounts(
        program_id,
        &mut accounts,
        realm,
        None,
        max_voter_weight_record,
    );

    let instruction = GovernanceInstruction::FinalizeVote {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates RelinquishVote instruction
#[allow(clippy::too_many_arguments)]
pub fn relinquish_vote(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    vote_governing_token_mint: &Pubkey,
    governance_authority: Option<Pubkey>,
    beneficiary: Option<Pubkey>,
) -> Instruction {
    let vote_record_address = get_vote_record_address(program_id, proposal, token_owner_record);

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*token_owner_record, false),
        AccountMeta::new(vote_record_address, false),
        AccountMeta::new_readonly(*vote_governing_token_mint, false),
    ];

    if let Some(governance_authority) = governance_authority {
        accounts.push(AccountMeta::new_readonly(governance_authority, true));
        accounts.push(AccountMeta::new(beneficiary.unwrap(), false));
    }

    let instruction = GovernanceInstruction::RelinquishVote {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates CancelProposal instruction
pub fn cancel_proposal(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governance: &Pubkey,
    proposal: &Pubkey,
    proposal_owner_record: &Pubkey,
    governance_authority: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*proposal_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
    ];

    let instruction = GovernanceInstruction::CancelProposal {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates InsertTransaction instruction
#[allow(clippy::too_many_arguments)]
pub fn insert_transaction(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    payer: &Pubkey,
    // Args
    option_index: u8,
    index: u16,
    instructions: Vec<InstructionData>,
) -> Instruction {
    let proposal_transaction_address = get_proposal_transaction_address(
        program_id,
        proposal,
        &option_index.to_le_bytes(),
        &index.to_le_bytes(),
    );

    let accounts = vec![
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(proposal_transaction_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    let instruction = GovernanceInstruction::InsertTransaction {
        option_index,
        index,
        legacy: 0,
        instructions,
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates RemoveTransaction instruction
pub fn remove_transaction(
    program_id: &Pubkey,
    // Accounts
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    proposal_transaction: &Pubkey,
    beneficiary: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(*proposal_transaction, false),
        AccountMeta::new(*beneficiary, false),
    ];

    let instruction = GovernanceInstruction::RemoveTransaction {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates ExecuteTransaction instruction
pub fn execute_transaction(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    proposal: &Pubkey,
    proposal_transaction: &Pubkey,
    instruction_program_id: &Pubkey,
    instruction_accounts: &[AccountMeta],
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*proposal_transaction, false),
        AccountMeta::new_readonly(*instruction_program_id, false),
    ];

    accounts.extend_from_slice(instruction_accounts);

    let instruction = GovernanceInstruction::ExecuteTransaction {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates SetGovernanceConfig instruction
pub fn set_governance_config(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    // Args
    config: GovernanceConfig,
) -> Instruction {
    let accounts = vec![AccountMeta::new(*governance, true)];

    let instruction = GovernanceInstruction::SetGovernanceConfig { config };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates SetRealmAuthority instruction
pub fn set_realm_authority(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    realm_authority: &Pubkey,
    new_realm_authority: Option<&Pubkey>,
    // Args
    action: SetRealmAuthorityAction,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(*realm, false),
        AccountMeta::new_readonly(*realm_authority, true),
    ];

    match action {
        SetRealmAuthorityAction::SetChecked | SetRealmAuthorityAction::SetUnchecked => {
            accounts.push(AccountMeta::new_readonly(
                *new_realm_authority.unwrap(),
                false,
            ));
        }
        SetRealmAuthorityAction::Remove => {}
    }

    let instruction = GovernanceInstruction::SetRealmAuthority { action };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates SetRealmConfig instruction
#[allow(clippy::too_many_arguments)]
pub fn set_realm_config(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    realm_authority: &Pubkey,
    council_token_mint: Option<Pubkey>,
    payer: &Pubkey,
    // Accounts  Args
    community_token_config_args: Option<GoverningTokenConfigAccountArgs>,
    council_token_config_args: Option<GoverningTokenConfigAccountArgs>,
    // Args
    min_community_weight_to_create_governance: u64,
    community_mint_max_voter_weight_source: MintMaxVoterWeightSource,
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(*realm, false),
        AccountMeta::new_readonly(*realm_authority, true),
    ];

    let use_council_mint = if let Some(council_token_mint) = council_token_mint {
        let council_token_holding_address =
            get_governing_token_holding_address(program_id, realm, &council_token_mint);

        accounts.push(AccountMeta::new_readonly(council_token_mint, false));
        accounts.push(AccountMeta::new(council_token_holding_address, false));
        true
    } else {
        false
    };

    accounts.push(AccountMeta::new_readonly(system_program::id(), false));

    // Always pass realm_config_address because it's needed when
    // use_community_voter_weight_addin is set to true but also when it's set to
    // false and the addin is being  removed from the realm
    let realm_config_address = get_realm_config_address(program_id, realm);
    accounts.push(AccountMeta::new(realm_config_address, false));

    let community_token_config_args =
        with_governing_token_config_args(&mut accounts, community_token_config_args);

    let council_token_config_args =
        with_governing_token_config_args(&mut accounts, council_token_config_args);

    accounts.push(AccountMeta::new(*payer, true));

    let instruction = GovernanceInstruction::SetRealmConfig {
        config_args: RealmConfigArgs {
            use_council_mint,
            min_community_weight_to_create_governance,
            community_mint_max_voter_weight_source,
            community_token_config_args,
            council_token_config_args,
        },
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Adds realm config account and accounts referenced by the config
/// 1) VoterWeightRecord
/// 2) MaxVoterWeightRecord
pub fn with_realm_config_accounts(
    program_id: &Pubkey,
    accounts: &mut Vec<AccountMeta>,
    realm: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    max_voter_weight_record: Option<Pubkey>,
) {
    let realm_config_address = get_realm_config_address(program_id, realm);
    accounts.push(AccountMeta::new_readonly(realm_config_address, false));

    if let Some(voter_weight_record) = voter_weight_record {
        accounts.push(AccountMeta::new_readonly(voter_weight_record, false));
        true
    } else {
        false
    };

    if let Some(max_voter_weight_record) = max_voter_weight_record {
        accounts.push(AccountMeta::new_readonly(max_voter_weight_record, false));
        true
    } else {
        false
    };
}

/// Creates CreateTokenOwnerRecord instruction
pub fn create_token_owner_record(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governing_token_owner: &Pubkey,
    governing_token_mint: &Pubkey,
    payer: &Pubkey,
) -> Instruction {
    let token_owner_record_address = get_token_owner_record_address(
        program_id,
        realm,
        governing_token_mint,
        governing_token_owner,
    );

    let accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(*governing_token_owner, false),
        AccountMeta::new(token_owner_record_address, false),
        AccountMeta::new_readonly(*governing_token_mint, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let instruction = GovernanceInstruction::CreateTokenOwnerRecord {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates UpdateProgramMetadata instruction
pub fn upgrade_program_metadata(
    program_id: &Pubkey,
    // Accounts
    payer: &Pubkey,
) -> Instruction {
    let program_metadata_address = get_program_metadata_address(program_id);

    let accounts = vec![
        AccountMeta::new(program_metadata_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let instruction = GovernanceInstruction::UpdateProgramMetadata {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates CreateNativeTreasury instruction
pub fn create_native_treasury(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    payer: &Pubkey,
) -> Instruction {
    let native_treasury_address = get_native_treasury_address(program_id, governance);

    let accounts = vec![
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(native_treasury_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let instruction = GovernanceInstruction::CreateNativeTreasury {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates RevokeGoverningTokens instruction
#[allow(clippy::too_many_arguments)]
pub fn revoke_governing_tokens(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governing_token_owner: &Pubkey,
    governing_token_mint: &Pubkey,
    revoke_authority: &Pubkey,
    // Args
    amount: u64,
) -> Instruction {
    let token_owner_record_address = get_token_owner_record_address(
        program_id,
        realm,
        governing_token_mint,
        governing_token_owner,
    );

    let governing_token_holding_address =
        get_governing_token_holding_address(program_id, realm, governing_token_mint);

    let realm_config_address = get_realm_config_address(program_id, realm);

    let accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(governing_token_holding_address, false),
        AccountMeta::new(token_owner_record_address, false),
        AccountMeta::new(*governing_token_mint, false),
        AccountMeta::new_readonly(*revoke_authority, true),
        AccountMeta::new_readonly(realm_config_address, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    let instruction = GovernanceInstruction::RevokeGoverningTokens { amount };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates AddRequiredSignatory instruction
pub fn add_required_signatory(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    payer: &Pubkey,
    // Args
    signatory: &Pubkey,
) -> Instruction {
    let required_signatory_address =
        get_required_signatory_address(program_id, governance, signatory);

    let accounts = vec![
        AccountMeta::new(*governance, true),
        AccountMeta::new(required_signatory_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let instruction = GovernanceInstruction::AddRequiredSignatory {
        signatory: *signatory,
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates RemoveRequiredSignatory instruction
pub fn remove_required_signatory(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    signatory: &Pubkey,
    beneficiary: &Pubkey,
) -> Instruction {
    let required_signatory_address =
        get_required_signatory_address(program_id, governance, signatory);

    let accounts = vec![
        AccountMeta::new(*governance, true),
        AccountMeta::new(required_signatory_address, false),
        AccountMeta::new(*beneficiary, false),
    ];

    let instruction = GovernanceInstruction::RemoveRequiredSignatory;

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Adds accounts specified by GoverningTokenConfigAccountArgs
/// and returns GoverningTokenConfigArgs
pub fn with_governing_token_config_args(
    accounts: &mut Vec<AccountMeta>,
    governing_token_config_args: Option<GoverningTokenConfigAccountArgs>,
) -> GoverningTokenConfigArgs {
    let governing_token_config_args = governing_token_config_args.unwrap_or_default();

    let use_voter_weight_addin =
        if let Some(voter_weight_addin) = governing_token_config_args.voter_weight_addin {
            accounts.push(AccountMeta::new_readonly(voter_weight_addin, false));
            true
        } else {
            false
        };

    let use_max_voter_weight_addin =
        if let Some(max_voter_weight_addin) = governing_token_config_args.max_voter_weight_addin {
            accounts.push(AccountMeta::new_readonly(max_voter_weight_addin, false));
            true
        } else {
            false
        };

    GoverningTokenConfigArgs {
        use_voter_weight_addin,
        use_max_voter_weight_addin,
        token_type: governing_token_config_args.token_type,
    }
}

/// Creates RefundProposalDeposit instruction
#[allow(clippy::too_many_arguments)]
pub fn refund_proposal_deposit(
    program_id: &Pubkey,
    // Accounts
    proposal: &Pubkey,
    proposal_deposit_payer: &Pubkey,
    // Args
) -> Instruction {
    let proposal_deposit_address =
        get_proposal_deposit_address(program_id, proposal, proposal_deposit_payer);

    let accounts = vec![
        AccountMeta::new_readonly(*proposal, false),
        AccountMeta::new(proposal_deposit_address, false),
        AccountMeta::new(*proposal_deposit_payer, false),
    ];

    let instruction = GovernanceInstruction::RefundProposalDeposit {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates CompleteProposal instruction to move proposal from Succeeded to
/// Completed
pub fn complete_proposal(
    program_id: &Pubkey,
    // Accounts
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    complete_proposal_authority: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*complete_proposal_authority, true),
    ];

    let instruction = GovernanceInstruction::CompleteProposal {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates SetTokenOwnerRecordLock instruction to issue TokenOwnerRecord lock
pub fn set_token_owner_record_lock(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    token_owner_record: &Pubkey,
    token_owner_record_lock_authority: &Pubkey,
    payer: &Pubkey,
    // Args
    lock_id: u8,
    expiry: Option<UnixTimestamp>,
) -> Instruction {
    let realm_config_address = get_realm_config_address(program_id, realm);

    let accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(realm_config_address, false),
        AccountMeta::new(*token_owner_record, false),
        AccountMeta::new_readonly(*token_owner_record_lock_authority, true),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let instruction = GovernanceInstruction::SetTokenOwnerRecordLock { lock_id, expiry };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates RelinquishTokenOwnerRecordLocks instruction to remove
/// TokenOwnerRecord locks
pub fn relinquish_token_owner_record_locks(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    token_owner_record: &Pubkey,
    token_owner_record_lock_authority: Option<Pubkey>,
    // Args
    lock_ids: Option<Vec<u8>>,
) -> Instruction {
    let realm_config_address = get_realm_config_address(program_id, realm);

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(realm_config_address, false),
        AccountMeta::new(*token_owner_record, false),
    ];

    if let Some(token_owner_record_lock_authority) = token_owner_record_lock_authority {
        accounts.push(AccountMeta::new_readonly(
            token_owner_record_lock_authority,
            true,
        ));
    }

    let instruction = GovernanceInstruction::RelinquishTokenOwnerRecordLocks { lock_ids };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}

/// Creates SetRealmConfigItem instruction to set realm config
pub fn set_realm_config_item(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    realm_authority: &Pubkey,
    payer: &Pubkey,
    // Args
    args: SetRealmConfigItemArgs,
) -> Instruction {
    let realm_config_address = get_realm_config_address(program_id, realm);

    let accounts = vec![
        AccountMeta::new(*realm, false),
        AccountMeta::new(realm_config_address, false),
        AccountMeta::new_readonly(*realm_authority, true),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
    ];

    let instruction = GovernanceInstruction::SetRealmConfigItem { args };

    Instruction {
        program_id: *program_id,
        accounts,
        data: borsh::to_vec(&instruction).unwrap(),
    }
}
