use std::str::FromStr;

use solana_program::{
    bpf_loader_upgradeable::{self, UpgradeableLoaderState},
    clock::UnixTimestamp,
    instruction::{AccountMeta, Instruction},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    system_instruction, system_program,
};

use solana_program_test::*;

use solana_sdk::signature::{Keypair, Signer};

use spl_governance::{
    addins::voter_weight::{VoterWeightAccountType, VoterWeightRecord},
    instruction::{
        add_signatory, cancel_proposal, cast_vote, create_account_governance,
        create_mint_governance, create_native_treasury, create_program_governance, create_proposal,
        create_realm, create_token_governance, create_token_owner_record, deposit_governing_tokens,
        execute_instruction, finalize_vote, flag_instruction_error, insert_instruction,
        relinquish_vote, remove_instruction, remove_signatory, set_governance_config,
        set_governance_delegate, set_realm_authority, set_realm_config, sign_off_proposal,
        upgrade_program_metadata, withdraw_governing_tokens,
    },
    processor::process_instruction,
    state::{
        enums::{
            GovernanceAccountType, InstructionExecutionFlags, InstructionExecutionStatus,
            MintMaxVoteWeightSource, ProposalState, VoteThresholdPercentage,
        },
        governance::{
            get_account_governance_address, get_mint_governance_address,
            get_program_governance_address, get_token_governance_address, Governance,
            GovernanceConfig,
        },
        native_treasury::{get_native_treasury_address, NativeTreasury},
        program_metadata::{get_program_metadata_address, ProgramMetadata},
        proposal::{get_proposal_address, OptionVoteResult, ProposalOption, ProposalV2, VoteType},
        proposal_instruction::{
            get_proposal_instruction_address, InstructionData, ProposalInstructionV2,
        },
        realm::{
            get_governing_token_holding_address, get_realm_address, Realm, RealmConfig,
            RealmConfigArgs,
        },
        realm_config::{get_realm_config_address, RealmConfigAccount},
        signatory_record::{get_signatory_record_address, SignatoryRecord},
        token_owner_record::{get_token_owner_record_address, TokenOwnerRecord},
        vote_record::{get_vote_record_address, Vote, VoteChoice, VoteRecordV2},
    },
    tools::bpf_loader_upgradeable::get_program_data_address,
};

pub mod cookies;

use crate::program_test::cookies::{
    RealmConfigCookie, SignatoryRecordCookie, VoterWeightRecordCookie,
};

use spl_governance_test_sdk::{
    addins::ensure_voter_weight_addin_is_built,
    cookies::WalletCookie,
    tools::{clone_keypair, NopOverride},
    ProgramTestBench,
};

use self::cookies::{
    GovernanceCookie, GovernedAccountCookie, GovernedMintCookie, GovernedProgramCookie,
    GovernedTokenCookie, NativeTreasuryCookie, ProgramMetadataCookie, ProposalCookie,
    ProposalInstructionCookie, RealmCookie, TokenOwnerRecordCookie, VoteRecordCookie,
};

/// Yes/No Vote
pub enum YesNoVote {
    /// Yes vote
    #[allow(dead_code)]
    Yes,
    /// No vote
    #[allow(dead_code)]
    No,
}

pub struct GovernanceProgramTest {
    pub bench: ProgramTestBench,
    pub next_realm_id: u8,
    pub program_id: Pubkey,
    pub voter_weight_addin_id: Option<Pubkey>,
}

impl GovernanceProgramTest {
    #[allow(dead_code)]
    pub async fn start_new() -> Self {
        Self::start_impl(false).await
    }

    #[allow(dead_code)]
    pub async fn start_with_voter_weight_addin() -> Self {
        ensure_voter_weight_addin_is_built();

        Self::start_impl(true).await
    }

    #[allow(dead_code)]
    async fn start_impl(use_voter_weight_addin: bool) -> Self {
        let mut program_test = ProgramTest::default();

        let program_id = Pubkey::from_str("Governance111111111111111111111111111111111").unwrap();
        program_test.add_program(
            "spl_governance",
            program_id,
            processor!(process_instruction),
        );

        let voter_weight_addin_id = if use_voter_weight_addin {
            let voter_weight_addin_id =
                Pubkey::from_str("VoterWeight11111111111111111111111111111111").unwrap();
            program_test.add_program(
                "spl_governance_voter_weight_addin",
                voter_weight_addin_id,
                None,
            );
            Some(voter_weight_addin_id)
        } else {
            None
        };

        let bench = ProgramTestBench::start_new(program_test).await;

        Self {
            bench,
            next_realm_id: 0,
            program_id,
            voter_weight_addin_id,
        }
    }

    #[allow(dead_code)]
    pub fn get_default_realm_config_args(&mut self) -> RealmConfigArgs {
        RealmConfigArgs {
            use_council_mint: true,
            community_mint_max_vote_weight_source: MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
            min_community_tokens_to_create_governance: 10,
            use_community_voter_weight_addin: self.voter_weight_addin_id.is_some(),
        }
    }

    #[allow(dead_code)]
    pub async fn with_realm(&mut self) -> RealmCookie {
        let realm_config_args = self.get_default_realm_config_args();
        self.with_realm_using_config_args(&realm_config_args).await
    }

    #[allow(dead_code)]
    pub async fn with_realm_using_config_args(
        &mut self,
        config_args: &RealmConfigArgs,
    ) -> RealmCookie {
        let name = format!("Realm #{}", self.next_realm_id).to_string();
        self.next_realm_id += 1;

        let realm_address = get_realm_address(&self.program_id, &name);

        let community_token_mint_keypair = Keypair::new();
        let community_token_mint_authority = Keypair::new();

        let community_token_holding_address = get_governing_token_holding_address(
            &self.program_id,
            &realm_address,
            &community_token_mint_keypair.pubkey(),
        );

        self.bench
            .create_mint(
                &community_token_mint_keypair,
                &community_token_mint_authority.pubkey(),
            )
            .await;

        let (
            council_token_mint_pubkey,
            council_token_holding_address,
            council_token_mint_authority,
        ) = if config_args.use_council_mint {
            let council_token_mint_keypair = Keypair::new();
            let council_token_mint_authority = Keypair::new();

            let council_token_holding_address = get_governing_token_holding_address(
                &self.program_id,
                &realm_address,
                &council_token_mint_keypair.pubkey(),
            );

            self.bench
                .create_mint(
                    &council_token_mint_keypair,
                    &council_token_mint_authority.pubkey(),
                )
                .await;

            (
                Some(council_token_mint_keypair.pubkey()),
                Some(council_token_holding_address),
                Some(council_token_mint_authority),
            )
        } else {
            (None, None, None)
        };

        let realm_authority = Keypair::new();

        let community_voter_weight_addin = if config_args.use_community_voter_weight_addin {
            self.voter_weight_addin_id
        } else {
            None
        };

        let create_realm_instruction = create_realm(
            &self.program_id,
            &realm_authority.pubkey(),
            &community_token_mint_keypair.pubkey(),
            &self.bench.payer.pubkey(),
            council_token_mint_pubkey,
            community_voter_weight_addin,
            name.clone(),
            config_args.min_community_tokens_to_create_governance,
            config_args.community_mint_max_vote_weight_source.clone(),
        );

        self.bench
            .process_transaction(&[create_realm_instruction], None)
            .await
            .unwrap();

        let account = Realm {
            account_type: GovernanceAccountType::Realm,
            community_mint: community_token_mint_keypair.pubkey(),

            name,
            reserved: [0; 8],
            authority: Some(realm_authority.pubkey()),
            config: RealmConfig {
                council_mint: council_token_mint_pubkey,
                reserved: [0; 7],

                min_community_tokens_to_create_governance: config_args
                    .min_community_tokens_to_create_governance,
                community_mint_max_vote_weight_source: config_args
                    .community_mint_max_vote_weight_source
                    .clone(),
                use_community_voter_weight_addin: false,
            },
        };

        let realm_config_cookie = if config_args.use_community_voter_weight_addin {
            Some(RealmConfigCookie {
                address: get_realm_config_address(&self.program_id, &realm_address),
                account: RealmConfigAccount {
                    account_type: GovernanceAccountType::RealmConfig,
                    realm: realm_address,
                    community_voter_weight_addin: self.voter_weight_addin_id,
                    community_max_vote_weight_addin: None,
                    council_voter_weight_addin: None,
                    council_max_vote_weight_addin: None,
                    reserved: [0; 128],
                },
            })
        } else {
            None
        };

        RealmCookie {
            address: realm_address,
            account,

            community_mint_authority: community_token_mint_authority,
            community_token_holding_account: community_token_holding_address,

            council_token_holding_account: council_token_holding_address,
            council_mint_authority: council_token_mint_authority,
            realm_authority: Some(realm_authority),
            realm_config: realm_config_cookie,
        }
    }

