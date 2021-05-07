#![cfg(feature = "test-bpf")]

use solana_program::{instruction::InstructionError, pubkey::Pubkey, system_instruction};
use solana_program_template::*;
use solana_program_test::*;
use solana_sdk::{
    signature::{Keypair, Signer},
    transaction::Transaction,
    transport::TransportError,
};

pub fn program_test() -> ProgramTest {
    let mut program = ProgramTest::new(
        "solana_program_template",
        id(),
        processor!(processor::Processor::process_instruction),
    );
    program.add_program("heap_storage", heap_storage::id(), processor!(heap_storage::processor::Processor::process_instruction));
    program
}

pub async fn init_heap(
    program_context: &mut ProgramTestContext,
) -> Result<Keypair, TransportError> {
    let rent = program_context.banks_client.get_rent().await.unwrap();
    let heap_min_rent = rent.minimum_balance(heap_storage::state::Heap::LEN);

    let heap_account = Keypair::new();

    let mut transaction = Transaction::new_with_payer(
        &[
            system_instruction::create_account(
                &program_context.payer.pubkey(),
                &heap_account.pubkey(),
                heap_min_rent,
                heap_storage::state::Heap::LEN as u64,
                &heap_storage::id(),
            ),
            instruction::init(&id(), &heap_account.pubkey()).unwrap(),
        ],
        Some(&program_context.payer.pubkey()),
    );

    transaction.sign(
        &[&program_context.payer, &heap_account],
        program_context.last_blockhash,
    );
    program_context
        .banks_client
        .process_transaction(transaction)
        .await?;
    Ok(heap_account)
}

#[tokio::test]
async fn test_call_example_instruction() {
    let mut program_context = program_test().start_with_context().await;

    let heap_acc = init_heap(&mut program_context).await.unwrap();
}
