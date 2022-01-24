//! Program instructions

use crate::{
    state::{
        enums::MintMaxVoteWeightSource,
        governance::{
            get_account_governance_address, get_mint_governance_address,
            get_program_governance_address, get_token_governance_address, GovernanceConfig,
        },
        native_treasury::get_native_treasury_address,
        program_metadata::get_program_metadata_address,
        proposal::{get_proposal_address, VoteType},
        proposal_instruction::{get_proposal_instruction_address, InstructionData},
        realm::{get_governing_token_holding_address, get_realm_address, RealmConfigArgs},
        realm_config::get_realm_config_address,
        signatory_record::get_signatory_record_address,
        token_owner_record::get_token_owner_record_address,
        vote_record::{get_vote_record_address, Vote},
    },
    tools::bpf_loader_upgradeable::get_program_data_address,
};
use borsh::{BorshDeserialize, BorshSchema, BorshSerialize};
use solana_program::{
    bpf_loader_upgradeable,
    instruction::{AccountMeta, Instruction},
    pubkey::Pubkey,
    system_program, sysvar,
};

/// Instructions supported by the Governance program
#[derive(Clone, Debug, PartialEq, BorshDeserialize, BorshSerialize, BorshSchema)]
#[repr(C)]
#[allow(clippy::large_enum_variant)]
pub enum GovernanceInstruction {
    /// Creates Governance Realm account which aggregates governances for given Community Mint and optional Council Mint
    ///
    /// 0. `[writable]` Governance Realm account. PDA seeds:['governance',name]
    /// 1. `[]` Realm authority
    /// 2. `[]` Community Token Mint
    /// 3. `[writable]` Community Token Holding account. PDA seeds: ['governance',realm,community_mint]
    ///     The account will be created with the Realm PDA as its owner
    /// 4. `[signer]` Payer
    /// 5. `[]` System
    /// 6. `[]` SPL Token
    /// 7. `[]` Sysvar Rent

    /// 8. `[]` Council Token Mint - optional
    /// 9. `[writable]` Council Token Holding account - optional unless council is used. PDA seeds: ['governance',realm,council_mint]
    ///     The account will be created with the Realm PDA as its owner

    /// 10. `[writable]` RealmConfig account. PDA seeds: ['realm-config', realm]
    /// 11. `[]` Optional Community Voter Weight Addin Program Id
    CreateRealm {
        #[allow(dead_code)]
        /// UTF-8 encoded Governance Realm name
        name: String,

        #[allow(dead_code)]
        /// Realm config args     
        config_args: RealmConfigArgs,
    },

    /// Deposits governing tokens (Community or Council) to Governance Realm and establishes your voter weight to be used for voting within the Realm
    /// Note: If subsequent (top up) deposit is made and there are active votes for the Voter then the vote weights won't be updated automatically
    /// It can be done by relinquishing votes on active Proposals and voting again with the new weight
    ///
    ///  0. `[]` Governance Realm account
    ///  1. `[writable]` Governing Token Holding account. PDA seeds: ['governance',realm, governing_token_mint]
    ///  2. `[writable]` Governing Token Source account. All tokens from the account will be transferred to the Holding account
    ///  3. `[signer]` Governing Token Owner account
    ///  4. `[signer]` Governing Token Transfer authority
    ///  5. `[writable]` Token Owner Record account. PDA seeds: ['governance',realm, governing_token_mint, governing_token_owner]
    ///  6. `[signer]` Payer
    ///  7. `[]` System
    ///  8. `[]` SPL Token
    ///  9. `[]` Sysvar Rent
    DepositGoverningTokens {
        /// The amount to deposit into the realm
        #[allow(dead_code)]
        amount: u64,
    },

    /// Withdraws governing tokens (Community or Council) from Governance Realm and downgrades your voter weight within the Realm
    /// Note: It's only possible to withdraw tokens if the Voter doesn't have any outstanding active votes
    /// If there are any outstanding votes then they must be relinquished before tokens could be withdrawn
    ///
    ///  0. `[]` Governance Realm account
    ///  1. `[writable]` Governing Token Holding account. PDA seeds: ['governance',realm, governing_token_mint]
    ///  2. `[writable]` Governing Token Destination account. All tokens will be transferred to this account
    ///  3. `[signer]` Governing Token Owner account
    ///  4. `[writable]` Token Owner  Record account. PDA seeds: ['governance',realm, governing_token_mint, governing_token_owner]
    ///  5. `[]` SPL Token
    WithdrawGoverningTokens {},

