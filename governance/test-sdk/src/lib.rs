use solana_program::{pubkey::Pubkey, rent::Rent};
use solana_program_test::{ProgramTest, ProgramTestContext};
use solana_sdk::{process_instruction::ProcessInstructionWithContext, signature::Keypair};
use tools::clone_keypair;

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
    pub async fn start_with_programs<'a>(programs: &[TestBenchProgram<'a>]) -> Self {
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
}