    #[allow(dead_code)]
    pub async fn with_realm_using_mints(&mut self, realm_cookie: &RealmCookie) -> RealmCookie {
        let name = format!("Realm #{}", self.next_realm_id).to_string();
        self.next_realm_id += 1;

        let realm_address = get_realm_address(&self.program_id, &name);
        let council_mint = realm_cookie.account.config.council_mint.unwrap();

        let realm_authority = Keypair::new();

        let community_mint_max_vote_weight_source = MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION;
        let min_community_tokens_to_create_governance = 10;

        let create_realm_instruction = create_realm(
            &self.program_id,
            &realm_authority.pubkey(),
            &realm_cookie.account.community_mint,
            &self.bench.context.payer.pubkey(),
            Some(council_mint),
            None,
            name.clone(),
            min_community_tokens_to_create_governance,
            community_mint_max_vote_weight_source,
        );

        self.bench
            .process_transaction(&[create_realm_instruction], None)
            .await
            .unwrap();

        let account = Realm {
            account_type: GovernanceAccountType::Realm,
            community_mint: realm_cookie.account.community_mint,

            name,
            reserved: [0; 8],
            authority: Some(realm_authority.pubkey()),
            config: RealmConfig {
                council_mint: Some(council_mint),
                reserved: [0; 7],

                community_mint_max_vote_weight_source:
                    MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
                min_community_tokens_to_create_governance,
                use_community_voter_weight_addin: false,
            },
        };

        let community_token_holding_address = get_governing_token_holding_address(
            &self.program_id,
            &realm_address,
            &realm_cookie.account.community_mint,
        );

        let council_token_holding_address =
            get_governing_token_holding_address(&self.program_id, &realm_address, &council_mint);

        RealmCookie {
            address: realm_address,
            account,

            community_mint_authority: clone_keypair(&realm_cookie.community_mint_authority),
            community_token_holding_account: community_token_holding_address,

            council_token_holding_account: Some(council_token_holding_address),
            council_mint_authority: Some(clone_keypair(
                realm_cookie.council_mint_authority.as_ref().unwrap(),
            )),
            realm_authority: Some(realm_authority),
            realm_config: None,
        }
    }

