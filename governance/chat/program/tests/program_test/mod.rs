use std::str::FromStr;

use solana_program::{program_error::ProgramError, pubkey::Pubkey};
use solana_program_test::{processor, ProgramTest};

use solana_sdk::{signature::Keypair, signer::Signer};
use spl_governance::{
    instruction::{
        create_account_governance, create_proposal, create_realm, create_token_owner_record,
        deposit_governing_tokens,
    },
    state::{
        enums::{MintMaxVoteWeightSource, VoteThresholdPercentage},
        governance::{get_account_governance_address, GovernanceConfig},
        proposal::{get_proposal_address, VoteType},
        realm::get_realm_address,
        token_owner_record::get_token_owner_record_address,
    },
};
use spl_governance_chat::{
    instruction::post_message,
    processor::process_instruction,
    state::{ChatMessage, GovernanceChatAccountType, MessageBody},
};
use spl_governance_test_sdk::{addins::ensure_voter_weight_addin_is_built, ProgramTestBench};
use spl_governance_voter_weight_addin::instruction::deposit_voter_weight;

use crate::program_test::cookies::{ChatMessageCookie, ProposalCookie};

use self::cookies::TokenOwnerRecordCookie;

pub mod cookies;

pub struct GovernanceChatProgramTest {
    pub bench: ProgramTestBench,
    pub program_id: Pubkey,
    pub governance_program_id: Pubkey,
    pub voter_weight_addin_id: Option<Pubkey>,
}

impl GovernanceChatProgramTest {
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

        let program_id = Pubkey::from_str("GovernanceChat11111111111111111111111111111").unwrap();
        program_test.add_program(
            "spl_governance_chat",
            program_id,
            processor!(process_instruction),
        );

