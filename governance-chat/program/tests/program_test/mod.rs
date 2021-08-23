use std::str::FromStr;

use solana_program::pubkey::Pubkey;
use solana_program_test::processor;

use solana_sdk::{signature::Keypair, signer::Signer};
use spl_governance::{
    instruction::{create_account_governance, create_realm, deposit_governing_tokens},
    state::{
        enums::{MintMaxVoteWeightSource, VoteThresholdPercentage},
        governance::GovernanceConfig,
        realm::get_realm_address,
        token_owner_record::get_token_owner_record_address,
    },
};
use spl_governance_chat::{
    instruction::post_message, processor::process_instruction, state::Message,
};
use spl_governance_test_sdk::{ProgramTestBench, TestBenchProgram};

use self::cookies::{MessageCookie, ProposalCookie};

pub mod cookies;

pub struct GovernanceChatProgramTest {
    pub bench: ProgramTestBench,
    pub program_id: Pubkey,
    pub governance_program_id: Pubkey,
}

impl GovernanceChatProgramTest {
    pub async fn start_new() -> Self {
        let program_id = Pubkey::from_str("GovernanceChat11111111111111111111111111111").unwrap();

        let chat_program = TestBenchProgram {
            program_name: "spl_governance_chat",
            program_id: program_id,
            process_instruction: processor!(process_instruction),
        };

        let governance_program_id =
            Pubkey::from_str("Governance111111111111111111111111111111111").unwrap();
        let governance_program = TestBenchProgram {
            program_name: "spl_governance",
            program_id: governance_program_id,
            process_instruction: processor!(spl_governance::processor::process_instruction),
        };

        let bench = ProgramTestBench::start_new(&[chat_program, governance_program]).await;

        Self {
            bench,
            program_id,
            governance_program_id,
        }
    }

    #[allow(dead_code)]
    pub async fn with_proposal(&mut self) -> ProposalCookie {
        // Create Realm
        let name = self.bench.get_unique_name("realm");

        let realm_address = get_realm_address(&self.governance_program_id, &name);

        let community_token_mint_keypair = Keypair::new();
        let community_token_mint_authority = Keypair::new();

        self.bench
            .create_mint(
                &community_token_mint_keypair,
                &community_token_mint_authority.pubkey(),
            )
            .await;

        let realm_authority = Keypair::new();

        let create_realm_ix = create_realm(
            &self.governance_program_id,
            &realm_authority.pubkey(),
            &community_token_mint_keypair.pubkey(),
            &self.bench.payer.pubkey(),
            None,
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

        let transfer_authority = Keypair::new();

        self.bench
            .create_token_account_with_transfer_authority(
                &token_source,
                &community_token_mint_keypair.pubkey(),
                &community_token_mint_authority,
                100,
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
            &community_token_mint_keypair.pubkey(),
        );

        self.bench
            .process_transaction(&[deposit_governing_tokens_ix], Some(&[&token_owner]))
            .await
            .unwrap();

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
            &community_token_mint_keypair.pubkey(),
            &token_owner.pubkey(),
        );

        let create_account_governance_ix = create_account_governance(
            &self.governance_program_id,
            &realm_address,
            &governed_account_address,
            &token_owner_record_address,
            &self.bench.payer.pubkey(),
            governance_config,
        );

        self.bench
            .process_transaction(&[create_account_governance_ix], None)
            .await
            .unwrap();

        let proposal = Pubkey::new_unique();

        ProposalCookie { address: proposal }
    }

    #[allow(dead_code)]
    pub async fn with_message(&mut self) -> MessageCookie {
        let _proposal = Pubkey::new_unique();

        let post_message_ix = post_message(
            &self.program_id,
            &self.bench.payer.pubkey(),
            &self.bench.payer.pubkey(),
        );

        let message = Message {
            proposal: Pubkey::new_unique(),
            author: Pubkey::new_unique(),
            post_at: 10,
            parent: None,
            body: "post ".to_string(),
        };

        self.bench
            .process_transaction(&[post_message_ix], None)
            .await
            .unwrap();

        MessageCookie {
            address: Pubkey::new_unique(),
            account: message,
        }
    }
}
