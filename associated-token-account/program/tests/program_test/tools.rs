use solana_program::{
    instruction::{Instruction, InstructionError},
    program_error::ProgramError,
    program_pack::{IsInitialized, Pack},
    pubkey::Pubkey,
    rent::Rent,
    system_instruction,
};
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    process_instruction::ProcessInstructionWithContext,
    signature::Keypair,
    signer::Signer,
    transaction::{Transaction, TransactionError},
    transport::TransportError,
};

use std::convert::TryFrom;

use crate::program_test::cookies::MintCookie;

use super::cookies::WalletCookie;

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

    pub async fn with_mint(&mut self) -> MintCookie {
        let mint_keypair = Keypair::new();
        let mint_authority = Keypair::new();

        self.create_mint(&mint_keypair, &mint_authority.pubkey())
            .await;

        MintCookie {
            address: mint_keypair.pubkey(),
            mint_authority,
        }
    }

    pub async fn with_wallet(&mut self) -> WalletCookie {
        let account_keypair = Keypair::new();
        let account_rent = self.rent.minimum_balance(0);

        let create_account_ix = system_instruction::create_account(
            &self.context.payer.pubkey(),
            &account_keypair.pubkey(),
            account_rent,
            spl_token::state::Mint::LEN as u64,
            &spl_token::id(),
        );

        self.process_transaction(&[create_account_ix], Some(&[&account_keypair]))
            .await
            .unwrap();

        WalletCookie {
            address: account_keypair.pubkey(),
        }
    }

    #[allow(dead_code)]
    async fn get_packed_account<T: Pack + IsInitialized>(&mut self, address: &Pubkey) -> T {
        self.context
            .banks_client
            .get_packed_account_data::<T>(*address)
            .await
            .unwrap()
    }

    #[allow(dead_code)]
    pub async fn get_token_account(&mut self, address: &Pubkey) -> spl_token::state::Account {
        self.get_packed_account(address).await
    }
}

/// TODO: Add to Solana SDK
/// Instruction errors not mapped in the sdk
pub enum ProgramInstructionError {
    /// Incorrect authority provided
    IncorrectAuthority = 600,

    /// Cross-program invocation with unauthorized signer or writable account
    PrivilegeEscalation,
}

impl From<ProgramInstructionError> for ProgramError {
    fn from(e: ProgramInstructionError) -> Self {
        ProgramError::Custom(e as u32)
    }
}

pub fn map_transaction_error(transport_error: TransportError) -> ProgramError {
    match transport_error {
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            InstructionError::Custom(error_index),
        )) => ProgramError::Custom(error_index),
        TransportError::TransactionError(TransactionError::InstructionError(
            _,
            instruction_error,
        )) => ProgramError::try_from(instruction_error).unwrap_or_else(|ie| match ie {
            InstructionError::IncorrectAuthority => {
                ProgramInstructionError::IncorrectAuthority.into()
            }
            InstructionError::PrivilegeEscalation => {
                ProgramInstructionError::PrivilegeEscalation.into()
            }
            _ => panic!("TEST-INSTRUCTION-ERROR {:?}", ie),
        }),

        _ => panic!("TEST-TRANSPORT-ERROR: {:?}", transport_error),
    }
}

pub fn clone_keypair(source: &Keypair) -> Keypair {
    Keypair::from_bytes(&source.to_bytes()).unwrap()
}
