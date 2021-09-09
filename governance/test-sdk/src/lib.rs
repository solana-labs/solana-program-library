use std::borrow::Borrow;

use borsh::BorshDeserialize;
use cookies::TokenAccountCookie;
use solana_program::{
    borsh::try_from_slice_unchecked, clock::Clock, instruction::Instruction,
    program_error::ProgramError, program_pack::Pack, pubkey::Pubkey, rent::Rent,
    system_instruction, sysvar,
};
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    account::Account, process_instruction::ProcessInstructionWithContext, signature::Keypair,
    signer::Signer, transaction::Transaction,
};

use bincode::deserialize;

use tools::clone_keypair;

use crate::tools::map_transaction_error;

pub mod cookies;
pub mod tools;

/// Specification of a program which is loaded into the test bench
#[derive(Clone)]
pub struct TestBenchProgram<'a> {
    pub program_name: &'a str,
    pub program_id: Pubkey,
    pub process_instruction: Option<ProcessInstructionWithContext>,
}

/// Program's test bench which captures test context, rent and payer and common utility functions
pub struct ProgramTestBench {
    pub context: ProgramTestContext,
    pub rent: Rent,
    pub payer: Keypair,
    pub next_id: u8,
}

impl ProgramTestBench {
    pub async fn start_new(programs: &[TestBenchProgram<'_>]) -> Self {
        let mut program_test = ProgramTest::default();

        for program in programs {
            program_test.add_program(
                program.program_name,
                program.program_id,
                program.process_instruction,
            )
        }

        let mut context = program_test.start_with_context().await;
        let rent = context.banks_client.get_rent().await.unwrap();

        let payer = clone_keypair(&context.payer);

        Self {
            context,
            rent,
            payer,
            next_id: 0,
        }
    }

    pub fn get_unique_name(&mut self, prefix: &str) -> String {
        self.next_id += 1;

        format!("{}.{}", prefix, self.next_id)
    }

    pub async fn process_transaction(
        &mut self,
        instructions: &[Instruction],
        signers: Option<&[&Keypair]>,
    ) -> Result<(), ProgramError> {
        let mut transaction = Transaction::new_with_payer(instructions, Some(&self.payer.pubkey()));

        let mut all_signers = vec![&self.payer];

        if let Some(signers) = signers {
            all_signers.extend_from_slice(signers);
        }

        let recent_blockhash = self
            .context
            .banks_client
            .get_recent_blockhash()
            .await
            .unwrap();

        transaction.sign(&all_signers, recent_blockhash);

        self.context
            .banks_client
            .process_transaction(transaction)
            .await
            .map_err(map_transaction_error)?;

        Ok(())
    }

    pub async fn create_mint(&mut self, mint_keypair: &Keypair, mint_authority: &Pubkey) {
        let mint_rent = self.rent.minimum_balance(spl_token::state::Mint::LEN);

        let instructions = [
            system_instruction::create_account(
                &self.context.payer.pubkey(),
                &mint_keypair.pubkey(),
                mint_rent,
                spl_token::state::Mint::LEN as u64,
                &spl_token::id(),
            ),
            spl_token::instruction::initialize_mint(
                &spl_token::id(),
                &mint_keypair.pubkey(),
                mint_authority,
                None,
                0,
            )
            .unwrap(),
        ];

        self.process_transaction(&instructions, Some(&[mint_keypair]))
            .await
            .unwrap();
    }

    #[allow(dead_code)]
    pub async fn create_empty_token_account(
        &mut self,
        token_account_keypair: &Keypair,
        token_mint: &Pubkey,
        owner: &Pubkey,
    ) {
        let create_account_instruction = system_instruction::create_account(
            &self.context.payer.pubkey(),
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
            owner,
        )
        .unwrap();

        self.process_transaction(
            &[create_account_instruction, initialize_account_instruction],
            Some(&[token_account_keypair]),
        )
        .await
        .unwrap();
    }

    #[allow(dead_code)]
    pub async fn with_token_account(
        &mut self,
        token_mint: &Pubkey,
        owner: &Pubkey,
        token_mint_authority: &Keypair,
        amount: u64,
    ) -> TokenAccountCookie {
        let token_account_keypair = Keypair::new();

        self.create_empty_token_account(&token_account_keypair, token_mint, owner)
            .await;

        self.mint_tokens(
            token_mint,
            token_mint_authority,
            &token_account_keypair.pubkey(),
            amount,
        )
        .await;

        TokenAccountCookie {
            address: token_account_keypair.pubkey(),
        }
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
            token_mint,
            token_account,
            &token_mint_authority.pubkey(),
            &[],
            amount,
        )
        .unwrap();

        self.process_transaction(&[mint_instruction], Some(&[token_mint_authority]))
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
            &self.context.payer.pubkey(),
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
            Some(&[token_account_keypair, token_mint_authority, owner]),
        )
        .await
        .unwrap();
    }

    #[allow(dead_code)]
    pub async fn get_clock(&mut self) -> Clock {
        self.get_bincode_account::<Clock>(&sysvar::clock::id())
            .await
    }

    #[allow(dead_code)]
    pub async fn get_bincode_account<T: serde::de::DeserializeOwned>(
        &mut self,
        address: &Pubkey,
    ) -> T {
        self.context
            .banks_client
            .get_account(*address)
            .await
            .unwrap()
            .map(|a| deserialize::<T>(a.data.borrow()).unwrap())
            .unwrap_or_else(|| panic!("GET-TEST-ACCOUNT-ERROR: Account {}", address))
    }

    /// TODO: Add to SDK
    pub async fn get_borsh_account<T: BorshDeserialize>(&mut self, address: &Pubkey) -> T {
        self.get_account(address)
            .await
            .map(|a| try_from_slice_unchecked(&a.data).unwrap())
            .unwrap_or_else(|| panic!("GET-TEST-ACCOUNT-ERROR: Account {} not found", address))
    }

    #[allow(dead_code)]
    pub async fn get_account(&mut self, address: &Pubkey) -> Option<Account> {
        self.context
            .banks_client
            .get_account(*address)
            .await
            .unwrap()
    }
}
