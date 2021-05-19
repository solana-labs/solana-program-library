use borsh::BorshDeserialize;
use solana_program::{
    borsh::try_from_slice_unchecked,
    instruction::Instruction,
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
};

use solana_program_test::ProgramTest;
use solana_program_test::*;

use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
};
use spl_governance::{
    instruction::{
        create_realm, deposit_governing_tokens, set_vote_authority, withdraw_governing_tokens,
    },
    processor::process_instruction,
    state::{
        enums::{GovernanceAccountType, GoverningTokenType},
        realm::{get_governing_token_holding_address, get_realm_address, Realm},
        voter_record::{get_voter_record_address, VoterRecord},
    },
};

pub mod cookies;
use self::cookies::{RealmCookie, VoterRecordCookie};

pub mod tools;
use self::tools::map_transaction_error;

pub struct GovernanceProgramTest {
    pub banks_client: BanksClient,
    pub payer: Keypair,
    pub rent: Rent,
}

impl GovernanceProgramTest {
    pub async fn start_new() -> Self {
        let program_test = ProgramTest::new(
            "spl_governance",
            spl_governance::id(),
            processor!(process_instruction),
        );

        let (mut banks_client, payer, _) = program_test.start().await;

        let rent = banks_client.get_rent().await.unwrap();

        Self {
            banks_client,
            payer,
            rent,
        }
    }

    pub async fn process_transaction(
        &mut self,
        instructions: &[Instruction],
        signers: Option<&[&Keypair]>,
    ) -> Result<(), ProgramError> {
        let mut transaction =
            Transaction::new_with_payer(&instructions, Some(&self.payer.pubkey()));

        let mut all_signers = vec![&self.payer];

        if let Some(signers) = signers {
            all_signers.extend_from_slice(signers);
        }

        let recent_blockhash = self.banks_client.get_recent_blockhash().await.unwrap();

        transaction.sign(&all_signers, recent_blockhash);

        self.banks_client
            .process_transaction(transaction)
            .await
            .map_err(map_transaction_error)
    }

    pub async fn get_account<T: BorshDeserialize>(&mut self, address: &Pubkey) -> T {
        let raw_account = self
            .banks_client
            .get_account(*address)
            .await
            .unwrap()
            .expect("GET-TEST-ACCOUNT-ERROR: Account not found");

        try_from_slice_unchecked(&raw_account.data).unwrap()
    }

    #[allow(dead_code)]
    pub async fn with_realm(&mut self) -> RealmCookie {
        let name = "Realm".to_string();

        let realm_address = get_realm_address(&name);

        let community_token_mint_keypair = Keypair::new();
        let community_token_mint_authority = Keypair::new();

        let community_token_holding_address = get_governing_token_holding_address(
            &realm_address,
            &community_token_mint_keypair.pubkey(),
        );

        self.create_mint(
            &community_token_mint_keypair,
            &community_token_mint_authority.pubkey(),
        )
        .await;

        let council_token_mint_keypair = Keypair::new();
        let council_token_mint_authority = Keypair::new();

        let council_token_holding_address = get_governing_token_holding_address(
            &realm_address,
            &council_token_mint_keypair.pubkey(),
        );

        self.create_mint(
            &council_token_mint_keypair,
            &council_token_mint_authority.pubkey(),
        )
        .await;

        let create_proposal_instruction = create_realm(
            &community_token_mint_keypair.pubkey(),
            &self.payer.pubkey(),
            Some(council_token_mint_keypair.pubkey()),
            name.clone(),
        );

        self.process_transaction(&[create_proposal_instruction], None)
            .await
            .unwrap();

        let account = Realm {
            account_type: GovernanceAccountType::Realm,
            community_mint: community_token_mint_keypair.pubkey(),
            council_mint: Some(council_token_mint_keypair.pubkey()),
            name,
        };

        RealmCookie {
            address: realm_address,
            account,

            community_mint_authority: community_token_mint_authority,
            community_token_holding_account: community_token_holding_address,

            council_token_holding_account: Some(council_token_holding_address),
            council_mint_authority: Some(council_token_mint_authority),
        }
    }

