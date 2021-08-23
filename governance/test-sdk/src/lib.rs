use solana_program::{
    instruction::Instruction, program_error::ProgramError, pubkey::Pubkey, rent::Rent,
};
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{
    process_instruction::ProcessInstructionWithContext, signature::Keypair, signer::Signer,
    transaction::Transaction,
};
use tools::clone_keypair;

use crate::tools::map_transaction_error;

pub mod tools;

/// Program's test bench which captures test context, rent and payer and common utility functions
pub struct ProgramTestBench {
    pub context: ProgramTestContext,
    pub rent: Rent,
    pub payer: Keypair,
}

/// Details of a program which is loaded into the test bench
#[derive(Clone)]
pub struct TestBenchProgram<'a> {
    pub program_name: &'a str,
    pub program_id: Pubkey,
    pub process_instruction: Option<ProcessInstructionWithContext>,
}

impl ProgramTestBench {
    pub async fn start_new<'a>(programs: &[TestBenchProgram<'a>]) -> Self {
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
}
