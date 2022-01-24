use std::borrow::Borrow;

use borsh::BorshDeserialize;
use cookies::{TokenAccountCookie, WalletCookie};
use solana_program::{
    borsh::try_from_slice_unchecked, clock::Clock, instruction::Instruction,
    program_error::ProgramError, program_pack::Pack, pubkey::Pubkey, rent::Rent,
    system_instruction, system_program, sysvar,
};
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{account::Account, signature::Keypair, signer::Signer, transaction::Transaction};

use bincode::deserialize;

use spl_token::instruction::{set_authority, AuthorityType};
use tools::clone_keypair;

use crate::tools::map_transaction_error;

pub mod addins;
pub mod cookies;
pub mod tools;

/// Program's test bench which captures test context, rent and payer and common utility functions
pub struct ProgramTestBench {
    pub context: ProgramTestContext,
    pub rent: Rent,
    pub payer: Keypair,
    pub next_id: u8,
}

impl ProgramTestBench {
    /// Create new bench given a ProgramTest instance populated with all of the
    /// desired programs.
    pub async fn start_new(program_test: ProgramTest) -> Self {
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
            .get_latest_blockhash()
            .await
            .unwrap();

        transaction.sign(&all_signers, recent_blockhash);

        #[allow(clippy::useless_conversion)] // Remove during upgrade to 1.10
        self.context
            .banks_client
            .process_transaction(transaction)
            .await
            .map_err(|e| map_transaction_error(e.into()))?;

        Ok(())
    }

    pub async fn with_wallet(&mut self) -> WalletCookie {
        let account_rent = self.rent.minimum_balance(0);
        let account_keypair = Keypair::new();

        let create_account_ix = system_instruction::create_account(
            &self.context.payer.pubkey(),
            &account_keypair.pubkey(),
            account_rent,
            0,
            &system_program::id(),
        );

        self.process_transaction(&[create_account_ix], Some(&[&account_keypair]))
            .await
            .unwrap();

        let account = Account {
            lamports: account_rent,
            data: vec![],
            owner: system_program::id(),
            executable: false,
            rent_epoch: 0,
        };

        WalletCookie {
            address: account_keypair.pubkey(),
            account,
        }
    }

    pub async fn create_mint(
        &mut self,
        mint_keypair: &Keypair,
        mint_authority: &Pubkey,
        freeze_authority: Option<&Pubkey>,
    ) {
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
                freeze_authority,
                0,
            )
            .unwrap(),
        ];

        self.process_transaction(&instructions, Some(&[mint_keypair]))
            .await
            .unwrap();
    }

    /// Sets spl-token program account (Mint or TokenAccount) authority
    pub async fn set_spl_token_account_authority(
        &mut self,
        account: &Pubkey,
        account_authority: &Keypair,
        new_authority: Option<&Pubkey>,
        authority_type: AuthorityType,
    ) {
        let set_authority_ix = set_authority(
            &spl_token::id(),
            account,
            new_authority,
            authority_type,
            &account_authority.pubkey(),
            &[],
        )
        .unwrap();

        self.process_transaction(&[set_authority_ix], Some(&[account_authority]))
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

    pub async fn transfer_sol(&mut self, to_account: &Pubkey, lamports: u64) {
        let transfer_ix = system_instruction::transfer(&self.payer.pubkey(), to_account, lamports);

        self.process_transaction(&[transfer_ix], None)
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