    /// Sets Governance Delegate for the given Realm and Governing Token Mint (Community or Council)
    /// The Delegate would have voting rights and could vote on behalf of the Governing Token Owner
    /// The Delegate would also be able to create Proposals on behalf of the Governing Token Owner
    /// Note: This doesn't take voting rights from the Token Owner who still can vote and change governance_delegate
    ///
    /// 0. `[signer]` Current Governance Delegate or Governing Token owner
    /// 1. `[writable]` Token Owner  Record
    SetGovernanceDelegate {
        #[allow(dead_code)]
        /// New Governance Delegate
        new_governance_delegate: Option<Pubkey>,
    },

    /// Creates Account Governance account which can be used to govern an arbitrary account
    ///
    ///   0. `[]` Realm account the created Governance belongs to
    ///   1. `[writable]` Account Governance account. PDA seeds: ['account-governance', realm, governed_account]
    ///   2. `[]` Account governed by this Governance
    ///   3. `[]` Governing TokenOwnerRecord account
    ///   4. `[signer]` Payer
    ///   5. `[]` System program
    ///   6. `[]` Sysvar Rent
    ///   7. `[signer]` Governance authority
    ///   8. `[]` Optional Realm Config
    ///   9. `[]` Optional Voter Weight Record
    CreateAccountGovernance {
        /// Governance config
        #[allow(dead_code)]
        config: GovernanceConfig,
    },

    /// Creates Program Governance account which governs an upgradable program
    ///
    ///   0. `[]` Realm account the created Governance belongs to
    ///   1. `[writable]` Program Governance account. PDA seeds: ['program-governance', realm, governed_program]
    ///   2. `[]` Program governed by this Governance account
    ///   3. `[writable]` Program Data account of the Program governed by this Governance account
    ///   4. `[signer]` Current Upgrade Authority account of the Program governed by this Governance account
    ///   5. `[]` Governing TokenOwnerRecord account     
    ///   6. `[signer]` Payer
    ///   7. `[]` bpf_upgradeable_loader program
    ///   8. `[]` System program
    ///   9. `[]` Sysvar Rent
    ///   10. `[signer]` Governance authority
    ///   11. `[]` Optional Realm Config
    ///   12. `[]` Optional Voter Weight Record
    CreateProgramGovernance {
        /// Governance config
        #[allow(dead_code)]
        config: GovernanceConfig,

        #[allow(dead_code)]
        /// Indicates whether Program's upgrade_authority should be transferred to the Governance PDA
        /// If it's set to false then it can be done at a later time
        /// However the instruction would validate the current upgrade_authority signed the transaction nonetheless
        transfer_upgrade_authority: bool,
    },

