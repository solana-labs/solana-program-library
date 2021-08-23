use std::str::FromStr;

use solana_program::pubkey::Pubkey;
use solana_program_test::processor;

use solana_sdk::{signature::Keypair, signer::Signer};
use spl_governance::{
    instruction::create_realm,
    state::{
        enums::MintMaxVoteWeightSource,
        realm::{get_governing_token_holding_address, get_realm_address},
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

        let realm_authority = Keypair::new();

        let create_realm_instruction = create_realm(
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
            .process_transaction(&[create_realm_instruction], None)
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