    #[allow(dead_code)]
    pub async fn with_initial_community_token_deposit(
        &mut self,
        realm_cookie: &RealmCookie,
    ) -> VoterRecordCookie {
        self.with_initial_governing_token_deposit(
            &realm_cookie.address,
            GoverningTokenType::Community,
            &realm_cookie.account.community_mint,
            &realm_cookie.community_mint_authority,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_community_token_deposit(
        &mut self,
        realm_cookie: &RealmCookie,
        voter_record_cookie: &VoterRecordCookie,
        amount: u64,
    ) {
        self.with_governing_token_deposit(
            &realm_cookie.address,
            &realm_cookie.account.community_mint,
            &realm_cookie.community_mint_authority,
            voter_record_cookie,
            amount,
        )
        .await;
    }

    #[allow(dead_code)]
    pub async fn with_council_token_deposit(
        &mut self,
        realm_cookie: &RealmCookie,
        voter_record_cookie: &VoterRecordCookie,
        amount: u64,
    ) {
        self.with_governing_token_deposit(
            &realm_cookie.address,
            &realm_cookie.account.council_mint.unwrap(),
            &realm_cookie.council_mint_authority.as_ref().unwrap(),
            voter_record_cookie,
            amount,
        )
        .await;
    }

    #[allow(dead_code)]
    pub async fn with_initial_council_token_deposit(
        &mut self,
        realm_cookie: &RealmCookie,
    ) -> VoterRecordCookie {
        self.with_initial_governing_token_deposit(
            &realm_cookie.address,
            GoverningTokenType::Council,
            &realm_cookie.account.council_mint.unwrap(),
            &realm_cookie.council_mint_authority.as_ref().unwrap(),
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn with_initial_governing_token_deposit(
        &mut self,
        realm_address: &Pubkey,
        governing_token_type: GoverningTokenType,
        governing_mint: &Pubkey,
        governing_mint_authority: &Keypair,
    ) -> VoterRecordCookie {
        let token_owner = Keypair::new();
        let token_source = Keypair::new();

        let source_amount = 100;
        let transfer_authority = Keypair::new();

        self.create_token_account_with_transfer_authority(
            &token_source,
            governing_mint,
            governing_mint_authority,
            source_amount,
            &token_owner,
            &transfer_authority.pubkey(),
        )
        .await;

        let deposit_governing_tokens_instruction = deposit_governing_tokens(
            realm_address,
            &token_source.pubkey(),
            &token_owner.pubkey(),
            &token_owner.pubkey(),
            &self.payer.pubkey(),
            governing_mint,
        );

        self.process_transaction(
            &[deposit_governing_tokens_instruction],
            Some(&[&token_owner]),
        )
        .await
        .unwrap();

        let voter_record_address =
            get_voter_record_address(realm_address, &governing_mint, &token_owner.pubkey());

        let account = VoterRecord {
            account_type: GovernanceAccountType::VoterRecord,
            realm: *realm_address,
            token_type: governing_token_type,
            token_owner: token_owner.pubkey(),
            token_deposit_amount: source_amount,
            vote_authority: None,
            active_votes_count: 0,
            total_votes_count: 0,
        };

        let vote_authority = Keypair::from_base58_string(&token_owner.to_base58_string());

        VoterRecordCookie {
            address: voter_record_address,
            account,

            token_source_amount: source_amount,
            token_source: token_source.pubkey(),
            token_owner,
            vote_authority,
        }
    }

    #[allow(dead_code)]
    async fn with_governing_token_deposit(
        &mut self,
        realm: &Pubkey,
        governing_token_mint: &Pubkey,
        governing_token_mint_authority: &Keypair,
        voter_record_cookie: &VoterRecordCookie,
        amount: u64,
    ) {
        self.mint_tokens(
            governing_token_mint,
            governing_token_mint_authority,
            &voter_record_cookie.token_source,
            amount,
        )
        .await;

        let deposit_governing_tokens_instruction = deposit_governing_tokens(
            realm,
            &voter_record_cookie.token_source,
            &voter_record_cookie.token_owner.pubkey(),
            &voter_record_cookie.token_owner.pubkey(),
            &self.payer.pubkey(),
            governing_token_mint,
        );

        self.process_transaction(
            &[deposit_governing_tokens_instruction],
            Some(&[&voter_record_cookie.token_owner]),
        )
        .await
        .unwrap();
    }

    #[allow(dead_code)]
    pub async fn with_community_vote_authority(
        &mut self,
        realm_cookie: &RealmCookie,
        voter_record_cookie: &mut VoterRecordCookie,
    ) {
        self.with_governing_token_vote_authority(
            &realm_cookie,
            &realm_cookie.account.community_mint,
            voter_record_cookie,
        )
        .await;
    }

    #[allow(dead_code)]
    pub async fn with_council_vote_authority(
        &mut self,
        realm_cookie: &RealmCookie,
        voter_record_cookie: &mut VoterRecordCookie,
    ) {
        self.with_governing_token_vote_authority(
            &realm_cookie,
            &realm_cookie.account.council_mint.unwrap(),
            voter_record_cookie,
        )
        .await;
    }

    #[allow(dead_code)]
    pub async fn with_governing_token_vote_authority(
        &mut self,
        realm_cookie: &RealmCookie,
        governing_token_mint: &Pubkey,
        voter_record_cookie: &mut VoterRecordCookie,
    ) {
        let new_vote_authority = Keypair::new();

        self.set_vote_authority(
            realm_cookie,
            voter_record_cookie,
            &voter_record_cookie.token_owner,
            governing_token_mint,
            &Some(new_vote_authority.pubkey()),
        )
        .await;

        voter_record_cookie.vote_authority = new_vote_authority;
    }

    #[allow(dead_code)]
    pub async fn set_vote_authority(
        &mut self,
        realm_cookie: &RealmCookie,
        voter_record_cookie: &VoterRecordCookie,
        signing_vote_authority: &Keypair,
        governing_token_mint: &Pubkey,
        new_vote_authority: &Option<Pubkey>,
    ) {
        let set_vote_authority_instruction = set_vote_authority(
            &signing_vote_authority.pubkey(),
            &realm_cookie.address,
            governing_token_mint,
            &voter_record_cookie.token_owner.pubkey(),
            new_vote_authority,
        );

        self.process_transaction(
            &[set_vote_authority_instruction],
            Some(&[&signing_vote_authority]),
        )
        .await
        .unwrap();
    }

    #[allow(dead_code)]
    pub async fn withdraw_community_tokens(
        &mut self,
        realm_cookie: &RealmCookie,
        voter_record_cookie: &VoterRecordCookie,
    ) -> Result<(), ProgramError> {
        self.withdraw_governing_tokens(
            realm_cookie,
            voter_record_cookie,
            &realm_cookie.account.community_mint,
            &voter_record_cookie.token_owner,
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn withdraw_council_tokens(
        &mut self,
        realm_cookie: &RealmCookie,
        voter_record_cookie: &VoterRecordCookie,
    ) -> Result<(), ProgramError> {
        self.withdraw_governing_tokens(
            realm_cookie,
            voter_record_cookie,
            &realm_cookie.account.council_mint.unwrap(),
            &voter_record_cookie.token_owner,
        )
        .await
    }

    #[allow(dead_code)]
    async fn withdraw_governing_tokens(
        &mut self,
        realm_cookie: &RealmCookie,
        voter_record_cookie: &VoterRecordCookie,
        governing_token_mint: &Pubkey,

        governing_token_owner: &Keypair,
    ) -> Result<(), ProgramError> {
        let deposit_governing_tokens_instruction = withdraw_governing_tokens(
            &realm_cookie.address,
            &voter_record_cookie.token_source,
            &governing_token_owner.pubkey(),
            governing_token_mint,
        );

        self.process_transaction(
            &[deposit_governing_tokens_instruction],
            Some(&[&governing_token_owner]),
        )
        .await
    }

    #[allow(dead_code)]
    pub async fn get_voter_record_account(&mut self, address: &Pubkey) -> VoterRecord {
        self.get_account::<VoterRecord>(address).await
    }

    #[allow(dead_code)]
    pub async fn get_realm_account(&mut self, root_governance_address: &Pubkey) -> Realm {
        self.get_account::<Realm>(root_governance_address).await
    }

    #[allow(dead_code)]
    async fn get_packed_account<T: Pack + IsInitialized>(&mut self, address: &Pubkey) -> T {
        let raw_account = self
            .banks_client
            .get_account(*address)
            .await
            .unwrap()
            .unwrap();

        T::unpack(&raw_account.data).unwrap()
    }

    #[allow(dead_code)]
    pub async fn get_token_account(&mut self, address: &Pubkey) -> spl_token::state::Account {
        self.get_packed_account(address).await
    }

    pub async fn create_mint(&mut self, mint_keypair: &Keypair, mint_authority: &Pubkey) {
        let mint_rent = self.rent.minimum_balance(spl_token::state::Mint::LEN);

        let instructions = [
            system_instruction::create_account(
                &self.payer.pubkey(),
                &mint_keypair.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint_keypair.pubkey(),
                &mint_authority,
                None,
                0,
            )
            .unwrap(),
        ];

        self.process_transaction(&instructions, Some(&[&mint_keypair]))
            .await
            .unwrap();
    }

    #[allow(dead_code)]
    pub async fn create_token_account(
        &mut self,
        token_account_keypair: &Keypair,
        token_mint: &Pubkey,
        token_mint_authority: &Keypair,
        amount: u64,
        owner: &Pubkey,
    ) {
        let create_account_instruction = system_instruction::create_account(
            &self.payer.pubkey(),
            &token_account_keypair.pubkey(),
            self.rent
                .minimum_balance(spl_token::state::Account::get_packed_len()),
            spl_token::state::Account::get_packed_len() as u64,
            &spl_token::id(),
        );

        let initialize_account_instruction = spl_token::instruction::initialize_account(
            &spl_token::id(),
            &token_account_keypair.pubkey(),
            token_mint,
            &owner,
        )
        .unwrap();

        let mint_instruction = spl_token::instruction::mint_to(
            &spl_token::id(),
            token_mint,
            &token_account_keypair.pubkey(),
            &token_mint_authority.pubkey(),
            &[],
            amount,
        )
        .unwrap();

        self.process_transaction(
            &[
                create_account_instruction,
                initialize_account_instruction,
                mint_instruction,
            ],
            Some(&[&token_account_keypair, &token_mint_authority]),
        )
        .await
        .unwrap();
    }

    #[allow(dead_code)]
    pub async fn create_token_account_with_transfer_authority(
        &mut self,
        token_account_keypair: &Keypair,
        token_mint: &Pubkey,
        token_mint_authority: &Keypair,
        amount: u64,
        owner: &Keypair,
        transfer_authority: &Pubkey,
    ) {
        let create_account_instruction = system_instruction::create_account(
            &self.payer.pubkey(),
            &token_account_keypair.pubkey(),
            self.rent
                .minimum_balance(spl_token::state::Account::get_packed_len()),
            spl_token::state::Account::get_packed_len() as u64,
            &spl_token::id(),
        );

        let initialize_account_instruction = spl_token::instruction::initialize_account(
            &spl_token::id(),
            &token_account_keypair.pubkey(),
            token_mint,
            &owner.pubkey(),
        )
        .unwrap();

        let mint_instruction = spl_token::instruction::mint_to(
            &spl_token::id(),
            token_mint,
            &token_account_keypair.pubkey(),
            &token_mint_authority.pubkey(),
            &[],
            amount,
        )
        .unwrap();

        let approve_instruction = spl_token::instruction::approve(
            &spl_token::id(),
            &token_account_keypair.pubkey(),
            transfer_authority,
            &owner.pubkey(),
            &[],
            amount,
        )
        .unwrap();

        self.process_transaction(
            &[
                create_account_instruction,
                initialize_account_instruction,
                mint_instruction,
                approve_instruction,
            ],
            Some(&[&token_account_keypair, &token_mint_authority, &owner]),
        )
        .await
        .unwrap();
    }

    pub async fn mint_tokens(
        &mut self,
        token_mint: &Pubkey,
        token_mint_authority: &Keypair,
        token_account: &Pubkey,
        amount: u64,
    ) {
        let mint_instruction = spl_token::instruction::mint_to(
            &spl_token::id(),
            &token_mint,
            &token_account,
            &token_mint_authority.pubkey(),
            &[],
            amount,
        )
        .unwrap();

        self.process_transaction(&[mint_instruction], Some(&[&token_mint_authority]))
            .await
            .unwrap();
    }
}