        let governance_program_id =
            Pubkey::from_str("Governance111111111111111111111111111111111").unwrap();
        program_test.add_program(
            "spl_governance",
            governance_program_id,
            processor!(spl_governance::processor::process_instruction),
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
            program_id,
            governance_program_id,
            voter_weight_addin_id,
        }
    }

    #[allow(dead_code)]
    pub async fn with_proposal(&mut self) -> ProposalCookie {
        // Create Realm
        let name = self.bench.get_unique_name("realm");

        let realm_address = get_realm_address(&self.governance_program_id, &name);

        let governing_token_mint_keypair = Keypair::new();
        let governing_token_mint_authority = Keypair::new();

        self.bench
            .create_mint(
                &governing_token_mint_keypair,
                &governing_token_mint_authority.pubkey(),
            )
            .await;

        let realm_authority = Keypair::new();

        let create_realm_ix = create_realm(
            &self.governance_program_id,
            &realm_authority.pubkey(),
            &governing_token_mint_keypair.pubkey(),
            &self.bench.payer.pubkey(),
            None,
            self.voter_weight_addin_id,
            name.clone(),
            1,
            MintMaxVoteWeightSource::FULL_SUPPLY_FRACTION,
        );

        self.bench
            .process_transaction(&[create_realm_ix], None)
            .await
            .unwrap();

        // Create TokenOwnerRecord
        let token_owner = Keypair::new();
        let token_source = Keypair::new();
        let amount = 100;

        if self.voter_weight_addin_id.is_none() {
            let transfer_authority = Keypair::new();

            self.bench
                .create_token_account_with_transfer_authority(
                    &token_source,
                    &governing_token_mint_keypair.pubkey(),
                    &governing_token_mint_authority,
                    amount,
                    &token_owner,
                    &transfer_authority.pubkey(),
                )
                .await;

            let deposit_governing_tokens_ix = deposit_governing_tokens(
                &self.governance_program_id,
                &realm_address,
                &token_source.pubkey(),
                &token_owner.pubkey(),
                &token_owner.pubkey(),
                &self.bench.payer.pubkey(),
                amount,
                &governing_token_mint_keypair.pubkey(),
            );

            self.bench
                .process_transaction(&[deposit_governing_tokens_ix], Some(&[&token_owner]))
                .await
                .unwrap();
        } else {
            let deposit_governing_tokens_ix = create_token_owner_record(
                &self.governance_program_id,
                &realm_address,
                &token_owner.pubkey(),
                &governing_token_mint_keypair.pubkey(),
                &self.bench.payer.pubkey(),
            );

            self.bench
                .process_transaction(&[deposit_governing_tokens_ix], None)
                .await
                .unwrap();
        }

        // Create Governance
        let governed_account_address = Pubkey::new_unique();

        let governance_config = GovernanceConfig {
            min_community_tokens_to_create_proposal: 5,
            min_council_tokens_to_create_proposal: 2,
            min_instruction_hold_up_time: 10,
            max_voting_time: 10,
            vote_threshold_percentage: VoteThresholdPercentage::YesVote(60),
            vote_weight_source: spl_governance::state::enums::VoteWeightSource::Deposit,
            proposal_cool_off_time: 0,
        };

        let token_owner_record_address = get_token_owner_record_address(
            &self.governance_program_id,
            &realm_address,
            &governing_token_mint_keypair.pubkey(),
            &token_owner.pubkey(),
        );

        let voter_weight_record = if self.voter_weight_addin_id.is_some() {
            let voter_weight_record = Keypair::new();
            let deposit_voter_weight_ix = deposit_voter_weight(
                &self.voter_weight_addin_id.unwrap(),
                &self.governance_program_id,
                &realm_address,
                &governing_token_mint_keypair.pubkey(),
                &token_owner_record_address,
                &voter_weight_record.pubkey(),
                &self.bench.payer.pubkey(),
                amount,
            );

            self.bench
                .process_transaction(&[deposit_voter_weight_ix], Some(&[&voter_weight_record]))
                .await
                .unwrap();

            Some(voter_weight_record.pubkey())
        } else {
            None
        };

        let create_account_governance_ix = create_account_governance(
            &self.governance_program_id,
            &realm_address,
            &governed_account_address,
            &token_owner_record_address,
            &self.bench.payer.pubkey(),
            &token_owner.pubkey(),
            voter_weight_record,
            governance_config,
        );

        self.bench
            .process_transaction(&[create_account_governance_ix], Some(&[&token_owner]))
            .await
            .unwrap();

        // Create Proposal

        let governance_address = get_account_governance_address(
            &self.governance_program_id,
            &realm_address,
            &governed_account_address,
        );

        let proposal_name = "Proposal #1".to_string();
        let description_link = "Proposal Description".to_string();
        let options = vec!["Yes".to_string()];
        let proposal_index: u32 = 0;
        let use_deny_option = true;

        let create_proposal_ix = create_proposal(
            &self.governance_program_id,
            &governance_address,
            &token_owner_record_address,
            &token_owner.pubkey(),
            &self.bench.payer.pubkey(),
            voter_weight_record,
            &realm_address,
            proposal_name,
            description_link.clone(),
            &governing_token_mint_keypair.pubkey(),
            VoteType::SingleChoice,
            options,
            use_deny_option,
            proposal_index,
        );

        self.bench
            .process_transaction(&[create_proposal_ix], Some(&[&token_owner]))
            .await
            .unwrap();

        let proposal_address = get_proposal_address(
            &self.governance_program_id,
            &governance_address,
            &governing_token_mint_keypair.pubkey(),
            &proposal_index.to_le_bytes(),
        );

        ProposalCookie {
            address: proposal_address,
            realm_address,
            governance_address,
            token_owner_record_address,
            token_owner,
            governing_token_mint: governing_token_mint_keypair.pubkey(),
            governing_token_mint_authority: governing_token_mint_authority,
            voter_weight_record,
        }
    }

    #[allow(dead_code)]
    pub async fn with_token_owner_deposit(
        &mut self,
        proposal_cookie: &ProposalCookie,
        deposit_amount: u64,
    ) -> TokenOwnerRecordCookie {
        let token_owner = Keypair::new();
        let token_source = Keypair::new();

        let transfer_authority = Keypair::new();

        self.bench
            .create_token_account_with_transfer_authority(
                &token_source,
                &proposal_cookie.governing_token_mint,
                &proposal_cookie.governing_token_mint_authority,
                deposit_amount,
                &token_owner,
                &transfer_authority.pubkey(),
            )
            .await;

        let deposit_governing_tokens_ix = deposit_governing_tokens(
            &self.governance_program_id,
            &proposal_cookie.realm_address,
            &token_source.pubkey(),
            &token_owner.pubkey(),
            &token_owner.pubkey(),
            &self.bench.payer.pubkey(),
            deposit_amount,
            &proposal_cookie.governing_token_mint,
        );

        self.bench
            .process_transaction(&[deposit_governing_tokens_ix], Some(&[&token_owner]))
            .await
            .unwrap();

        let token_owner_record_address = get_token_owner_record_address(
            &self.governance_program_id,
            &proposal_cookie.realm_address,
            &proposal_cookie.governing_token_mint,
            &token_owner.pubkey(),
        );
        TokenOwnerRecordCookie {
            address: token_owner_record_address,
            token_owner,
        }
    }

    #[allow(dead_code)]
    pub async fn with_chat_message(
        &mut self,
        proposal_cookie: &ProposalCookie,
        reply_to: Option<Pubkey>,
    ) -> Result<ChatMessageCookie, ProgramError> {
        let message_account = Keypair::new();
        let message_body = MessageBody::Text("My comment".to_string());

        let post_message_ix = post_message(
            &self.program_id,
            &self.governance_program_id,
            &proposal_cookie.realm_address,
            &proposal_cookie.governance_address,
            &proposal_cookie.address,
            &proposal_cookie.token_owner_record_address,
            &proposal_cookie.token_owner.pubkey(),
            reply_to,
            &message_account.pubkey(),
            &self.bench.payer.pubkey(),
            proposal_cookie.voter_weight_record,
            message_body.clone(),
        );

        let clock = self.bench.get_clock().await;

        let message = ChatMessage {
            account_type: GovernanceChatAccountType::ChatMessage,
            proposal: proposal_cookie.address,
            author: proposal_cookie.token_owner.pubkey(),
            posted_at: clock.unix_timestamp,
            reply_to,
            body: message_body,
        };

        self.bench
            .process_transaction(
                &[post_message_ix],
                Some(&[&proposal_cookie.token_owner, &message_account]),
            )
            .await?;

        Ok(ChatMessageCookie {
            address: message_account.pubkey(),
            account: message,
        })
    }

    #[allow(dead_code)]
    pub async fn get_message_account(&mut self, message_address: &Pubkey) -> ChatMessage {
        self.bench
            .get_borsh_account::<ChatMessage>(message_address)
            .await
    }
}