    // Creates TokenOwner which owns 100 community tokens and deposits them into the given Realm
    #[allow(dead_code)]
    pub async fn with_community_token_deposit(
        &mut self,
        realm_cookie: &RealmCookie,
    ) -> Result<TokenOwnerRecordCookie, ProgramError> {
        self.with_initial_governing_token_deposit(
            &realm_cookie.address,
            &realm_cookie.account.community_mint,
            &realm_cookie.community_mint_authority,
            100,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_token_owner_record(
        &mut self,
        realm_cookie: &RealmCookie,
    ) -> TokenOwnerRecordCookie {
        let token_owner = Keypair::new();

        let create_token_owner_record_ix = create_token_owner_record(
            &self.program_id,
            &realm_cookie.address,
            &token_owner.pubkey(),
            &realm_cookie.account.community_mint,
            &self.bench.payer.pubkey(),
        );

        self.bench
            .process_transaction(&[create_token_owner_record_ix], None)
            .await
            .unwrap();

        let account = TokenOwnerRecord {
            account_type: GovernanceAccountType::TokenOwnerRecord,
            realm: realm_cookie.address,
            governing_token_mint: realm_cookie.account.community_mint,
            governing_token_owner: token_owner.pubkey(),
            governing_token_deposit_amount: 0,
            governance_delegate: None,
            unrelinquished_votes_count: 0,
            total_votes_count: 0,
            outstanding_proposal_count: 0,
            reserved: [0; 7],
        };

        let token_owner_record_address = get_token_owner_record_address(
            &self.program_id,
            &realm_cookie.address,
            &realm_cookie.account.community_mint,
            &token_owner.pubkey(),
        );

        TokenOwnerRecordCookie {
            address: token_owner_record_address,
            account,
            token_source_amount: 0,
            token_source: Pubkey::new_unique(),
            token_owner,
            governance_authority: None,
            governance_delegate: Keypair::new(),
            voter_weight_record: None,
        }
    }

    #[allow(dead_code)]
    pub async fn with_program_metadata(&mut self) -> ProgramMetadataCookie {
        let update_program_metadata_ix =
            upgrade_program_metadata(&self.program_id, &self.bench.payer.pubkey());

        self.bench
            .process_transaction(&[update_program_metadata_ix], None)
            .await
            .unwrap();

        const VERSION: &str = env!("CARGO_PKG_VERSION");
        let clock = self.bench.get_clock().await;

        let account = ProgramMetadata {
            account_type: GovernanceAccountType::ProgramMetadata,
            updated_at: clock.slot,
            version: VERSION.to_string(),
            reserved: [0; 64],
        };

        let program_metadata_address = get_program_metadata_address(&self.program_id);

        ProgramMetadataCookie {
            address: program_metadata_address,
            account,
        }
    }

    #[allow(dead_code)]
    pub async fn with_native_treasury(
        &mut self,
        governance_cookie: &GovernanceCookie,
    ) -> NativeTreasuryCookie {
        let create_treasury_ix = create_native_treasury(
            &self.program_id,
            &governance_cookie.address,
            &self.bench.payer.pubkey(),
        );

        let treasury_address =
            get_native_treasury_address(&self.program_id, &governance_cookie.address);

        let transfer_ix = system_instruction::transfer(
            &self.bench.payer.pubkey(),
            &treasury_address,
            1_000_000_000,
        );

        self.bench
            .process_transaction(&[create_treasury_ix, transfer_ix], None)
            .await
            .unwrap();

        NativeTreasuryCookie {
            address: treasury_address,
            account: NativeTreasury {},
        }
    }

    #[allow(dead_code)]
    pub async fn with_community_token_deposit_amount(
        &mut self,
        realm_cookie: &RealmCookie,
        amount: u64,
    ) -> Result<TokenOwnerRecordCookie, ProgramError> {
        self.with_initial_governing_token_deposit(
            &realm_cookie.address,
            &realm_cookie.account.community_mint,
            &realm_cookie.community_mint_authority,
            amount,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_subsequent_community_token_deposit(
        &mut self,
        realm_cookie: &RealmCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        amount: u64,
    ) {
        self.with_subsequent_governing_token_deposit(
            &realm_cookie.address,
            &realm_cookie.account.community_mint,
            &realm_cookie.community_mint_authority,
            token_owner_record_cookie,
            amount,
        )
        .await;
    }

    #[allow(dead_code)]
    pub async fn with_subsequent_council_token_deposit(
        &mut self,
        realm_cookie: &RealmCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        amount: u64,
    ) {
        self.with_subsequent_governing_token_deposit(
            &realm_cookie.address,
            &realm_cookie.account.config.council_mint.unwrap(),
            realm_cookie.council_mint_authority.as_ref().unwrap(),
            token_owner_record_cookie,
            amount,
        )
        .await;
    }

    #[allow(dead_code)]
    pub async fn with_council_token_deposit_amount(
        &mut self,
        realm_cookie: &RealmCookie,
        amount: u64,
    ) -> Result<TokenOwnerRecordCookie, ProgramError> {
        self.with_initial_governing_token_deposit(
            &realm_cookie.address,
            &realm_cookie.account.config.council_mint.unwrap(),
            &realm_cookie.council_mint_authority.as_ref().unwrap(),
            amount,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_council_token_deposit(
        &mut self,
        realm_cookie: &RealmCookie,
    ) -> Result<TokenOwnerRecordCookie, ProgramError> {
        self.with_initial_governing_token_deposit(
            &realm_cookie.address,
            &realm_cookie.account.config.council_mint.unwrap(),
            realm_cookie.council_mint_authority.as_ref().unwrap(),
            100,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_initial_governing_token_deposit(
        &mut self,
        realm_address: &Pubkey,
        governing_mint: &Pubkey,
        governing_mint_authority: &Keypair,
        amount: u64,
    ) -> Result<TokenOwnerRecordCookie, ProgramError> {
        let token_owner = Keypair::new();
        let token_source = Keypair::new();

        let transfer_authority = Keypair::new();

        self.bench
            .create_token_account_with_transfer_authority(
                &token_source,
                governing_mint,
                governing_mint_authority,
                amount,
                &token_owner,
                &transfer_authority.pubkey(),
            )
            .await;

        let deposit_governing_tokens_instruction = deposit_governing_tokens(
            &self.program_id,
            realm_address,
            &token_source.pubkey(),
            &token_owner.pubkey(),
            &token_owner.pubkey(),
            &self.bench.payer.pubkey(),
            amount,
            governing_mint,
        );

        self.bench
            .process_transaction(
                &[deposit_governing_tokens_instruction],
                Some(&[&token_owner]),
            )
            .await?;

        let token_owner_record_address = get_token_owner_record_address(
            &self.program_id,
            realm_address,
            governing_mint,
            &token_owner.pubkey(),
        );

        let account = TokenOwnerRecord {
            account_type: GovernanceAccountType::TokenOwnerRecord,
            realm: *realm_address,
            governing_token_mint: *governing_mint,
            governing_token_owner: token_owner.pubkey(),
            governing_token_deposit_amount: amount,
            governance_delegate: None,
            unrelinquished_votes_count: 0,
            total_votes_count: 0,
            outstanding_proposal_count: 0,
            reserved: [0; 7],
        };

        let governance_delegate = Keypair::from_base58_string(&token_owner.to_base58_string());

        Ok(TokenOwnerRecordCookie {
            address: token_owner_record_address,
            account,

            token_source_amount: amount,
            token_source: token_source.pubkey(),
            token_owner,
            governance_authority: None,
            governance_delegate,
            voter_weight_record: None,
        })
    }

    #[allow(dead_code)]
    pub async fn mint_community_tokens(&mut self, realm_cookie: &RealmCookie, amount: u64) {
        let token_account_keypair = Keypair::new();

        self.bench
            .create_empty_token_account(
                &token_account_keypair,
                &realm_cookie.account.community_mint,
                &self.bench.payer.pubkey(),
            )
            .await;

        self.bench
            .mint_tokens(
                &realm_cookie.account.community_mint,
                &realm_cookie.community_mint_authority,
                &token_account_keypair.pubkey(),
                amount,
            )
            .await;
    }

    #[allow(dead_code)]
    async fn with_subsequent_governing_token_deposit(
        &mut self,
        realm: &Pubkey,
        governing_token_mint: &Pubkey,
        governing_token_mint_authority: &Keypair,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        amount: u64,
    ) {
        self.bench
            .mint_tokens(
                governing_token_mint,
                governing_token_mint_authority,
                &token_owner_record_cookie.token_source,
                amount,
            )
            .await;

        let deposit_governing_tokens_instruction = deposit_governing_tokens(
            &self.program_id,
            realm,
            &token_owner_record_cookie.token_source,
            &token_owner_record_cookie.token_owner.pubkey(),
            &token_owner_record_cookie.token_owner.pubkey(),
            &self.bench.payer.pubkey(),
            amount,
            governing_token_mint,
        );

        self.bench
            .process_transaction(
                &[deposit_governing_tokens_instruction],
                Some(&[&token_owner_record_cookie.token_owner]),
            )
            .await
            .unwrap();
    }

    #[allow(dead_code)]
    pub async fn with_community_governance_delegate(
        &mut self,
        realm_cookie: &RealmCookie,
        token_owner_record_cookie: &mut TokenOwnerRecordCookie,
    ) {
        self.with_governing_token_governance_delegate(
            realm_cookie,
            &realm_cookie.account.community_mint,
            token_owner_record_cookie,
        )
        .await;
    }

    #[allow(dead_code)]
    pub async fn with_council_governance_delegate(
        &mut self,
        realm_cookie: &RealmCookie,
        token_owner_record_cookie: &mut TokenOwnerRecordCookie,
    ) {
        self.with_governing_token_governance_delegate(
            realm_cookie,
            &realm_cookie.account.config.council_mint.unwrap(),
            token_owner_record_cookie,
        )
        .await;
    }

    #[allow(dead_code)]
    pub async fn with_governing_token_governance_delegate(
        &mut self,
        realm_cookie: &RealmCookie,
        governing_token_mint: &Pubkey,
        token_owner_record_cookie: &mut TokenOwnerRecordCookie,
    ) {
        let new_governance_delegate = Keypair::new();

        self.set_governance_delegate(
            realm_cookie,
            token_owner_record_cookie,
            &token_owner_record_cookie.token_owner,
            governing_token_mint,
            &Some(new_governance_delegate.pubkey()),
        )
        .await;

        token_owner_record_cookie.governance_delegate = new_governance_delegate;
    }

    #[allow(dead_code)]
    pub async fn set_governance_delegate(
        &mut self,
        realm_cookie: &RealmCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        signing_governance_authority: &Keypair,
        governing_token_mint: &Pubkey,
        new_governance_delegate: &Option<Pubkey>,
    ) {
        let set_governance_delegate_instruction = set_governance_delegate(
            &self.program_id,
            &signing_governance_authority.pubkey(),
            &realm_cookie.address,
            governing_token_mint,
            &token_owner_record_cookie.token_owner.pubkey(),
            new_governance_delegate,
        );

        self.bench
            .process_transaction(
                &[set_governance_delegate_instruction],
                Some(&[signing_governance_authority]),
            )
            .await
            .unwrap();
    }

    #[allow(dead_code)]
    pub async fn set_realm_authority(
        &mut self,
        realm_cookie: &RealmCookie,
        new_realm_authority: &Option<Pubkey>,
    ) -> Result<(), ProgramError> {
        self.set_realm_authority_using_instruction(
            realm_cookie,
            new_realm_authority,
            NopOverride,
            None,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn set_realm_authority_using_instruction<F: Fn(&mut Instruction)>(
        &mut self,
        realm_cookie: &RealmCookie,
        new_realm_authority: &Option<Pubkey>,
        instruction_override: F,
        signers_override: Option<&[&Keypair]>,
    ) -> Result<(), ProgramError> {
        let mut set_realm_authority_ix = set_realm_authority(
            &self.program_id,
            &realm_cookie.address,
            &realm_cookie.realm_authority.as_ref().unwrap().pubkey(),
            new_realm_authority,
        );

        instruction_override(&mut set_realm_authority_ix);

        let default_signers = &[realm_cookie.realm_authority.as_ref().unwrap()];
        let signers = signers_override.unwrap_or(default_signers);

        self.bench
            .process_transaction(&[set_realm_authority_ix], Some(signers))
            .await
    }

    #[allow(dead_code)]
    pub async fn set_realm_config(
        &mut self,
        realm_cookie: &mut RealmCookie,
        realm_config_args: &RealmConfigArgs,
    ) -> Result<(), ProgramError> {
        self.set_realm_config_using_instruction(realm_cookie, realm_config_args, NopOverride, None)
            .await
    }

    #[allow(dead_code)]
    pub async fn set_realm_config_using_instruction<F: Fn(&mut Instruction)>(
        &mut self,
        realm_cookie: &mut RealmCookie,
        realm_config_args: &RealmConfigArgs,
        instruction_override: F,
        signers_override: Option<&[&Keypair]>,
    ) -> Result<(), ProgramError> {
        let council_token_mint = if realm_config_args.use_council_mint {
            realm_cookie.account.config.council_mint
        } else {
            None
        };

        let community_voter_weight_addin = if realm_config_args.use_community_voter_weight_addin {
            self.voter_weight_addin_id
        } else {
            None
        };

        let mut set_realm_config_ix = set_realm_config(
            &self.program_id,
            &realm_cookie.address,
            &realm_cookie.realm_authority.as_ref().unwrap().pubkey(),
            council_token_mint,
            &self.bench.payer.pubkey(),
            community_voter_weight_addin,
            realm_config_args.min_community_tokens_to_create_governance,
            realm_config_args
                .community_mint_max_vote_weight_source
                .clone(),
        );

        instruction_override(&mut set_realm_config_ix);

        let default_signers = &[realm_cookie.realm_authority.as_ref().unwrap()];
        let signers = signers_override.unwrap_or(default_signers);

        realm_cookie.account.config.council_mint = council_token_mint;
        realm_cookie
            .account
            .config
            .community_mint_max_vote_weight_source = realm_config_args
            .community_mint_max_vote_weight_source
            .clone();

        if realm_config_args.use_community_voter_weight_addin {
            let community_voter_weight_addin_index = if realm_config_args.use_council_mint {
                7
            } else {
                5
            };
            realm_cookie.realm_config = Some(RealmConfigCookie {
                address: get_realm_config_address(&self.program_id, &realm_cookie.address),
                account: RealmConfigAccount {
                    account_type: GovernanceAccountType::RealmConfig,
                    realm: realm_cookie.address,
                    community_voter_weight_addin: Some(
                        set_realm_config_ix.accounts[community_voter_weight_addin_index].pubkey,
                    ),
                    community_max_vote_weight_addin: None,
                    council_voter_weight_addin: None,
                    council_max_vote_weight_addin: None,
                    reserved: [0; 128],
                },
            })
        }

        self.bench
            .process_transaction(&[set_realm_config_ix], Some(signers))
            .await
    }

    #[allow(dead_code)]
    pub async fn withdraw_community_tokens(
        &mut self,
        realm_cookie: &RealmCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
    ) -> Result<(), ProgramError> {
        self.withdraw_governing_tokens(
            realm_cookie,
            token_owner_record_cookie,
            &realm_cookie.account.community_mint,
            &token_owner_record_cookie.token_owner,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn withdraw_council_tokens(
        &mut self,
        realm_cookie: &RealmCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
    ) -> Result<(), ProgramError> {
        self.withdraw_governing_tokens(
            realm_cookie,
            token_owner_record_cookie,
            &realm_cookie.account.config.council_mint.unwrap(),
            &token_owner_record_cookie.token_owner,
        )
        .await
    }

    #[allow(dead_code)]
    async fn withdraw_governing_tokens(
        &mut self,
        realm_cookie: &RealmCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        governing_token_mint: &Pubkey,

        governing_token_owner: &Keypair,
    ) -> Result<(), ProgramError> {
        let deposit_governing_tokens_instruction = withdraw_governing_tokens(
            &self.program_id,
            &realm_cookie.address,
            &token_owner_record_cookie.token_source,
            &governing_token_owner.pubkey(),
            governing_token_mint,
        );

        self.bench
            .process_transaction(
                &[deposit_governing_tokens_instruction],
                Some(&[governing_token_owner]),
            )
            .await
    }

    #[allow(dead_code)]
    pub async fn with_governed_account(&mut self) -> GovernedAccountCookie {
        GovernedAccountCookie {
            address: Pubkey::new_unique(),
        }
    }

    #[allow(dead_code)]
    pub async fn with_governed_mint(&mut self) -> GovernedMintCookie {
        let mint_keypair = Keypair::new();
        let mint_authority = Keypair::new();

        self.bench
            .create_mint(&mint_keypair, &mint_authority.pubkey())
            .await;

        GovernedMintCookie {
            address: mint_keypair.pubkey(),
            mint_authority,
            transfer_mint_authority: true,
        }
    }

    #[allow(dead_code)]
    pub async fn with_governed_token(&mut self) -> GovernedTokenCookie {
        let mint_keypair = Keypair::new();
        let mint_authority = Keypair::new();

        self.bench
            .create_mint(&mint_keypair, &mint_authority.pubkey())
            .await;

        let token_keypair = Keypair::new();
        let token_owner = Keypair::new();

        self.bench
            .create_empty_token_account(
                &token_keypair,
                &mint_keypair.pubkey(),
                &token_owner.pubkey(),
            )
            .await;

        self.bench
            .mint_tokens(
                &mint_keypair.pubkey(),
                &mint_authority,
                &token_keypair.pubkey(),
                100,
            )
            .await;

        GovernedTokenCookie {
            address: token_keypair.pubkey(),
            token_owner,
            transfer_token_owner: true,
            token_mint: mint_keypair.pubkey(),
        }
    }

    pub fn get_default_governance_config(&mut self) -> GovernanceConfig {
        GovernanceConfig {
            min_community_tokens_to_create_proposal: 5,
            min_council_tokens_to_create_proposal: 2,
            min_instruction_hold_up_time: 10,
            max_voting_time: 10,
            vote_threshold_percentage: VoteThresholdPercentage::YesVote(60),
            vote_weight_source: spl_governance::state::enums::VoteWeightSource::Deposit,
            proposal_cool_off_time: 0,
        }
    }

    #[allow(dead_code)]
    pub async fn with_account_governance(
        &mut self,
        realm_cookie: &RealmCookie,
        governed_account_cookie: &GovernedAccountCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
    ) -> Result<GovernanceCookie, ProgramError> {
        let config = self.get_default_governance_config();
        self.with_account_governance_using_config(
            realm_cookie,
            governed_account_cookie,
            token_owner_record_cookie,
            &config,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_account_governance_using_config(
        &mut self,
        realm_cookie: &RealmCookie,
        governed_account_cookie: &GovernedAccountCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        governance_config: &GovernanceConfig,
    ) -> Result<GovernanceCookie, ProgramError> {
        let voter_weight_record =
            if let Some(voter_weight_record) = &token_owner_record_cookie.voter_weight_record {
                Some(voter_weight_record.address)
            } else {
                None
            };

        let create_account_governance_instruction = create_account_governance(
            &self.program_id,
            &realm_cookie.address,
            &governed_account_cookie.address,
            &token_owner_record_cookie.address,
            &self.bench.payer.pubkey(),
            &token_owner_record_cookie.token_owner.pubkey(),
            voter_weight_record,
            governance_config.clone(),
        );

        let account = Governance {
            account_type: GovernanceAccountType::AccountGovernance,
            realm: realm_cookie.address,
            governed_account: governed_account_cookie.address,
            config: governance_config.clone(),
            proposals_count: 0,
            reserved: [0; 8],
        };

        self.bench
            .process_transaction(
                &[create_account_governance_instruction],
                Some(&[&token_owner_record_cookie.token_owner]),
            )
            .await?;

        let account_governance_address = get_account_governance_address(
            &self.program_id,
            &realm_cookie.address,
            &governed_account_cookie.address,
        );

        Ok(GovernanceCookie {
            address: account_governance_address,
            account,
            next_proposal_index: 0,
        })
    }

    #[allow(dead_code)]
    pub async fn with_governed_program(&mut self) -> GovernedProgramCookie {
        let program_keypair = Keypair::new();
        let program_buffer_keypair = Keypair::new();
        let program_upgrade_authority_keypair = Keypair::new();

        let program_data_address = get_program_data_address(&program_keypair.pubkey());

        // Load solana_bpf_rust_upgradeable program taken from solana test programs
        let path_buf = find_file("solana_bpf_rust_upgradeable.so").unwrap();
        let program_data = read_file(path_buf);

        let program_buffer_rent = self
            .bench
            .rent
            .minimum_balance(UpgradeableLoaderState::programdata_len(program_data.len()).unwrap());

        let mut instructions = bpf_loader_upgradeable::create_buffer(
            &self.bench.payer.pubkey(),
            &program_buffer_keypair.pubkey(),
            &program_upgrade_authority_keypair.pubkey(),
            program_buffer_rent,
            program_data.len(),
        )
        .unwrap();

        let chunk_size = 800;

        for (chunk, i) in program_data.chunks(chunk_size).zip(0..) {
            instructions.push(bpf_loader_upgradeable::write(
                &program_buffer_keypair.pubkey(),
                &program_upgrade_authority_keypair.pubkey(),
                (i * chunk_size) as u32,
                chunk.to_vec(),
            ));
        }

        let program_account_rent = self
            .bench
            .rent
            .minimum_balance(UpgradeableLoaderState::program_len().unwrap());

        let deploy_instructions = bpf_loader_upgradeable::deploy_with_max_program_len(
            &self.bench.payer.pubkey(),
            &program_keypair.pubkey(),
            &program_buffer_keypair.pubkey(),
            &program_upgrade_authority_keypair.pubkey(),
            program_account_rent,
            program_data.len(),
        )
        .unwrap();

        instructions.extend_from_slice(&deploy_instructions);

        self.bench
            .process_transaction(
                &instructions[..],
                Some(&[
                    &program_upgrade_authority_keypair,
                    &program_keypair,
                    &program_buffer_keypair,
                ]),
            )
            .await
            .unwrap();

        GovernedProgramCookie {
            address: program_keypair.pubkey(),
            upgrade_authority: program_upgrade_authority_keypair,
            data_address: program_data_address,
            transfer_upgrade_authority: true,
        }
    }

    #[allow(dead_code)]
    pub async fn with_program_governance(
        &mut self,
        realm_cookie: &RealmCookie,
        governed_program_cookie: &GovernedProgramCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
    ) -> Result<GovernanceCookie, ProgramError> {
        self.with_program_governance_using_instruction(
            realm_cookie,
            governed_program_cookie,
            token_owner_record_cookie,
            NopOverride,
            None,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_program_governance_using_instruction<F: Fn(&mut Instruction)>(
        &mut self,
        realm_cookie: &RealmCookie,
        governed_program_cookie: &GovernedProgramCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        instruction_override: F,
        signers_override: Option<&[&Keypair]>,
    ) -> Result<GovernanceCookie, ProgramError> {
        let config = self.get_default_governance_config();

        let voter_weight_record =
            if let Some(voter_weight_record) = &token_owner_record_cookie.voter_weight_record {
                Some(voter_weight_record.address)
            } else {
                None
            };

        let mut create_program_governance_instruction = create_program_governance(
            &self.program_id,
            &realm_cookie.address,
            &governed_program_cookie.address,
            &governed_program_cookie.upgrade_authority.pubkey(),
            &token_owner_record_cookie.address,
            &self.bench.payer.pubkey(),
            &token_owner_record_cookie.token_owner.pubkey(),
            voter_weight_record,
            config.clone(),
            governed_program_cookie.transfer_upgrade_authority,
        );

        instruction_override(&mut create_program_governance_instruction);

        let default_signers = &[
            &governed_program_cookie.upgrade_authority,
            &token_owner_record_cookie.token_owner,
        ];
        let signers = signers_override.unwrap_or(default_signers);

        self.bench
            .process_transaction(&[create_program_governance_instruction], Some(signers))
            .await?;

        let account = Governance {
            account_type: GovernanceAccountType::ProgramGovernance,
            realm: realm_cookie.address,
            governed_account: governed_program_cookie.address,
            config,
            proposals_count: 0,
            reserved: [0; 8],
        };

        let program_governance_address = get_program_governance_address(
            &self.program_id,
            &realm_cookie.address,
            &governed_program_cookie.address,
        );

        Ok(GovernanceCookie {
            address: program_governance_address,
            account,
            next_proposal_index: 0,
        })
    }

    #[allow(dead_code)]
    pub async fn with_mint_governance(
        &mut self,
        realm_cookie: &RealmCookie,
        governed_mint_cookie: &GovernedMintCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
    ) -> Result<GovernanceCookie, ProgramError> {
        self.with_mint_governance_using_instruction(
            realm_cookie,
            governed_mint_cookie,
            token_owner_record_cookie,
            NopOverride,
            None,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_mint_governance_using_config(
        &mut self,
        realm_cookie: &RealmCookie,
        governed_mint_cookie: &GovernedMintCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        governance_config: &GovernanceConfig,
    ) -> Result<GovernanceCookie, ProgramError> {
        self.with_mint_governance_using_config_and_instruction(
            realm_cookie,
            governed_mint_cookie,
            token_owner_record_cookie,
            governance_config,
            NopOverride,
            None,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_mint_governance_using_instruction<F: Fn(&mut Instruction)>(
        &mut self,
        realm_cookie: &RealmCookie,
        governed_mint_cookie: &GovernedMintCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        instruction_override: F,
        signers_override: Option<&[&Keypair]>,
    ) -> Result<GovernanceCookie, ProgramError> {
        let governance_config = self.get_default_governance_config();

        self.with_mint_governance_using_config_and_instruction(
            realm_cookie,
            governed_mint_cookie,
            token_owner_record_cookie,
            &governance_config,
            instruction_override,
            signers_override,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_mint_governance_using_config_and_instruction<F: Fn(&mut Instruction)>(
        &mut self,
        realm_cookie: &RealmCookie,
        governed_mint_cookie: &GovernedMintCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        governance_config: &GovernanceConfig,
        instruction_override: F,
        signers_override: Option<&[&Keypair]>,
    ) -> Result<GovernanceCookie, ProgramError> {
        let voter_weight_record =
            if let Some(voter_weight_record) = &token_owner_record_cookie.voter_weight_record {
                Some(voter_weight_record.address)
            } else {
                None
            };

        let mut create_mint_governance_instruction = create_mint_governance(
            &self.program_id,
            &realm_cookie.address,
            &governed_mint_cookie.address,
            &governed_mint_cookie.mint_authority.pubkey(),
            &token_owner_record_cookie.address,
            &self.bench.payer.pubkey(),
            &token_owner_record_cookie.token_owner.pubkey(),
            voter_weight_record,
            governance_config.clone(),
            governed_mint_cookie.transfer_mint_authority,
        );

        instruction_override(&mut create_mint_governance_instruction);

        let default_signers = &[
            &governed_mint_cookie.mint_authority,
            &token_owner_record_cookie.token_owner,
        ];
        let signers = signers_override.unwrap_or(default_signers);

        self.bench
            .process_transaction(&[create_mint_governance_instruction], Some(signers))
            .await?;

        let account = Governance {
            account_type: GovernanceAccountType::MintGovernance,
            realm: realm_cookie.address,
            governed_account: governed_mint_cookie.address,
            config: governance_config.clone(),
            proposals_count: 0,
            reserved: [0; 8],
        };

        let mint_governance_address = get_mint_governance_address(
            &self.program_id,
            &realm_cookie.address,
            &governed_mint_cookie.address,
        );

        Ok(GovernanceCookie {
            address: mint_governance_address,
            account,
            next_proposal_index: 0,
        })
    }

    #[allow(dead_code)]
    pub async fn with_token_governance(
        &mut self,
        realm_cookie: &RealmCookie,
        governed_token_cookie: &GovernedTokenCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
    ) -> Result<GovernanceCookie, ProgramError> {
        self.with_token_governance_using_instruction(
            realm_cookie,
            governed_token_cookie,
            &token_owner_record_cookie,
            NopOverride,
            None,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_token_governance_using_instruction<F: Fn(&mut Instruction)>(
        &mut self,
        realm_cookie: &RealmCookie,
        governed_token_cookie: &GovernedTokenCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        instruction_override: F,
        signers_override: Option<&[&Keypair]>,
    ) -> Result<GovernanceCookie, ProgramError> {
        let config = self.get_default_governance_config();

        let voter_weight_record =
            if let Some(voter_weight_record) = &token_owner_record_cookie.voter_weight_record {
                Some(voter_weight_record.address)
            } else {
                None
            };

        let mut create_token_governance_instruction = create_token_governance(
            &self.program_id,
            &realm_cookie.address,
            &governed_token_cookie.address,
            &governed_token_cookie.token_owner.pubkey(),
            &token_owner_record_cookie.address,
            &self.bench.payer.pubkey(),
            &token_owner_record_cookie.token_owner.pubkey(),
            voter_weight_record,
            config.clone(),
            governed_token_cookie.transfer_token_owner,
        );

        instruction_override(&mut create_token_governance_instruction);

        let default_signers = &[
            &governed_token_cookie.token_owner,
            &token_owner_record_cookie.token_owner,
        ];
        let signers = signers_override.unwrap_or(default_signers);

        self.bench
            .process_transaction(&[create_token_governance_instruction], Some(signers))
            .await?;

        let account = Governance {
            account_type: GovernanceAccountType::TokenGovernance,
            realm: realm_cookie.address,
            governed_account: governed_token_cookie.address,
            config,
            proposals_count: 0,
            reserved: [0; 8],
        };

        let token_governance_address = get_token_governance_address(
            &self.program_id,
            &realm_cookie.address,
            &governed_token_cookie.address,
        );

        Ok(GovernanceCookie {
            address: token_governance_address,
            account,
            next_proposal_index: 0,
        })
    }

    #[allow(dead_code)]
    pub async fn with_proposal(
        &mut self,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        governance_cookie: &mut GovernanceCookie,
    ) -> Result<ProposalCookie, ProgramError> {
        self.with_proposal_using_instruction(
            token_owner_record_cookie,
            governance_cookie,
            NopOverride,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_multi_option_proposal(
        &mut self,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        governance_cookie: &mut GovernanceCookie,
        options: Vec<String>,
        use_deny_option: bool,
        vote_type: VoteType,
    ) -> Result<ProposalCookie, ProgramError> {
        self.with_proposal_using_instruction_impl(
            token_owner_record_cookie,
            governance_cookie,
            options,
            use_deny_option,
            vote_type,
            NopOverride,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_signed_off_proposal(
        &mut self,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        governance_cookie: &mut GovernanceCookie,
    ) -> Result<ProposalCookie, ProgramError> {
        let proposal_cookie = self
            .with_proposal(token_owner_record_cookie, governance_cookie)
            .await?;

        let signatory_record_cookie = self
            .with_signatory(&proposal_cookie, token_owner_record_cookie)
            .await?;

        self.sign_off_proposal(&proposal_cookie, &signatory_record_cookie)
            .await?;

        Ok(proposal_cookie)
    }

    #[allow(dead_code)]
    pub async fn with_proposal_using_instruction<F: Fn(&mut Instruction)>(
        &mut self,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        governance_cookie: &mut GovernanceCookie,
        instruction_override: F,
    ) -> Result<ProposalCookie, ProgramError> {
        let options = vec!["Yes".to_string()];

        self.with_proposal_using_instruction_impl(
            token_owner_record_cookie,
            governance_cookie,
            options,
            true,
            VoteType::SingleChoice,
            instruction_override,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_proposal_using_instruction_impl<F: Fn(&mut Instruction)>(
        &mut self,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        governance_cookie: &mut GovernanceCookie,
        options: Vec<String>,
        use_deny_option: bool,
        vote_type: VoteType,
        instruction_override: F,
    ) -> Result<ProposalCookie, ProgramError> {
        let proposal_index = governance_cookie.next_proposal_index;
        governance_cookie.next_proposal_index += 1;

        let name = format!("Proposal #{}", proposal_index);

        let description_link = "Proposal Description".to_string();

        let governance_authority = token_owner_record_cookie.get_governance_authority();

        let voter_weight_record =
            if let Some(voter_weight_record) = &token_owner_record_cookie.voter_weight_record {
                Some(voter_weight_record.address)
            } else {
                None
            };

        let mut create_proposal_instruction = create_proposal(
            &self.program_id,
            &governance_cookie.address,
            &token_owner_record_cookie.address,
            &governance_authority.pubkey(),
            &self.bench.payer.pubkey(),
            voter_weight_record,
            &governance_cookie.account.realm,
            name.clone(),
            description_link.clone(),
            &token_owner_record_cookie.account.governing_token_mint,
            vote_type.clone(),
            options.clone(),
            use_deny_option,
            proposal_index,
        );

        instruction_override(&mut create_proposal_instruction);

        self.bench
            .process_transaction(
                &[create_proposal_instruction],
                Some(&[governance_authority]),
            )
            .await?;

        let clock = self.bench.get_clock().await;

        let proposal_options: Vec<ProposalOption> = options
            .iter()
            .map(|o| ProposalOption {
                label: o.to_string(),
                vote_weight: 0,
                vote_result: OptionVoteResult::None,
                instructions_executed_count: 0,
                instructions_count: 0,
                instructions_next_index: 0,
            })
            .collect();

        let deny_vote_weight = if use_deny_option { Some(0) } else { None };

        let account = ProposalV2 {
            account_type: GovernanceAccountType::ProposalV2,
            description_link,
            name: name.clone(),
            governance: governance_cookie.address,
            governing_token_mint: token_owner_record_cookie.account.governing_token_mint,
            state: ProposalState::Draft,
            signatories_count: 0,

            draft_at: clock.unix_timestamp,
            signing_off_at: None,

            voting_at: None,
            voting_at_slot: None,
            voting_completed_at: None,
            executing_at: None,
            closed_at: None,

            token_owner_record: token_owner_record_cookie.address,
            signatories_signed_off_count: 0,

            vote_type: vote_type,
            options: proposal_options,
            deny_vote_weight,

            execution_flags: InstructionExecutionFlags::None,
            max_vote_weight: None,
            vote_threshold_percentage: None,
        };

        let proposal_address = get_proposal_address(
            &self.program_id,
            &governance_cookie.address,
            &token_owner_record_cookie.account.governing_token_mint,
            &proposal_index.to_le_bytes(),
        );

        Ok(ProposalCookie {
            address: proposal_address,
            account,
            proposal_owner: governance_authority.pubkey(),
        })
    }

    #[allow(dead_code)]
    pub async fn with_signatory(
        &mut self,
        proposal_cookie: &ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
    ) -> Result<SignatoryRecordCookie, ProgramError> {
        let signatory = Keypair::new();

        let add_signatory_instruction = add_signatory(
            &self.program_id,
            &proposal_cookie.address,
            &token_owner_record_cookie.address,
            &token_owner_record_cookie.token_owner.pubkey(),
            &self.bench.payer.pubkey(),
            &signatory.pubkey(),
        );

        self.bench
            .process_transaction(
                &[add_signatory_instruction],
                Some(&[&token_owner_record_cookie.token_owner]),
            )
            .await?;

        let signatory_record_address = get_signatory_record_address(
            &self.program_id,
            &proposal_cookie.address,
            &signatory.pubkey(),
        );

        let signatory_record_data = SignatoryRecord {
            account_type: GovernanceAccountType::SignatoryRecord,
            proposal: proposal_cookie.address,
            signatory: signatory.pubkey(),
            signed_off: false,
        };

        let signatory_record_cookie = SignatoryRecordCookie {
            address: signatory_record_address,
            account: signatory_record_data,
            signatory,
        };

        Ok(signatory_record_cookie)
    }

    #[allow(dead_code)]
    pub async fn remove_signatory(
        &mut self,
        proposal_cookie: &ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        signatory_record_cookie: &SignatoryRecordCookie,
    ) -> Result<(), ProgramError> {
        let remove_signatory_instruction = remove_signatory(
            &self.program_id,
            &proposal_cookie.address,
            &token_owner_record_cookie.address,
            &token_owner_record_cookie.token_owner.pubkey(),
            &signatory_record_cookie.account.signatory,
            &token_owner_record_cookie.token_owner.pubkey(),
        );

        self.bench
            .process_transaction(
                &[remove_signatory_instruction],
                Some(&[&token_owner_record_cookie.token_owner]),
            )
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn sign_off_proposal(
        &mut self,
        proposal_cookie: &ProposalCookie,
        signatory_record_cookie: &SignatoryRecordCookie,
    ) -> Result<(), ProgramError> {
        self.sign_off_proposal_using_instruction(
            proposal_cookie,
            signatory_record_cookie,
            NopOverride,
            None,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn sign_off_proposal_using_instruction<F: Fn(&mut Instruction)>(
        &mut self,
        proposal_cookie: &ProposalCookie,
        signatory_record_cookie: &SignatoryRecordCookie,
        instruction_override: F,
        signers_override: Option<&[&Keypair]>,
    ) -> Result<(), ProgramError> {
        let mut sign_off_proposal_ix = sign_off_proposal(
            &self.program_id,
            &proposal_cookie.address,
            &signatory_record_cookie.signatory.pubkey(),
        );

        instruction_override(&mut sign_off_proposal_ix);

        let default_signers = &[&signatory_record_cookie.signatory];
        let signers = signers_override.unwrap_or(default_signers);

        self.bench
            .process_transaction(&[sign_off_proposal_ix], Some(signers))
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn finalize_vote(
        &mut self,
        realm_cookie: &RealmCookie,
        proposal_cookie: &ProposalCookie,
    ) -> Result<(), ProgramError> {
        let finalize_vote_instruction = finalize_vote(
            &self.program_id,
            &realm_cookie.address,
            &proposal_cookie.account.governance,
            &proposal_cookie.address,
            &proposal_cookie.account.token_owner_record,
            &proposal_cookie.account.governing_token_mint,
        );

        self.bench
            .process_transaction(&[finalize_vote_instruction], None)
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn relinquish_vote(
        &mut self,
        proposal_cookie: &ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
    ) -> Result<(), ProgramError> {
        self.relinquish_vote_using_instruction(
            proposal_cookie,
            token_owner_record_cookie,
            NopOverride,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn relinquish_vote_using_instruction<F: Fn(&mut Instruction)>(
        &mut self,
        proposal_cookie: &ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        instruction_override: F,
    ) -> Result<(), ProgramError> {
        let mut relinquish_vote_instruction = relinquish_vote(
            &self.program_id,
            &proposal_cookie.account.governance,
            &proposal_cookie.address,
            &token_owner_record_cookie.address,
            &proposal_cookie.account.governing_token_mint,
            Some(token_owner_record_cookie.token_owner.pubkey()),
            Some(self.bench.payer.pubkey()),
        );

        instruction_override(&mut relinquish_vote_instruction);

        self.bench
            .process_transaction(
                &[relinquish_vote_instruction],
                Some(&[&token_owner_record_cookie.token_owner]),
            )
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn cancel_proposal(
        &mut self,
        proposal_cookie: &ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
    ) -> Result<(), ProgramError> {
        let cancel_proposal_instruction = cancel_proposal(
            &self.program_id,
            &proposal_cookie.address,
            &token_owner_record_cookie.address,
            &token_owner_record_cookie.token_owner.pubkey(),
            &proposal_cookie.account.governance,
        );

        self.bench
            .process_transaction(
                &[cancel_proposal_instruction],
                Some(&[&token_owner_record_cookie.token_owner]),
            )
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn with_cast_vote(
        &mut self,
        proposal_cookie: &ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        yes_no_vote: YesNoVote,
    ) -> Result<VoteRecordCookie, ProgramError> {
        let vote = match yes_no_vote {
            YesNoVote::Yes => Vote::Approve(vec![VoteChoice {
                rank: 0,
                weight_percentage: 100,
            }]),
            YesNoVote::No => Vote::Deny,
        };

        self.with_cast_multi_option_vote(proposal_cookie, token_owner_record_cookie, vote)
            .await
    }

    #[allow(dead_code)]
    pub async fn with_cast_multi_option_vote(
        &mut self,
        proposal_cookie: &ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        vote: Vote,
    ) -> Result<VoteRecordCookie, ProgramError> {
        let voter_weight_record =
            if let Some(voter_weight_record) = &token_owner_record_cookie.voter_weight_record {
                Some(voter_weight_record.address)
            } else {
                None
            };

        let vote_instruction = cast_vote(
            &self.program_id,
            &token_owner_record_cookie.account.realm,
            &proposal_cookie.account.governance,
            &proposal_cookie.address,
            &proposal_cookie.account.token_owner_record,
            &token_owner_record_cookie.address,
            &token_owner_record_cookie.token_owner.pubkey(),
            &proposal_cookie.account.governing_token_mint,
            &self.bench.payer.pubkey(),
            voter_weight_record,
            vote.clone(),
        );

        self.bench
            .process_transaction(
                &[vote_instruction],
                Some(&[&token_owner_record_cookie.token_owner]),
            )
            .await?;

        let vote_amount = token_owner_record_cookie
            .account
            .governing_token_deposit_amount;

        let account = VoteRecordV2 {
            account_type: GovernanceAccountType::VoteRecordV2,
            proposal: proposal_cookie.address,
            governing_token_owner: token_owner_record_cookie.token_owner.pubkey(),
            vote,
            voter_weight: vote_amount,
            is_relinquished: false,
        };

        let vote_record_cookie = VoteRecordCookie {
            address: get_vote_record_address(
                &self.program_id,
                &proposal_cookie.address,
                &token_owner_record_cookie.address,
            ),
            account,
        };

        Ok(vote_record_cookie)
    }

    #[allow(dead_code)]
    pub async fn with_set_governance_config_instruction(
        &mut self,
        proposal_cookie: &mut ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        governance_config: &GovernanceConfig,
    ) -> Result<ProposalInstructionCookie, ProgramError> {
        let mut set_governance_config_ix = set_governance_config(
            &self.program_id,
            &proposal_cookie.account.governance,
            governance_config.clone(),
        );

        self.with_instruction(
            proposal_cookie,
            token_owner_record_cookie,
            0,
            None,
            &mut set_governance_config_ix,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_mint_tokens_instruction(
        &mut self,
        governed_mint_cookie: &GovernedMintCookie,
        proposal_cookie: &mut ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        option_index: u16,
        index: Option<u16>,
    ) -> Result<ProposalInstructionCookie, ProgramError> {
        let token_account_keypair = Keypair::new();
        self.bench
            .create_empty_token_account(
                &token_account_keypair,
                &governed_mint_cookie.address,
                &self.bench.payer.pubkey(),
            )
            .await;

        let mut instruction = spl_token::instruction::mint_to(
            &spl_token::id(),
            &governed_mint_cookie.address,
            &token_account_keypair.pubkey(),
            &proposal_cookie.account.governance,
            &[],
            10,
        )
        .unwrap();

        self.with_instruction(
            proposal_cookie,
            token_owner_record_cookie,
            option_index,
            index,
            &mut instruction,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_transfer_tokens_instruction(
        &mut self,
        governed_token_cookie: &GovernedTokenCookie,
        proposal_cookie: &mut ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        index: Option<u16>,
    ) -> Result<ProposalInstructionCookie, ProgramError> {
        let token_account_keypair = Keypair::new();
        self.bench
            .create_empty_token_account(
                &token_account_keypair,
                &governed_token_cookie.token_mint,
                &self.bench.payer.pubkey(),
            )
            .await;

        let mut instruction = spl_token::instruction::transfer(
            &spl_token::id(),
            &governed_token_cookie.address,
            &token_account_keypair.pubkey(),
            &proposal_cookie.account.governance,
            &[],
            15,
        )
        .unwrap();

        self.with_instruction(
            proposal_cookie,
            token_owner_record_cookie,
            0,
            index,
            &mut instruction,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_native_transfer_instruction(
        &mut self,
        governance_cookie: &GovernanceCookie,
        proposal_cookie: &mut ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        to_wallet_cookie: &WalletCookie,
        lamports: u64,
    ) -> Result<ProposalInstructionCookie, ProgramError> {
        let treasury_address =
            get_native_treasury_address(&self.program_id, &governance_cookie.address);

        let mut transfer_ix =
            system_instruction::transfer(&treasury_address, &to_wallet_cookie.address, lamports);

        self.with_instruction(
            proposal_cookie,
            token_owner_record_cookie,
            0,
            None,
            &mut transfer_ix,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_upgrade_program_instruction(
        &mut self,
        governance_cookie: &GovernanceCookie,
        proposal_cookie: &mut ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
    ) -> Result<ProposalInstructionCookie, ProgramError> {
        let program_buffer_keypair = Keypair::new();
        let buffer_authority_keypair = Keypair::new();

        // Load solana_bpf_rust_upgraded program taken from solana test programs
        let path_buf = find_file("solana_bpf_rust_upgraded.so").unwrap();
        let program_data = read_file(path_buf);

        let program_buffer_rent = self
            .bench
            .rent
            .minimum_balance(UpgradeableLoaderState::programdata_len(program_data.len()).unwrap());

        let mut instructions = bpf_loader_upgradeable::create_buffer(
            &self.bench.payer.pubkey(),
            &program_buffer_keypair.pubkey(),
            &buffer_authority_keypair.pubkey(),
            program_buffer_rent,
            program_data.len(),
        )
        .unwrap();

        let chunk_size = 800;

        for (chunk, i) in program_data.chunks(chunk_size).zip(0..) {
            instructions.push(bpf_loader_upgradeable::write(
                &program_buffer_keypair.pubkey(),
                &buffer_authority_keypair.pubkey(),
                (i * chunk_size) as u32,
                chunk.to_vec(),
            ));
        }

        instructions.push(bpf_loader_upgradeable::set_buffer_authority(
            &program_buffer_keypair.pubkey(),
            &buffer_authority_keypair.pubkey(),
            &governance_cookie.address,
        ));

        self.bench
            .process_transaction(
                &instructions[..],
                Some(&[&program_buffer_keypair, &buffer_authority_keypair]),
            )
            .await
            .unwrap();

        let mut upgrade_instruction = bpf_loader_upgradeable::upgrade(
            &governance_cookie.account.governed_account,
            &program_buffer_keypair.pubkey(),
            &governance_cookie.address,
            &governance_cookie.address,
        );

        self.with_instruction(
            proposal_cookie,
            token_owner_record_cookie,
            0,
            None,
            &mut upgrade_instruction,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_nop_instruction(
        &mut self,
        proposal_cookie: &mut ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        option_index: u16,
        index: Option<u16>,
    ) -> Result<ProposalInstructionCookie, ProgramError> {
        // Create NOP instruction as a placeholder
        // Note: The actual instruction is irrelevant because we do not execute it in tests
        let mut instruction = Instruction {
            program_id: Pubkey::new_unique(),
            accounts: vec![],
            data: vec![],
        };

        self.with_instruction(
            proposal_cookie,
            token_owner_record_cookie,
            option_index,
            index,
            &mut instruction,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_instruction(
        &mut self,
        proposal_cookie: &mut ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        option_index: u16,
        index: Option<u16>,
        instruction: &mut Instruction,
    ) -> Result<ProposalInstructionCookie, ProgramError> {
        let hold_up_time = 15;

        let instruction_data: InstructionData = instruction.clone().into();
        let mut yes_option = &mut proposal_cookie.account.options[0];

        let instruction_index = index.unwrap_or(yes_option.instructions_next_index);

        yes_option.instructions_next_index += 1;

        let insert_instruction_instruction = insert_instruction(
            &self.program_id,
            &proposal_cookie.account.governance,
            &proposal_cookie.address,
            &token_owner_record_cookie.address,
            &token_owner_record_cookie.token_owner.pubkey(),
            &self.bench.payer.pubkey(),
            option_index,
            instruction_index,
            hold_up_time,
            instruction_data.clone(),
        );

        self.bench
            .process_transaction(
                &[insert_instruction_instruction],
                Some(&[&token_owner_record_cookie.token_owner]),
            )
            .await?;

        let proposal_instruction_address = get_proposal_instruction_address(
            &self.program_id,
            &proposal_cookie.address,
            &option_index.to_le_bytes(),
            &instruction_index.to_le_bytes(),
        );

        let proposal_instruction_data = ProposalInstructionV2 {
            account_type: GovernanceAccountType::ProposalInstructionV2,
            option_index,
            instruction_index,
            hold_up_time,
            instruction: instruction_data,
            executed_at: None,
            execution_status: InstructionExecutionStatus::None,
            proposal: proposal_cookie.address,
        };

        instruction.accounts = instruction
            .accounts
            .iter()
            .map(|a| AccountMeta {
                pubkey: a.pubkey,
                is_signer: false, // Remove signer since the Governance account PDA will be signing the instruction for us
                is_writable: a.is_writable,
            })
            .collect();

        let proposal_instruction_cookie = ProposalInstructionCookie {
            address: proposal_instruction_address,
            account: proposal_instruction_data,
            instruction: instruction.clone(),
        };

        Ok(proposal_instruction_cookie)
    }

    #[allow(dead_code)]
    pub async fn remove_instruction(
        &mut self,
        proposal_cookie: &mut ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        proposal_instruction_cookie: &ProposalInstructionCookie,
    ) -> Result<(), ProgramError> {
        let remove_instruction_instruction = remove_instruction(
            &self.program_id,
            &proposal_cookie.address,
            &token_owner_record_cookie.address,
            &token_owner_record_cookie.token_owner.pubkey(),
            &proposal_instruction_cookie.address,
            &self.bench.payer.pubkey(),
        );

        self.bench
            .process_transaction(
                &[remove_instruction_instruction],
                Some(&[&token_owner_record_cookie.token_owner]),
            )
            .await?;

        Ok(())
    }

    #[allow(dead_code)]
    pub async fn execute_instruction(
        &mut self,
        proposal_cookie: &ProposalCookie,
        proposal_instruction_cookie: &ProposalInstructionCookie,
    ) -> Result<(), ProgramError> {
        let execute_instruction_instruction = execute_instruction(
            &self.program_id,
            &proposal_cookie.account.governance,
            &proposal_cookie.address,
            &proposal_instruction_cookie.address,
            &proposal_instruction_cookie.instruction.program_id,
            &proposal_instruction_cookie.instruction.accounts,
        );

        self.bench
            .process_transaction(&[execute_instruction_instruction], None)
            .await
    }

    #[allow(dead_code)]
    pub async fn flag_instruction_error(
        &mut self,
        proposal_cookie: &ProposalCookie,
        token_owner_record_cookie: &TokenOwnerRecordCookie,
        proposal_instruction_cookie: &ProposalInstructionCookie,
    ) -> Result<(), ProgramError> {
        let governance_authority = token_owner_record_cookie.get_governance_authority();

        let flag_instruction_error_ix = flag_instruction_error(
            &self.program_id,
            &proposal_cookie.address,
            &proposal_cookie.account.token_owner_record,
            &governance_authority.pubkey(),
            &proposal_instruction_cookie.address,
        );

        self.bench
            .process_transaction(&[flag_instruction_error_ix], Some(&[&governance_authority]))
            .await
    }

    #[allow(dead_code)]
    pub async fn get_token_owner_record_account(&mut self, address: &Pubkey) -> TokenOwnerRecord {
        self.bench
            .get_borsh_account::<TokenOwnerRecord>(address)
            .await
    }

    #[allow(dead_code)]
    pub async fn get_program_metadata_account(&mut self, address: &Pubkey) -> ProgramMetadata {
        self.bench
            .get_borsh_account::<ProgramMetadata>(address)
            .await
    }

    #[allow(dead_code)]
    pub async fn get_native_treasury_account(&mut self, address: &Pubkey) -> NativeTreasury {
        self.bench
            .get_borsh_account::<NativeTreasury>(address)
            .await
    }

    #[allow(dead_code)]
    pub async fn get_realm_account(&mut self, realm_address: &Pubkey) -> Realm {
        self.bench.get_borsh_account::<Realm>(realm_address).await
    }

    #[allow(dead_code)]
    pub async fn get_realm_config_data(
        &mut self,
        realm_config_address: &Pubkey,
    ) -> RealmConfigAccount {
        self.bench
            .get_borsh_account::<RealmConfigAccount>(realm_config_address)
            .await
    }

    #[allow(dead_code)]
    pub async fn get_governance_account(&mut self, governance_address: &Pubkey) -> Governance {
        self.bench
            .get_borsh_account::<Governance>(governance_address)
            .await
    }

    #[allow(dead_code)]
    pub async fn get_proposal_account(&mut self, proposal_address: &Pubkey) -> ProposalV2 {
        self.bench
            .get_borsh_account::<ProposalV2>(proposal_address)
            .await
    }

    #[allow(dead_code)]
    pub async fn get_vote_record_account(&mut self, vote_record_address: &Pubkey) -> VoteRecordV2 {
        self.bench
            .get_borsh_account::<VoteRecordV2>(vote_record_address)
            .await
    }

    #[allow(dead_code)]
    pub async fn get_proposal_instruction_account(
        &mut self,
        proposal_instruction_address: &Pubkey,
    ) -> ProposalInstructionV2 {
        self.bench
            .get_borsh_account::<ProposalInstructionV2>(proposal_instruction_address)
            .await
    }

    #[allow(dead_code)]
    pub async fn get_signatory_record_account(
        &mut self,
        proposal_address: &Pubkey,
    ) -> SignatoryRecord {
        self.bench
            .get_borsh_account::<SignatoryRecord>(proposal_address)
            .await
    }

    #[allow(dead_code)]
    async fn get_packed_account<T: Pack + IsInitialized>(&mut self, address: &Pubkey) -> T {
        self.bench
            .context
            .banks_client
            .get_packed_account_data::<T>(*address)
            .await
            .unwrap()
    }

    #[allow(dead_code)]
    pub async fn advance_clock_past_timestamp(&mut self, unix_timestamp: UnixTimestamp) {
        let mut clock = self.bench.get_clock().await;
        let mut n = 1;

        while clock.unix_timestamp <= unix_timestamp {
            // Since the exact time is not deterministic keep wrapping by arbitrary 400 slots until we pass the requested timestamp
            self.bench
                .context
                .warp_to_slot(clock.slot + n * 400)
                .unwrap();

            n += 1;
            clock = self.bench.get_clock().await;
        }
    }

    #[allow(dead_code)]
    pub async fn advance_clock_by_min_timespan(&mut self, time_span: u64) {
        let clock = self.bench.get_clock().await;
        self.advance_clock_past_timestamp(clock.unix_timestamp + (time_span as i64))
            .await;
    }

    #[allow(dead_code)]
    pub async fn advance_clock(&mut self) {
        let clock = self.bench.get_clock().await;
        self.bench.context.warp_to_slot(clock.slot + 2).unwrap();
    }

    #[allow(dead_code)]
    pub async fn get_upgradable_loader_account(
        &mut self,
        address: &Pubkey,
    ) -> UpgradeableLoaderState {
        self.bench.get_bincode_account(address).await
    }

    #[allow(dead_code)]
    pub async fn get_token_account(&mut self, address: &Pubkey) -> spl_token::state::Account {
        self.get_packed_account(address).await
    }

    #[allow(dead_code)]
    pub async fn get_mint_account(&mut self, address: &Pubkey) -> spl_token::state::Mint {
        self.get_packed_account(address).await
    }

    /// ----------- VoterWeight Addin -----------------------------
    #[allow(dead_code)]
    pub async fn with_voter_weight_addin_deposit(
        &mut self,
        token_owner_record_cookie: &mut TokenOwnerRecordCookie,
    ) -> Result<VoterWeightRecordCookie, ProgramError> {
        let voter_weight_record_account = Keypair::new();

        // Governance program has no dependency on the voter-weight-addin program and hence we can't use its instruction creator here
        // and the instruction has to be created manually
        let accounts = vec![
            AccountMeta::new_readonly(self.program_id, false),
            AccountMeta::new_readonly(token_owner_record_cookie.account.realm, false),
            AccountMeta::new_readonly(
                token_owner_record_cookie.account.governing_token_mint,
                false,
            ),
            AccountMeta::new_readonly(token_owner_record_cookie.address, false),
            AccountMeta::new(voter_weight_record_account.pubkey(), true),
            AccountMeta::new_readonly(self.bench.payer.pubkey(), true),
            AccountMeta::new_readonly(system_program::id(), false),
        ];

        let deposit_ix = Instruction {
            program_id: self.voter_weight_addin_id.unwrap(),
            accounts,
            data: vec![1, 120, 0, 0, 0, 0, 0, 0, 0], // 1 - Deposit instruction, 100 amount (u64)
        };

        self.bench
            .process_transaction(&[deposit_ix], Some(&[&voter_weight_record_account]))
            .await?;

        let voter_weight_record_cookie = VoterWeightRecordCookie {
            address: voter_weight_record_account.pubkey(),
            account: VoterWeightRecord {
                account_type: VoterWeightAccountType::VoterWeightRecord,
                realm: token_owner_record_cookie.account.realm,
                governing_token_mint: token_owner_record_cookie.account.governing_token_mint,
                governing_token_owner: token_owner_record_cookie.account.governing_token_owner,
                voter_weight: 100,
                voter_weight_expiry: None,
            },
        };

        token_owner_record_cookie.voter_weight_record = Some(voter_weight_record_cookie.clone());

        Ok(voter_weight_record_cookie)
    }
}