    /// Creates Proposal account for Instructions that will be executed at some point in the future
    ///
    ///   0. `[]` Realm account the created Proposal belongs to
    ///   1. `[writable]` Proposal account. PDA seeds ['governance',governance, governing_token_mint, proposal_index]
    ///   2. `[writable]` Governance account
    ///   3. `[writable]` TokenOwnerRecord account of the Proposal owner
    ///   4. `[]` Governing Token Mint the Proposal is created for
    ///   5. `[signer]` Governance Authority (Token Owner or Governance Delegate)
    ///   6. `[signer]` Payer
    ///   7. `[]` System program
    ///   8. `[]` Rent sysvar
    ///   9. `[]` Clock sysvar
    ///   10. `[]` Optional Realm Config
    ///   11. `[]` Optional Voter Weight Record
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
        /// Only proposals with the rejecting option can have executable instructions
        use_deny_option: bool,
    },

    /// Adds a signatory to the Proposal which means this Proposal can't leave Draft state until yet another Signatory signs
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[]` TokenOwnerRecord account of the Proposal owner
    ///   2. `[signer]` Governance Authority (Token Owner or Governance Delegate)
    ///   3. `[writable]` Signatory Record Account
    ///   4. `[signer]` Payer
    ///   5. `[]` System program
    ///   6. `[]` Rent sysvar
    AddSignatory {
        #[allow(dead_code)]
        /// Signatory to add to the Proposal
        signatory: Pubkey,
    },

    /// Removes a Signatory from the Proposal
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[]` TokenOwnerRecord account of the Proposal owner
    ///   2. `[signer]` Governance Authority (Token Owner or Governance Delegate)
    ///   3. `[writable]` Signatory Record Account
    ///   4. `[writable]` Beneficiary Account which would receive lamports from the disposed Signatory Record Account
    RemoveSignatory {
        #[allow(dead_code)]
        /// Signatory to remove from the Proposal
        signatory: Pubkey,
    },

    /// Inserts an instruction for the Proposal at the given index position
    /// New Instructions must be inserted at the end of the range indicated by Proposal instructions_next_index
    /// If an Instruction replaces an existing Instruction at a given index then the old one must be removed using RemoveInstruction first

    ///   0. `[]` Governance account
    ///   1. `[writable]` Proposal account
    ///   2. `[]` TokenOwnerRecord account of the Proposal owner
    ///   3. `[signer]` Governance Authority (Token Owner or Governance Delegate)
    ///   4. `[writable]` ProposalInstruction account. PDA seeds: ['governance',proposal,index]
    ///   5. `[signer]` Payer
    ///   6. `[]` System program
    ///   7. `[]` Rent sysvar
    InsertInstruction {
        #[allow(dead_code)]
        /// The index of the option the instruction is for
        option_index: u16,
        #[allow(dead_code)]
        /// Instruction index to be inserted at.
        index: u16,
        #[allow(dead_code)]
        /// Waiting time (in seconds) between vote period ending and this being eligible for execution
        hold_up_time: u32,

        #[allow(dead_code)]
        /// Instruction Data
        instruction: InstructionData,
    },

    /// Removes instruction from the Proposal
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[]` TokenOwnerRecord account of the Proposal owner
    ///   2. `[signer]` Governance Authority (Token Owner or Governance Delegate)
    ///   3. `[writable]` ProposalInstruction account
    ///   4. `[writable]` Beneficiary Account which would receive lamports from the disposed ProposalInstruction account
    RemoveInstruction,

    /// Cancels Proposal by changing its state to Canceled
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[writable]`  TokenOwnerRecord account of the  Proposal owner
    ///   2. `[signer]` Governance Authority (Token Owner or Governance Delegate)
    ///   3. `[]` Clock sysvar
    ///   4. `[]` Governance account
    CancelProposal,

    /// Signs off Proposal indicating the Signatory approves the Proposal
    /// When the last Signatory signs the Proposal state moves to Voting state
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[writable]` Signatory Record account
    ///   2. `[signer]` Signatory account
    ///   3. `[]` Clock sysvar
    SignOffProposal,

    ///  Uses your voter weight (deposited Community or Council tokens) to cast a vote on a Proposal
    ///  By doing so you indicate you approve or disapprove of running the Proposal set of instructions
    ///  If you tip the consensus then the instructions can begin to be run after their hold up time
    ///
    ///   0. `[]` Realm account
    ///   1. `[]` Governance account
    ///   2. `[writable]` Proposal account
    ///   4. `[writable]` TokenOwnerRecord of the Proposal owner    
    ///   3. `[writable]` TokenOwnerRecord of the voter. PDA seeds: ['governance',realm, governing_token_mint, governing_token_owner]
    ///   4. `[signer]` Governance Authority (Token Owner or Governance Delegate)
    ///   5. `[writable]` Proposal VoteRecord account. PDA seeds: ['governance',proposal,governing_token_owner_record]
    ///   6. `[]` Governing Token Mint
    ///   7. `[signer]` Payer
    ///   8. `[]` System program
    ///   9. `[]` Rent sysvar
    ///   10. `[]` Clock sysvar
    ///   11. `[]` Optional Realm Config
    ///   12. `[]` Optional Voter Weight Record
    CastVote {
        #[allow(dead_code)]
        /// User's vote
        vote: Vote,
    },

    /// Finalizes vote in case the Vote was not automatically tipped within max_voting_time period
    ///
    ///   0. `[]` Realm account    
    ///   1. `[]` Governance account
    ///   2. `[writable]` Proposal account
    ///   3. `[writable]` TokenOwnerRecord of the Proposal owner        
    ///   4. `[]` Governing Token Mint
    ///   5. `[]` Clock sysvar
    FinalizeVote {},

    ///  Relinquish Vote removes voter weight from a Proposal and removes it from voter's active votes
    ///  If the Proposal is still being voted on then the voter's weight won't count towards the vote outcome
    ///  If the Proposal is already in decided state then the instruction has no impact on the Proposal
    ///  and only allows voters to prune their outstanding votes in case they wanted to withdraw Governing tokens from the Realm
    ///
    ///   0. `[]` Governance account
    ///   1. `[writable]` Proposal account
    ///   2. `[writable]` TokenOwnerRecord account. PDA seeds: ['governance',realm, governing_token_mint, governing_token_owner]
    ///   3. `[writable]` Proposal VoteRecord account. PDA seeds: ['governance',proposal,governing_token_owner_record]
    ///   4. `[]` Governing Token Mint
    ///   5. `[signer]` Optional Governance Authority (Token Owner or Governance Delegate)
    ///       It's required only when Proposal is still being voted on
    ///   6. `[writable]` Optional Beneficiary account which would receive lamports when VoteRecord Account is disposed
    ///       It's required only when Proposal is still being voted on
    RelinquishVote,

    /// Executes an instruction in the Proposal
    /// Anybody can execute transaction once Proposal has been voted Yes and transaction_hold_up time has passed
    /// The actual instruction being executed will be signed by Governance PDA the Proposal belongs to
    /// For example to execute Program upgrade the ProgramGovernance PDA would be used as the singer
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[writable]` ProposalInstruction account you wish to execute
    ///   2. `[]` Clock sysvar
    ///   3+ Any extra accounts that are part of the instruction, in order
    ExecuteInstruction,

    /// Creates Mint Governance account which governs a mint
    ///
    ///   0. `[]` Realm account the created Governance belongs to    
    ///   1. `[writable]` Mint Governance account. PDA seeds: ['mint-governance', realm, governed_mint]
    ///   2. `[writable]` Mint governed by this Governance account
    ///   3. `[signer]` Current Mint Authority
    ///   4. `[]` Governing TokenOwnerRecord account    
    ///   5. `[signer]` Payer
    ///   6. `[]` SPL Token program
    ///   7. `[]` System program
    ///   8. `[]` Sysvar Rent
    ///   8. `[signer]` Governance authority
    ///   9. `[]` Optional Realm Config
    ///   10. `[]` Optional Voter Weight Record
    CreateMintGovernance {
        #[allow(dead_code)]
        /// Governance config
        config: GovernanceConfig,

        #[allow(dead_code)]
        /// Indicates whether Mint's authority should be transferred to the Governance PDA
        /// If it's set to false then it can be done at a later time
        /// However the instruction would validate the current mint authority signed the transaction nonetheless
        transfer_mint_authority: bool,
    },

    /// Creates Token Governance account which governs a token account
    ///
    ///   0. `[]` Realm account the created Governance belongs to    
    ///   1. `[writable]` Token Governance account. PDA seeds: ['token-governance', realm, governed_token]
    ///   2. `[writable]` Token account governed by this Governance account
    ///   3. `[signer]` Current Token account
    ///   4. `[]` Governing TokenOwnerRecord account        
    ///   5. `[signer]` Payer
    ///   6. `[]` SPL Token program
    ///   7. `[]` System program
    ///   8. `[]` Sysvar Rent
    ///   9. `[signer]` Governance authority
    ///   10. `[]` Optional Realm Config
    ///   11. `[]` Optional Voter Weight Record   
    CreateTokenGovernance {
        #[allow(dead_code)]
        /// Governance config
        config: GovernanceConfig,

        #[allow(dead_code)]
        /// Indicates whether token owner should be transferred to the Governance PDA
        /// If it's set to false then it can be done at a later time
        /// However the instruction would validate the current token owner signed the transaction nonetheless
        transfer_token_owner: bool,
    },

    /// Sets GovernanceConfig for a Governance
    ///
    ///   0. `[]` Realm account the Governance account belongs to    
    ///   1. `[writable, signer]` The Governance account the config is for
    SetGovernanceConfig {
        #[allow(dead_code)]
        /// New governance config
        config: GovernanceConfig,
    },

    /// Flags an instruction and its parent Proposal with error status
    /// It can be used by Proposal owner in case the instruction is permanently broken and can't be executed
    /// Note: This instruction is a workaround because currently it's not possible to catch errors from CPI calls
    ///       and the Governance program has no way to know when instruction failed and flag it automatically
    ///
    ///   0. `[writable]` Proposal account
    ///   1. `[]` TokenOwnerRecord account of the Proposal owner
    ///   2. `[signer]` Governance Authority (Token Owner or Governance Delegate)    
    ///   3. `[writable]` ProposalInstruction account to flag
    ///   4. `[]` Clock sysvar
    FlagInstructionError,

    /// Sets new Realm authority
    ///
    ///   0. `[writable]` Realm account
    ///   1. `[signer]` Current Realm authority    
    ///   2. `[]` New realm authority. Must be one of the realm governances when set
    SetRealmAuthority {
        #[allow(dead_code)]
        /// Indicates whether the realm authority should be removed and set to None
        remove_authority: bool,
    },

    /// Sets realm config
    ///   0. `[writable]` Realm account
    ///   1. `[signer]`  Realm authority    
    ///   2. `[]` Council Token Mint - optional
    ///       Note: In the current version it's only possible to remove council mint (set it to None)
    ///       After setting council to None it won't be possible to withdraw the tokens from the Realm any longer
    ///       If that's required then it must be done before executing this instruction
    ///   3. `[writable]` Council Token Holding account - optional unless council is used. PDA seeds: ['governance',realm,council_mint]
    ///       The account will be created with the Realm PDA as its owner
    ///   4. `[]` System
    ///   5. `[writable]` RealmConfig account. PDA seeds: ['realm-config', realm]
    ///   6. `[signer]` Optional Payer
    ///   7. `[]` Optional Community Voter Weight Addin Program Id    
    SetRealmConfig {
        #[allow(dead_code)]
        /// Realm config args
        config_args: RealmConfigArgs,
    },

    /// Creates TokenOwnerRecord with 0 deposit amount
    /// It's used to register TokenOwner when voter weight addin is used and the Governance program doesn't take deposits
    ///
    ///   0. `[]` Realm account
    ///   1. `[]` Governing Token Owner account
    ///   2. `[writable]` TokenOwnerRecord account. PDA seeds: ['governance',realm, governing_token_mint, governing_token_owner]
    ///   3. `[]` Governing Token Mint   
    ///   4. `[signer]` Payer
    ///   5. `[]` System
    CreateTokenOwnerRecord {},

    /// Updates ProgramMetadata account
    /// The instruction dumps information implied by the program's code into a persistent account
    ///
    ///  0. `[writable]` ProgramMetadata account. PDA seeds: ['metadata']
    ///  1. `[signer]` Payer
    ///  2. `[]` System
    UpdateProgramMetadata {},

    /// Creates native SOL treasury account for a Governance account
    /// The account has no data and can be used as a payer for instructions signed by Governance PDAs or as a native SOL treasury
    ///
    ///  0. `[]` Governance account the treasury account is for
    ///  1. `[writable]` NativeTreasury account. PDA seeds: ['treasury', governance]
    ///  2. `[signer]` Payer
    ///  3. `[]` System
    CreateNativeTreasury,
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
    community_voter_weight_addin: Option<Pubkey>,
    // Args
    name: String,
    min_community_tokens_to_create_governance: u64,
    community_mint_max_vote_weight_source: MintMaxVoteWeightSource,
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

    let use_community_voter_weight_addin =
        if let Some(community_voter_weight_addin) = community_voter_weight_addin {
            accounts.push(AccountMeta::new_readonly(
                community_voter_weight_addin,
                false,
            ));
            true
        } else {
            false
        };

    let instruction = GovernanceInstruction::CreateRealm {
        config_args: RealmConfigArgs {
            use_council_mint,
            min_community_tokens_to_create_governance,
            community_mint_max_vote_weight_source,
            use_community_voter_weight_addin,
        },
        name,
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
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
    governing_token_transfer_authority: &Pubkey,
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

    let accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(governing_token_holding_address, false),
        AccountMeta::new(*governing_token_source, false),
        AccountMeta::new_readonly(*governing_token_owner, true),
        AccountMeta::new_readonly(*governing_token_transfer_authority, true),
        AccountMeta::new(token_owner_record_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    let instruction = GovernanceInstruction::DepositGoverningTokens { amount };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
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

    let accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(governing_token_holding_address, false),
        AccountMeta::new(*governing_token_destination, false),
        AccountMeta::new_readonly(*governing_token_owner, true),
        AccountMeta::new(token_owner_record_address, false),
        AccountMeta::new_readonly(spl_token::id(), false),
    ];

    let instruction = GovernanceInstruction::WithdrawGoverningTokens {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
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
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates CreateAccountGovernance instruction using optional voter weight addin
#[allow(clippy::too_many_arguments)]
pub fn create_account_governance(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governed_account: &Pubkey,
    token_owner_record: &Pubkey,
    payer: &Pubkey,
    governance_authority: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    // Args
    config: GovernanceConfig,
) -> Instruction {
    let account_governance_address =
        get_account_governance_address(program_id, realm, governed_account);

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(account_governance_address, false),
        AccountMeta::new_readonly(*governed_account, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(*governance_authority, true),
    ];

    with_voter_weight_accounts(program_id, &mut accounts, realm, voter_weight_record);

    let instruction = GovernanceInstruction::CreateAccountGovernance { config };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates CreateProgramGovernance instruction
#[allow(clippy::too_many_arguments)]
pub fn create_program_governance(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governed_program: &Pubkey,
    governed_program_upgrade_authority: &Pubkey,
    token_owner_record: &Pubkey,
    payer: &Pubkey,
    governance_authority: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    // Args
    config: GovernanceConfig,
    transfer_upgrade_authority: bool,
) -> Instruction {
    let program_governance_address =
        get_program_governance_address(program_id, realm, governed_program);
    let governed_program_data_address = get_program_data_address(governed_program);

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(program_governance_address, false),
        AccountMeta::new_readonly(*governed_program, false),
        AccountMeta::new(governed_program_data_address, false),
        AccountMeta::new_readonly(*governed_program_upgrade_authority, true),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(bpf_loader_upgradeable::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(*governance_authority, true),
    ];

    with_voter_weight_accounts(program_id, &mut accounts, realm, voter_weight_record);

    let instruction = GovernanceInstruction::CreateProgramGovernance {
        config,
        transfer_upgrade_authority,
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates CreateMintGovernance
#[allow(clippy::too_many_arguments)]
pub fn create_mint_governance(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governed_mint: &Pubkey,
    governed_mint_authority: &Pubkey,
    token_owner_record: &Pubkey,
    payer: &Pubkey,
    governance_authority: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    // Args
    config: GovernanceConfig,
    transfer_mint_authority: bool,
) -> Instruction {
    let mint_governance_address = get_mint_governance_address(program_id, realm, governed_mint);

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(mint_governance_address, false),
        AccountMeta::new(*governed_mint, false),
        AccountMeta::new_readonly(*governed_mint_authority, true),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(*governance_authority, true),
    ];

    with_voter_weight_accounts(program_id, &mut accounts, realm, voter_weight_record);

    let instruction = GovernanceInstruction::CreateMintGovernance {
        config,
        transfer_mint_authority,
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates CreateTokenGovernance instruction
#[allow(clippy::too_many_arguments)]
pub fn create_token_governance(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    governed_token: &Pubkey,
    governed_token_owner: &Pubkey,
    token_owner_record: &Pubkey,
    payer: &Pubkey,
    governance_authority: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    // Args
    config: GovernanceConfig,
    transfer_token_owner: bool,
) -> Instruction {
    let token_governance_address = get_token_governance_address(program_id, realm, governed_token);

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(token_governance_address, false),
        AccountMeta::new(*governed_token, false),
        AccountMeta::new_readonly(*governed_token_owner, true),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(spl_token::id(), false),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(*governance_authority, true),
    ];

    with_voter_weight_accounts(program_id, &mut accounts, realm, voter_weight_record);

    let instruction = GovernanceInstruction::CreateTokenGovernance {
        config,
        transfer_token_owner,
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
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
    proposal_index: u32,
) -> Instruction {
    let proposal_address = get_proposal_address(
        program_id,
        governance,
        governing_token_mint,
        &proposal_index.to_le_bytes(),
    );

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new(proposal_address, false),
        AccountMeta::new(*governance, false),
        AccountMeta::new(*proposal_owner_record, false),
        AccountMeta::new_readonly(*governing_token_mint, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    with_voter_weight_accounts(program_id, &mut accounts, realm, voter_weight_record);

    let instruction = GovernanceInstruction::CreateProposal {
        name,
        description_link,
        vote_type,
        options,
        use_deny_option,
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates AddSignatory instruction
pub fn add_signatory(
    program_id: &Pubkey,
    // Accounts
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    payer: &Pubkey,
    // Args
    signatory: &Pubkey,
) -> Instruction {
    let signatory_record_address = get_signatory_record_address(program_id, proposal, signatory);

    let accounts = vec![
        AccountMeta::new(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(signatory_record_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    let instruction = GovernanceInstruction::AddSignatory {
        signatory: *signatory,
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates RemoveSignatory instruction
pub fn remove_signatory(
    program_id: &Pubkey,
    // Accounts
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    signatory: &Pubkey,
    beneficiary: &Pubkey,
) -> Instruction {
    let signatory_record_address = get_signatory_record_address(program_id, proposal, signatory);

    let accounts = vec![
        AccountMeta::new(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(signatory_record_address, false),
        AccountMeta::new(*beneficiary, false),
    ];

    let instruction = GovernanceInstruction::RemoveSignatory {
        signatory: *signatory,
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates SignOffProposal instruction
pub fn sign_off_proposal(
    program_id: &Pubkey,
    // Accounts
    proposal: &Pubkey,
    signatory: &Pubkey,
) -> Instruction {
    let signatory_record_address = get_signatory_record_address(program_id, proposal, signatory);

    let accounts = vec![
        AccountMeta::new(*proposal, false),
        AccountMeta::new(signatory_record_address, false),
        AccountMeta::new_readonly(*signatory, true),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    let instruction = GovernanceInstruction::SignOffProposal;

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
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
    governing_token_mint: &Pubkey,
    payer: &Pubkey,
    voter_weight_record: Option<Pubkey>,
    // Args
    vote: Vote,
) -> Instruction {
    let vote_record_address =
        get_vote_record_address(program_id, proposal, voter_token_owner_record);

    let mut accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*proposal_owner_record, false),
        AccountMeta::new(*voter_token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(vote_record_address, false),
        AccountMeta::new_readonly(*governing_token_mint, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    with_voter_weight_accounts(program_id, &mut accounts, realm, voter_weight_record);

    let instruction = GovernanceInstruction::CastVote { vote };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
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
) -> Instruction {
    let accounts = vec![
        AccountMeta::new_readonly(*realm, false),
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*proposal_owner_record, false),
        AccountMeta::new_readonly(*governing_token_mint, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    let instruction = GovernanceInstruction::FinalizeVote {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates RelinquishVote instruction
pub fn relinquish_vote(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governing_token_mint: &Pubkey,
    governance_authority: Option<Pubkey>,
    beneficiary: Option<Pubkey>,
) -> Instruction {
    let vote_record_address = get_vote_record_address(program_id, proposal, token_owner_record);

    let mut accounts = vec![
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*token_owner_record, false),
        AccountMeta::new(vote_record_address, false),
        AccountMeta::new_readonly(*governing_token_mint, false),
    ];

    if let Some(governance_authority) = governance_authority {
        accounts.push(AccountMeta::new_readonly(governance_authority, true));
        accounts.push(AccountMeta::new(beneficiary.unwrap(), false));
    }

    let instruction = GovernanceInstruction::RelinquishVote {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates CancelProposal instruction
pub fn cancel_proposal(
    program_id: &Pubkey,
    // Accounts
    proposal: &Pubkey,
    proposal_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    governance: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*proposal_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(*governance, false),
    ];

    let instruction = GovernanceInstruction::CancelProposal {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates InsertInstruction instruction
#[allow(clippy::too_many_arguments)]
pub fn insert_instruction(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    payer: &Pubkey,
    // Args
    option_index: u16,
    index: u16,
    hold_up_time: u32,
    instruction: InstructionData,
) -> Instruction {
    let proposal_instruction_address = get_proposal_instruction_address(
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
        AccountMeta::new(proposal_instruction_address, false),
        AccountMeta::new(*payer, true),
        AccountMeta::new_readonly(system_program::id(), false),
        AccountMeta::new_readonly(sysvar::rent::id(), false),
    ];

    let instruction = GovernanceInstruction::InsertInstruction {
        option_index,
        index,
        hold_up_time,
        instruction,
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates RemoveInstruction instruction
pub fn remove_instruction(
    program_id: &Pubkey,
    // Accounts
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    proposal_instruction: &Pubkey,
    beneficiary: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(*proposal_instruction, false),
        AccountMeta::new(*beneficiary, false),
    ];

    let instruction = GovernanceInstruction::RemoveInstruction {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates ExecuteInstruction instruction
pub fn execute_instruction(
    program_id: &Pubkey,
    // Accounts
    governance: &Pubkey,
    proposal: &Pubkey,
    proposal_instruction: &Pubkey,
    instruction_program_id: &Pubkey,
    instruction_accounts: &[AccountMeta],
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new_readonly(*governance, false),
        AccountMeta::new(*proposal, false),
        AccountMeta::new(*proposal_instruction, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
        AccountMeta::new_readonly(*instruction_program_id, false),
    ];

    accounts.extend_from_slice(instruction_accounts);

    let instruction = GovernanceInstruction::ExecuteInstruction {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
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
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates FlagInstructionError instruction
pub fn flag_instruction_error(
    program_id: &Pubkey,
    // Accounts
    proposal: &Pubkey,
    token_owner_record: &Pubkey,
    governance_authority: &Pubkey,
    proposal_instruction: &Pubkey,
) -> Instruction {
    let accounts = vec![
        AccountMeta::new(*proposal, false),
        AccountMeta::new_readonly(*token_owner_record, false),
        AccountMeta::new_readonly(*governance_authority, true),
        AccountMeta::new(*proposal_instruction, false),
        AccountMeta::new_readonly(sysvar::clock::id(), false),
    ];

    let instruction = GovernanceInstruction::FlagInstructionError {};

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Creates SetRealmAuthority instruction
pub fn set_realm_authority(
    program_id: &Pubkey,
    // Accounts
    realm: &Pubkey,
    realm_authority: &Pubkey,
    new_realm_authority: &Option<Pubkey>,
    // Args
) -> Instruction {
    let mut accounts = vec![
        AccountMeta::new(*realm, false),
        AccountMeta::new_readonly(*realm_authority, true),
    ];

    let remove_authority = if let Some(new_realm_authority) = new_realm_authority {
        accounts.push(AccountMeta::new_readonly(*new_realm_authority, false));
        false
    } else {
        true
    };

    let instruction = GovernanceInstruction::SetRealmAuthority { remove_authority };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
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
    community_voter_weight_addin: Option<Pubkey>,
    // Args
    min_community_tokens_to_create_governance: u64,
    community_mint_max_vote_weight_source: MintMaxVoteWeightSource,
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

    // Always pass realm_config_address because it's needed when use_community_voter_weight_addin is set to true
    // but also when it's set to false and the addin is being  removed from the realm
    let realm_config_address = get_realm_config_address(program_id, realm);
    accounts.push(AccountMeta::new(realm_config_address, false));

    let use_community_voter_weight_addin =
        if let Some(community_voter_weight_addin) = community_voter_weight_addin {
            accounts.push(AccountMeta::new(*payer, true));
            accounts.push(AccountMeta::new_readonly(
                community_voter_weight_addin,
                false,
            ));
            true
        } else {
            false
        };

    let instruction = GovernanceInstruction::SetRealmConfig {
        config_args: RealmConfigArgs {
            use_council_mint,
            min_community_tokens_to_create_governance,
            community_mint_max_vote_weight_source,
            use_community_voter_weight_addin,
        },
    };

    Instruction {
        program_id: *program_id,
        accounts,
        data: instruction.try_to_vec().unwrap(),
    }
}

/// Adds voter weight accounts to the given accounts if voter_weight_record is Some
pub fn with_voter_weight_accounts(
    program_id: &Pubkey,
    accounts: &mut Vec<AccountMeta>,
    realm: &Pubkey,
    voter_weight_record: Option<Pubkey>,
) {
    if let Some(voter_weight_record) = voter_weight_record {
        let realm_config_address = get_realm_config_address(program_id, realm);
        accounts.push(AccountMeta::new_readonly(realm_config_address, false));
        accounts.push(AccountMeta::new_readonly(voter_weight_record, false));
    }
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
        data: instruction.try_to_vec().unwrap(),
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
        data: instruction.try_to_vec().unwrap(),
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
        data: instruction.try_to_vec().unwrap(),
    }
}
